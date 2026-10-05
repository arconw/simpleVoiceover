import { currentLanguage, t } from '../i18n'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { BrowserAudioIO } from '../audio/BrowserAudioIO'
import {
  decodeAudioPacket,
  type Snapshot,
  type CommandPayloads,
  type CommandName,
  type CommandResult,
  type OperationProgress,
} from '../protocol'

export class StudioClient {
  private socket: WebSocket | null = null
  private audio = new BrowserAudioIO()
  private inFlight = 0
  private receiving = false
  private ended = false
  private unlisten: UnlistenFn[] = []
  private disposed = false
  private drained: (() => void) | null = null
  private transitions: Promise<unknown> = Promise.resolve()
  onSnapshot: (value: Snapshot) => void = () => {}
  onProgress: (value: OperationProgress) => void = () => {}
  onPosition: (value: number) => void = () => {}
  onEnded: () => void = () => {}
  onError: (reason: Error) => void = () => {}
  onInput: (level: number) => void = () => {}
  levels: Record<string, number> = {}

  constructor() {
    this.audio.onPosition = (position) => this.onPosition(position)
    this.audio.onEnded = () => {
      this.receiving = false
      this.onEnded()
    }
    this.audio.onNeed = (queued) => {
      if (this.receiving && !this.ended && queued + this.inFlight < 3) this.pull()
    }
    this.audio.onInput = (samples) => {
      if (!this.socket || this.socket.bufferedAmount > 2 * 1024 * 1024) {
        this.onError(new Error(t('error.microphoneBackpressure')))
        return
      }
      const packet = new Uint8Array(samples.byteLength + 1)
      packet[0] = 1
      packet.set(new Uint8Array(samples.buffer), 1)
      this.socket.send(packet)
    }
  }
  async connect() {
    if (!isTauri()) throw new Error(t('error.desktopOnly'))
    const unlisten = await Promise.all([
      listen<Snapshot>('studio-snapshot', (event) => this.onSnapshot(event.payload)),
      listen<OperationProgress>('studio-progress', (event) => this.onProgress(event.payload)),
    ])
    if (this.disposed) {
      unlisten.forEach((unsubscribe) => unsubscribe())
      return
    }
    this.unlisten = unlisten
    this.socket = new WebSocket(`ws://${location.host}/ws`)
    this.socket.binaryType = 'arraybuffer'
    this.socket.onmessage = ({ data }) => {
      if (data instanceof ArrayBuffer) {
        this.inFlight = Math.max(0, this.inFlight - 1)
        if (!this.receiving) return
        const packet = decodeAudioPacket(data)
        this.audio.enqueue(packet.samples, packet.position)
        return
      }
      const message = JSON.parse(data)
      if (message.type === 'input-drained') this.drained?.()
      if (message.type === 'meters') {
        this.levels = message.levels
        if (message.finished) {
          this.ended = true
          this.audio.finish()
        }
      }
      if (message.type === 'input') this.onInput(message.level)
      if (message.type === 'error' && this.receiving && !this.ended)
        this.onError(new Error(message.error))
    }
    this.socket.onclose = () => {
      this.receiving = false
      this.audio.clear()
      this.audio.releaseMicrophone()
      this.onError(new Error(t('error.audioClosed')))
    }
    await new Promise<void>((resolve, reject) => {
      this.socket!.onopen = () => resolve()
      this.socket!.onerror = () => reject(new Error(t('error.audioConnect')))
    })
    await this.request('snapshot')
  }
  async request<K extends CommandName>(
    command: K,
    payload: CommandPayloads[K] = {} as CommandPayloads[K],
  ): Promise<CommandResult> {
    try {
      return await invoke<CommandResult>('studio_command', {
        request: { command, ...payload, language: currentLanguage() },
      })
    } catch (error) {
      throw new Error(String(error))
    }
  }
  private pull() {
    this.inFlight++
    this.socket?.send(new Uint8Array([3]))
  }
  private async drain() {
    if (this.socket?.readyState !== WebSocket.OPEN) return
    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.drained = null
        reject(new Error(t('error.audioDrain')))
      }, 5000)
      this.drained = () => {
        clearTimeout(timeout)
        this.drained = null
        resolve()
      }
      this.socket!.send(new Uint8Array([2]))
    })
    this.inFlight = 0
  }
  private start(position: number) {
    this.inFlight = 0
    this.ended = false
    this.receiving = true
    this.audio.clear(position, true)
    for (let i = 0; i < 3; i++) this.pull()
  }
  private transition(action: () => Promise<void>) {
    const result = this.transitions.then(action)
    this.transitions = result.catch(() => {})
    return result
  }
  play(position: number) {
    return this.transition(async () => {
      await this.audio.open()
      this.receiving = false
      await this.drain()
      await this.request('play', { position })
      this.start(position)
    })
  }
  pause() {
    return this.transition(async () => {
      this.receiving = false
      this.audio.clear()
      this.levels = {}
      await this.drain()
      await this.request('pause')
    })
  }
  async prepareMicrophone(deviceId: string) {
    await this.audio.prepareMicrophone(deviceId)
  }
  async startRecording(position: number, monitor: boolean) {
    await this.request('monitor', { enabled: monitor })
    await this.request('record_begin', { position })
    this.start(position)
    this.audio.startMicrophone()
  }
  async stopRecording() {
    await this.audio.stopMicrophone()
    this.receiving = false
    this.audio.clear()
    await this.drain()
    return this.request('record_end')
  }
  setMonitor(enabled: boolean) {
    void this.request('monitor', { enabled }).catch(this.onError)
  }
  getLevel(id: string) {
    return this.levels[id] ?? 0
  }
  close() {
    this.disposed = true
    this.unlisten.forEach((unsubscribe) => unsubscribe())
    if (this.socket) this.socket.onclose = null
    this.socket?.close()
    this.audio.close()
  }
}

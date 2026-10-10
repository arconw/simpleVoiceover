import { currentLanguage, t } from '../i18n'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { BrowserAudioIO } from '../audio/BrowserAudioIO'
import {
  browserAudioDevices,
  defaultInputSignature,
  effectiveAudioDevice,
  emptyAudioDevices,
  nativeBrowserInput,
  type AudioDeviceCatalog,
} from '../audio/devices'
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
  private playbackRevision = 0
  private deviceTransitions: Promise<unknown> = Promise.resolve()
  private devices = emptyAudioDevices
  private inputDevice = ''
  private outputDevice = ''
  private defaultInput = ''
  private deviceTimer: ReturnType<typeof setInterval> | null = null
  private refreshingDevices = false
  private pendingReconnect = false
  private preferenceTransition: Promise<void> = Promise.resolve()
  private deviceChanged = () => {
    void this.refreshAudioDevices().catch(this.onError)
  }
  onSnapshot: (value: Snapshot) => void = () => {}
  onProgress: (value: OperationProgress) => void = () => {}
  onPosition: (value: number) => void = () => {}
  onEnded: () => void = () => {}
  onError: (reason: Error) => void = () => {}
  onInput: (level: number) => void = () => {}
  onAudioDevices: (devices: AudioDeviceCatalog) => void = () => {}
  levels: Record<string, number> = {}

  constructor() {
    this.audio.onDeviceLost = () => {
      void this.refreshAudioDevices(true).catch(this.onError)
    }
    this.audio.onPosition = (position) => {
      if (this.receiving) this.onPosition(position)
    }
    this.audio.onEnded = () => {
      if (!this.receiving) return
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
      if (message.type === 'meters' && this.receiving) {
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
    await this.refreshAudioDevices()
    if (this.disposed) return
    globalThis.navigator?.mediaDevices?.addEventListener('devicechange', this.deviceChanged)
    this.deviceTimer = setInterval(this.deviceChanged, 2000)
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
  private currentPlayback(revision: number) {
    return !this.disposed && revision === this.playbackRevision
  }
  openAudio() {
    return this.audio.open()
  }
  play(position: number) {
    const revision = ++this.playbackRevision
    this.receiving = false
    this.audio.clear(position)
    return this.transition(async () => {
      if (!this.currentPlayback(revision)) return
      await this.audio.open()
      await this.synchronizeAudioDevices()
      if (!this.currentPlayback(revision)) return
      await this.drain()
      if (!this.currentPlayback(revision)) return
      await this.request('play', { position })
      if (!this.currentPlayback(revision)) return
      this.start(position)
    })
  }
  pause() {
    const revision = ++this.playbackRevision
    this.receiving = false
    this.audio.clear()
    this.levels = {}
    return this.transition(async () => {
      if (!this.currentPlayback(revision)) return
      await this.drain()
      if (!this.currentPlayback(revision)) return
      await this.request('pause')
    })
  }
  async prepareMicrophone() {
    await this.audio.prepareMicrophone(await this.microphoneDevice())
    await this.synchronizeAudioDevices()
    await this.refreshAudioDevices()
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
  private async synchronizeAudioDevices() {
    if (this.devices.nativeRouting) await this.request('audio_devices_sync')
  }
  private async microphoneDevice() {
    if (!this.devices.nativeRouting)
      return effectiveAudioDevice(this.inputDevice, this.devices.inputs)
    const browserDevices = (await globalThis.navigator?.mediaDevices?.enumerateDevices()) ?? []
    return nativeBrowserInput(this.inputDevice, this.devices.inputs, browserDevices)
  }
  setAudioDevices(input: string, output: string) {
    if (input === this.inputDevice && output === this.outputDevice) return this.preferenceTransition
    this.inputDevice = input
    this.outputDevice = output
    this.preferenceTransition = this.updateAudioDevices(false).then(() =>
      this.synchronizeAudioDevices(),
    )
    return this.preferenceTransition
  }
  private updateAudioDevices(reconnectInput: boolean) {
    const update = this.deviceTransitions.then(async () => {
      if (this.disposed) return
      await this.audio.setDevices(
        await this.microphoneDevice(),
        this.devices.nativeRouting
          ? ''
          : effectiveAudioDevice(this.outputDevice, this.devices.outputs),
        reconnectInput,
      )
      if (reconnectInput) await this.synchronizeAudioDevices()
    })
    this.deviceTransitions = update.catch(() => {})
    return update
  }
  async refreshAudioDevices(reconnectInput = false) {
    if (this.disposed) return
    if (this.refreshingDevices) {
      this.pendingReconnect ||= reconnectInput
      return
    }
    this.refreshingDevices = true
    try {
      const result = await this.request('audio_devices')
      let catalog = result.audioDevices ?? emptyAudioDevices
      if (!catalog.nativeRouting) {
        const devices = (await globalThis.navigator?.mediaDevices?.enumerateDevices()) ?? []
        const signature = defaultInputSignature(devices)
        reconnectInput ||=
          !effectiveAudioDevice(this.inputDevice, browserAudioDevices(devices).inputs) &&
          !!this.defaultInput &&
          signature !== this.defaultInput
        this.defaultInput = signature
        catalog = browserAudioDevices(devices)
      }
      if (this.disposed) return
      this.devices = catalog
      this.onAudioDevices(catalog)
      await this.updateAudioDevices(reconnectInput)
    } finally {
      this.refreshingDevices = false
      if (this.pendingReconnect) {
        this.pendingReconnect = false
        await this.refreshAudioDevices(true)
      }
    }
  }
  close() {
    this.disposed = true
    this.playbackRevision++
    if (this.deviceTimer) clearInterval(this.deviceTimer)
    globalThis.navigator?.mediaDevices?.removeEventListener('devicechange', this.deviceChanged)
    this.unlisten.forEach((unsubscribe) => unsubscribe())
    if (this.socket) this.socket.onclose = null
    this.socket?.close()
    this.audio.close()
  }
}

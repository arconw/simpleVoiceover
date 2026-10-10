import { afterEach, describe, expect, it, vi } from 'vitest'

const { calls, subscriptions, audio } = vi.hoisted(() => ({
  calls: [] as { command: string; position?: number }[],
  subscriptions: [] as string[],
  audio: {
    flushed: false,
    closed: false,
    native: false,
    reconnect: false,
    deviceLost: () => {},
    position: (_position: number) => {},
    clears: [] as { position?: number; active: boolean }[],
    playWait: null as Promise<void> | null,
    finishes: 0,
  },
}))
vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  invoke: async (_name: string, data: { request: { command: string } }) => {
    calls.push(data.request)
    if (data.request.command === 'play') await audio.playWait
    return data.request.command === 'audio_devices'
      ? { audioDevices: { inputs: [], outputs: [], nativeRouting: audio.native, available: true } }
      : {}
  },
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string) => {
    subscriptions.push(name)
    return () => {}
  },
}))
vi.mock('../src/audio/BrowserAudioIO', () => ({
  BrowserAudioIO: class {
    onPosition = (_position: number) => {}
    onEnded = () => {}
    onNeed = () => {}
    onInput = () => {}
    onDeviceLost = () => {}
    constructor() {
      audio.deviceLost = () => this.onDeviceLost()
      audio.position = (position) => this.onPosition(position)
    }
    async open() {}
    async setDevices(_input: string, _output: string, reconnect: boolean) {
      audio.reconnect = reconnect
    }
    clear(position?: number, active = false) {
      audio.clears.push({ position, active })
    }
    enqueue() {}
    finish() {
      audio.finishes++
    }
    async prepareMicrophone() {}
    startMicrophone() {}
    async stopMicrophone() {
      audio.flushed = true
    }
    releaseMicrophone() {}
    close() {
      audio.closed = true
    }
  },
}))
import { StudioClient } from '../src/services/StudioClient'

class Socket {
  static OPEN = 1
  static instances: Socket[] = []
  readyState = 1
  bufferedAmount = 0
  binaryType = ''
  sent: Uint8Array[] = []
  onopen: (() => void) | null = null
  onerror: (() => void) | null = null
  onclose: (() => void) | null = null
  onmessage: ((event: { data: string }) => void) | null = null
  constructor() {
    Socket.instances.push(this)
    queueMicrotask(() => this.onopen?.())
  }
  send(bytes: Uint8Array) {
    this.sent.push(bytes)
    if (bytes[0] === 2)
      queueMicrotask(() => this.onmessage?.({ data: JSON.stringify({ type: 'input-drained' }) }))
  }
  close() {
    this.readyState = 3
  }
}

afterEach(() => {
  vi.unstubAllGlobals()
  calls.length = 0
  subscriptions.length = 0
  Socket.instances.length = 0
  audio.flushed = false
  audio.closed = false
  audio.native = false
  audio.reconnect = false
  audio.clears.length = 0
  audio.playWait = null
  audio.finishes = 0
  vi.useRealTimers()
})

async function connected() {
  vi.stubGlobal('WebSocket', Socket)
  vi.stubGlobal('location', { host: '127.0.0.1:5174' })
  const client = new StudioClient()
  await client.connect()
  return client
}

describe('Tauri studio transport', () => {
  it('reopens a lost microphone and restores native routing', async () => {
    audio.native = true
    const client = await connected()
    audio.deviceLost()
    await vi.waitFor(() => {
      expect(audio.reconnect).toBe(true)
      expect(calls.at(-1)?.command).toBe('audio_devices_sync')
    })
    client.close()
  })
  it('synchronizes native routing before playback and microphone recording', async () => {
    audio.native = true
    const client = await connected()
    await client.setAudioDevices('microphone', 'speakers')
    await client.play(0)
    await client.prepareMicrophone()
    expect(calls.map((call) => call.command)).toEqual([
      'audio_devices',
      'snapshot',
      'audio_devices_sync',
      'audio_devices_sync',
      'play',
      'audio_devices_sync',
      'audio_devices',
    ])
    client.close()
  })
  it('removes device listeners and polling when closed', async () => {
    vi.useFakeTimers()
    const mediaDevices = {
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      enumerateDevices: vi.fn().mockResolvedValue([]),
    }
    vi.stubGlobal('navigator', { mediaDevices, languages: ['en'] })
    const client = await connected()
    expect(mediaDevices.addEventListener).toHaveBeenCalledWith('devicechange', expect.any(Function))
    client.close()
    const before = calls.length
    await vi.advanceTimersByTimeAsync(4000)
    expect(calls.length).toBe(before)
    expect(mediaDevices.removeEventListener).toHaveBeenCalledWith(
      'devicechange',
      mediaDevices.addEventListener.mock.calls[0][1],
    )
    vi.useRealTimers()
  })
  it('keeps project commands in IPC and only binary audio in the socket', async () => {
    const client = await connected()
    await client.request('track_patch', { trackId: 'test', patch: { mute: true } })
    expect(calls.map((call) => call.command)).toEqual(['audio_devices', 'snapshot', 'track_patch'])
    expect(subscriptions).toEqual(['studio-snapshot', 'studio-progress'])
    expect(Socket.instances[0].sent).toHaveLength(0)
    await client.play(3)
    expect(Socket.instances[0].sent.map((packet) => packet[0])).toEqual([2, 3, 3, 3])
    expect(calls.at(-1)?.command).toBe('play')
    client.close()
  })
  it('cancels queued seeks when paused and waits for microphone flush before record end', async () => {
    const client = await connected()
    await Promise.all([client.play(1), client.play(2), client.pause()])
    expect(calls.map((call) => call.command)).toEqual(['audio_devices', 'snapshot', 'pause'])
    await client.stopRecording()
    expect(audio.flushed).toBe(true)
    expect(Socket.instances[0].sent.at(-1)?.[0]).toBe(2)
    expect(calls.at(-1)?.command).toBe('record_end')
    client.close()
    expect(audio.closed).toBe(true)
  })
  it('starts only the latest queued seek and suppresses previous playback positions', async () => {
    const client = await connected()
    client.onPosition = vi.fn()
    await client.play(0)
    const first = client.play(10)
    audio.position(0.03)
    const second = client.play(20)
    Socket.instances[0].onmessage?.({
      data: JSON.stringify({ type: 'meters', levels: {}, finished: true }),
    })
    expect(client.onPosition).not.toHaveBeenCalled()
    expect(audio.finishes).toBe(0)
    await Promise.all([first, second])
    expect(calls.filter((call) => call.command === 'play').map((call) => call.position)).toEqual([
      0, 20,
    ])
    expect(audio.clears.filter((clear) => clear.active).map((clear) => clear.position)).toEqual([
      0, 20,
    ])
    audio.position(20.03)
    expect(client.onPosition).toHaveBeenCalledWith(20.03)
    client.close()
  })
  it('discards a seek superseded while its backend command is in progress', async () => {
    const client = await connected()
    let complete!: () => void
    audio.playWait = new Promise<void>((resolve) => {
      complete = resolve
    })
    const first = client.play(10)
    await vi.waitFor(() => expect(calls.at(-1)?.position).toBe(10))
    const second = client.play(20)
    const third = client.play(30)
    complete()
    await Promise.all([first, second, third])
    expect(calls.filter((call) => call.command === 'play').map((call) => call.position)).toEqual([
      10, 30,
    ])
    expect(audio.clears.filter((clear) => clear.active).map((clear) => clear.position)).toEqual([
      30,
    ])
    client.close()
  })
})

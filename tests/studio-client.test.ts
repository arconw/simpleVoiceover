import { afterEach, describe, expect, it, vi } from 'vitest'

const { calls, subscriptions, audio } = vi.hoisted(() => ({
  calls: [] as { command: string }[],
  subscriptions: [] as string[],
  audio: { flushed: false, closed: false },
}))
vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  invoke: async (_name: string, data: { request: { command: string } }) => {
    calls.push(data.request)
    return {}
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
    onPosition = () => {}
    onEnded = () => {}
    onNeed = () => {}
    onInput = () => {}
    async open() {}
    clear() {}
    enqueue() {}
    finish() {}
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
})

async function connected() {
  vi.stubGlobal('WebSocket', Socket)
  vi.stubGlobal('location', { host: '127.0.0.1:5174' })
  const client = new StudioClient()
  await client.connect()
  return client
}

describe('Tauri studio transport', () => {
  it('keeps project commands in IPC and only binary audio in the socket', async () => {
    const client = await connected()
    await client.request('track_patch', { trackId: 'test', patch: { mute: true } })
    expect(calls.map((call) => call.command)).toEqual(['snapshot', 'track_patch'])
    expect(subscriptions).toEqual(['studio-snapshot'])
    expect(Socket.instances[0].sent).toHaveLength(0)
    await client.play(3)
    expect(Socket.instances[0].sent.map((packet) => packet[0])).toEqual([2, 3, 3, 3])
    expect(calls.at(-1)?.command).toBe('play')
    client.close()
  })
  it('serializes rapid seeks and waits for microphone flush before record end', async () => {
    const client = await connected()
    await Promise.all([client.play(1), client.play(2), client.pause()])
    expect(calls.map((call) => call.command)).toEqual(['snapshot', 'play', 'play', 'pause'])
    await client.stopRecording()
    expect(audio.flushed).toBe(true)
    expect(Socket.instances[0].sent.at(-1)?.[0]).toBe(2)
    expect(calls.at(-1)?.command).toBe('record_end')
    client.close()
    expect(audio.closed).toBe(true)
  })
})

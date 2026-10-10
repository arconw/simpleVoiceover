import { afterEach, describe, expect, it, vi } from 'vitest'
import { BrowserAudioIO } from '../src/audio/BrowserAudioIO'

class Node {
  static instances: Node[] = []
  constructor() {
    Node.instances.push(this)
  }
  connect = vi.fn()
  disconnect = vi.fn()
  port = { onmessage: null as ((event: { data: unknown }) => void) | null, postMessage: vi.fn() }
}
class Context {
  static instances: Context[] = []
  sampleRate = 48000
  destination = new Node()
  audioWorklet = { addModule: vi.fn().mockResolvedValue(undefined) }
  sinkId = ''
  setSinkId = vi.fn(async (id: string) => {
    this.sinkId = id
  })
  resume = vi.fn().mockResolvedValue(undefined)
  close = vi.fn().mockResolvedValue(undefined)
  sources: Node[] = []
  createMediaStreamSource() {
    const source = new Node()
    this.sources.push(source)
    return source
  }
  constructor() {
    Context.instances.push(this)
  }
}
const stream = () => {
  const track = { stop: vi.fn(), addEventListener: vi.fn() }
  return { getTracks: () => [track], getAudioTracks: () => [track] } as unknown as MediaStream
}

afterEach(() => {
  vi.unstubAllGlobals()
  Context.instances.length = 0
  Node.instances.length = 0
})

function setup(getUserMedia = vi.fn().mockResolvedValue(stream())) {
  vi.stubGlobal('AudioContext', Context)
  vi.stubGlobal('AudioWorkletNode', Node)
  vi.stubGlobal('navigator', { languages: ['en'], mediaDevices: { getUserMedia } })
  return new BrowserAudioIO()
}

describe('active audio devices', () => {
  it('rejects delayed position, refill, and end events from playback before a seek', async () => {
    const audio = setup()
    await audio.open()
    const output = Node.instances[1]
    audio.onPosition = vi.fn()
    audio.onNeed = vi.fn()
    audio.onEnded = vi.fn()
    audio.clear(1, true)
    const first = output.port.postMessage.mock.calls.at(-1)?.[0].generation
    output.port.onmessage?.({ data: { type: 'position', position: 1.03, generation: first } })
    expect(audio.onPosition).toHaveBeenCalledWith(1.03)
    audio.clear(20, true)
    const current = output.port.postMessage.mock.calls.at(-1)?.[0].generation
    output.port.onmessage?.({ data: { type: 'position', position: 1.04, generation: first } })
    output.port.onmessage?.({ data: { type: 'need', queued: 0, generation: first } })
    output.port.onmessage?.({ data: { type: 'ended', generation: first } })
    expect(audio.position).toBe(20)
    expect(audio.onPosition).toHaveBeenCalledOnce()
    expect(audio.onNeed).not.toHaveBeenCalled()
    expect(audio.onEnded).not.toHaveBeenCalled()
    output.port.onmessage?.({ data: { type: 'position', position: 20.03, generation: current } })
    expect(audio.position).toBe(20.03)
    expect(audio.onPosition).toHaveBeenLastCalledWith(20.03)
    audio.close()
  })
  it('changes and resets output without recreating the audio context', async () => {
    const audio = setup()
    await audio.open()
    await audio.setDevices('', 'speaker')
    await audio.setDevices('', '')
    expect(Context.instances).toHaveLength(1)
    expect(Context.instances[0].setSinkId.mock.calls).toEqual([['speaker'], ['']])
    expect(Context.instances[0].close).not.toHaveBeenCalled()
    audio.close()
  })
  it('reconnects the default microphone to the existing input worklet', async () => {
    const first = stream()
    const second = stream()
    const capture = vi.fn().mockResolvedValueOnce(first).mockResolvedValueOnce(second)
    const audio = setup(capture)
    await audio.prepareMicrophone('')
    audio.startMicrophone()
    const context = Context.instances[0]
    const worklet = context.sources[0].connect.mock.calls[0][0]
    await audio.setDevices('', '', true)
    expect(first.getTracks()[0].stop).toHaveBeenCalledOnce()
    expect(second.getTracks()[0].stop).not.toHaveBeenCalled()
    expect(context.sources[1].connect).toHaveBeenCalledWith(worklet)
    expect(context.close).not.toHaveBeenCalled()
    audio.close()
  })
  it('releases a microphone that finishes opening after the client closes', async () => {
    let resolve!: (value: MediaStream) => void
    const capture = vi.fn().mockImplementation(
      () =>
        new Promise<MediaStream>((done) => {
          resolve = done
        }),
    )
    const audio = setup(capture)
    const preparing = audio.prepareMicrophone('')
    await vi.waitFor(() => expect(capture).toHaveBeenCalled())
    audio.close()
    const late = stream()
    resolve(late)
    await preparing
    expect(late.getTracks()[0].stop).toHaveBeenCalledOnce()
    expect(Context.instances[0].sources).toHaveLength(0)
  })
})

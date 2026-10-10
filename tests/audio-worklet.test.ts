import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { describe, expect, it } from 'vitest'

function worklets() {
  const processors: Record<string, new () => Worklet> = {}
  class Worklet {
    messages: { type: string; position?: number; samples?: Float32Array; generation?: number }[] =
      []
    port = {
      onmessage: (_: { data: unknown }) => {},
      postMessage: (message: Worklet['messages'][number]) => this.messages.push(message),
    }
    process(_: Float32Array[][], __: Float32Array[][]) {
      return true
    }
  }
  runInNewContext(readFileSync(new URL('../public/audio-io.js', import.meta.url), 'utf8'), {
    AudioWorkletProcessor: Worklet,
    sampleRate: 48000,
    Float32Array,
    registerProcessor: (name: string, processor: new () => Worklet) => {
      processors[name] = processor
    },
  })
  return processors
}

describe('physical audio I/O', () => {
  it('keeps paused seeking stable and reports end only after the final samples', () => {
    const output = new (worklets()['studio-output'])()
    const render = () => output.process([], [[new Float32Array(128), new Float32Array(128)]])
    output.port.onmessage({ data: { type: 'clear', position: 8, active: false, generation: 1 } })
    for (let i = 0; i < 8; i++) render()
    expect(output.messages).toHaveLength(0)
    output.port.onmessage({ data: { type: 'clear', position: 2, active: true, generation: 2 } })
    output.port.onmessage({
      data: {
        type: 'block',
        samples: new Float32Array(2048).fill(0.25),
        position: 2,
        generation: 2,
      },
    })
    output.port.onmessage({ data: { type: 'finished', generation: 2 } })
    render()
    expect(output.messages.some((message) => message.type === 'ended')).toBe(false)
    for (let i = 0; i < 7; i++) render()
    expect(output.messages.find((message) => message.type === 'position')?.position).toBeCloseTo(
      2 + 1024 / 48000,
    )
    expect(output.messages.at(-1)?.type).toBe('ended')
    expect(output.messages.at(-1)?.generation).toBe(2)
  })
  it('cannot play queued samples or finish events from before a seek', () => {
    const output = new (worklets()['studio-output'])()
    const channels = [new Float32Array(128), new Float32Array(128)]
    output.port.onmessage({ data: { type: 'clear', position: 1, active: true, generation: 1 } })
    output.port.onmessage({
      data: {
        type: 'block',
        samples: new Float32Array(2048).fill(0.5),
        position: 1,
        generation: 1,
      },
    })
    output.port.onmessage({ data: { type: 'clear', position: 30, active: true, generation: 2 } })
    output.port.onmessage({
      data: {
        type: 'block',
        samples: new Float32Array(2048).fill(0.5),
        position: 2,
        generation: 1,
      },
    })
    output.port.onmessage({ data: { type: 'finished', generation: 1 } })
    output.port.onmessage({
      data: {
        type: 'block',
        samples: new Float32Array(2048).fill(0.25),
        position: 30,
        generation: 2,
      },
    })
    for (let index = 0; index < 8; index++) output.process([], [channels])
    expect(channels[0][0]).toBe(0.25)
    expect(output.messages.find((message) => message.type === 'position')?.position).toBeCloseTo(
      30 + 1024 / 48000,
    )
    expect(output.messages.every((message) => message.generation === 2)).toBe(true)
    expect(output.messages.some((message) => message.type === 'ended')).toBe(false)
  })
  it('captures mono without changing samples and flushes the last partial block on stop', () => {
    const input = new (worklets()['studio-input'])()
    input.port.onmessage({ data: 'start' })
    input.process([[new Float32Array(128).fill(0.75), new Float32Array(128).fill(-0.25)]], [])
    expect(input.messages).toHaveLength(0)
    input.port.onmessage({ data: 'stop' })
    expect(input.messages[0].type).toBe('input')
    expect(input.messages[0].samples?.length).toBe(128)
    expect(input.messages[0].samples?.[0]).toBe(0.25)
    expect(input.messages[1].type).toBe('flushed')
  })
})

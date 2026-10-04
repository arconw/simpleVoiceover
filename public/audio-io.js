class StudioOutput extends AudioWorkletProcessor {
  constructor() {
    super()
    this.blocks = []
    this.cursor = 0
    this.position = 0
    this.active = false
    this.finished = false
    this.ticks = 0
    this.port.onmessage = ({ data }) => {
      if (data.type === 'clear') {
        this.blocks = []
        this.cursor = 0
        this.position = data.position
        this.active = data.active
        this.finished = false
      }
      if (data.type === 'block') this.blocks.push(data)
      if (data.type === 'finished') this.finished = true
    }
  }
  process(_, outputs) {
    const output = outputs[0]
    if (!output[0]) return true
    for (let i = 0; i < output[0].length; i++) {
      const block = this.blocks[0]
      if (!block) break
      if (this.cursor === 0) this.position = block.position
      output[0][i] = block.samples[this.cursor * 2]
      if (output[1]) output[1][i] = block.samples[this.cursor * 2 + 1]
      this.cursor++
      this.position += 1 / sampleRate
      if (this.cursor === block.samples.length / 2) {
        this.blocks.shift()
        this.cursor = 0
      }
    }
    if (++this.ticks % 8 === 0) {
      if (this.active || this.blocks.length)
        this.port.postMessage({ type: 'position', position: this.position })
      if (this.active && !this.finished)
        this.port.postMessage({ type: 'need', queued: this.blocks.length })
      if (this.finished && this.blocks.length === 0) {
        this.active = false
        this.port.postMessage({ type: 'ended' })
        this.finished = false
      }
    }
    return true
  }
}
class StudioInput extends AudioWorkletProcessor {
  constructor() {
    super()
    this.samples = new Float32Array(2048)
    this.count = 0
    this.active = false
    this.port.onmessage = ({ data }) => {
      if (data === 'start') this.active = true
      if (data === 'stop') {
        this.active = false
        this.flush()
        this.port.postMessage({ type: 'flushed' })
      }
    }
  }
  flush() {
    if (!this.count) return
    const samples = this.samples.slice(0, this.count)
    this.port.postMessage({ type: 'input', samples }, [samples.buffer])
    this.count = 0
  }
  process(inputs) {
    const input = inputs[0]
    if (!this.active || !input?.length) return true
    for (let i = 0; i < input[0].length; i++) {
      let value = 0
      for (const channel of input) value += channel[i] / input.length
      this.samples[this.count++] = value
      if (this.count === this.samples.length) this.flush()
    }
    return true
  }
}
registerProcessor('studio-output', StudioOutput)
registerProcessor('studio-input', StudioInput)

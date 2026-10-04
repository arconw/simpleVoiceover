export class BrowserAudioIO {
  private context: AudioContext | null = null
  private output: AudioWorkletNode | null = null
  private input: AudioWorkletNode | null = null
  private stream: MediaStream | null = null
  private flushInput: (() => void) | null = null
  onPosition: (position: number) => void = () => {}
  onNeed: (queued: number) => void = () => {}
  onEnded: () => void = () => {}
  onInput: (samples: Float32Array) => void = () => {}
  position = 0

  async open() {
    if (!this.context) {
      this.context = new AudioContext({ sampleRate: 48000, latencyHint: 'interactive' })
      if (this.context.sampleRate !== 48000) throw new Error('Браузер не поддерживает аудио 48 кГц')
      await this.context.audioWorklet.addModule('/audio-io.js')
      this.output = new AudioWorkletNode(this.context, 'studio-output', {
        numberOfInputs: 0,
        outputChannelCount: [2],
      })
      this.output.connect(this.context.destination)
      this.output.port.onmessage = ({ data }) => {
        if (data.type === 'position') {
          this.position = data.position
          this.onPosition(data.position)
        }
        if (data.type === 'need') this.onNeed(data.queued)
        if (data.type === 'ended') this.onEnded()
      }
    }
    await this.context.resume()
  }
  clear(position = this.position, active = false) {
    this.position = position
    this.output?.port.postMessage({ type: 'clear', position, active })
  }
  enqueue(samples: Float32Array, position: number) {
    this.output?.port.postMessage({ type: 'block', samples, position }, [samples.buffer])
  }
  finish() {
    this.output?.port.postMessage({ type: 'finished' })
  }
  async prepareMicrophone(deviceId: string) {
    await this.open()
    this.stream = await navigator.mediaDevices.getUserMedia({
      audio: {
        deviceId: deviceId ? { exact: deviceId } : undefined,
        channelCount: 1,
        echoCancellation: false,
        noiseSuppression: false,
        autoGainControl: false,
      },
    })
    this.input = new AudioWorkletNode(this.context!, 'studio-input')
    this.context!.createMediaStreamSource(this.stream).connect(this.input)
    this.input.connect(this.context!.destination)
    this.input.port.onmessage = ({ data }) => {
      if (data.type === 'flushed') {
        this.flushInput?.()
        this.flushInput = null
      }
      if (data.type === 'input') this.onInput(data.samples)
    }
  }
  startMicrophone() {
    this.input?.port.postMessage('start')
  }
  async stopMicrophone() {
    if (this.input) {
      await new Promise<void>((resolve) => {
        this.flushInput = resolve
        this.input!.port.postMessage('stop')
      })
      this.input.disconnect()
      this.input = null
    }
    this.releaseMicrophone()
  }
  releaseMicrophone() {
    this.stream?.getTracks().forEach((track) => track.stop())
    this.stream = null
  }
  close() {
    this.releaseMicrophone()
    void this.context?.close()
  }
}

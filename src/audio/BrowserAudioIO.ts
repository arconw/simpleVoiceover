import { t } from '../i18n'
export class BrowserAudioIO {
  private context: AudioContext | null = null
  private output: AudioWorkletNode | null = null
  private input: AudioWorkletNode | null = null
  private stream: MediaStream | null = null
  private source: MediaStreamAudioSourceNode | null = null
  private inputDevice = ''
  private outputDevice = ''
  private microphoneTransitions: Promise<unknown> = Promise.resolve()
  private flushInput: (() => void) | null = null
  private disposed = false
  private microphoneGeneration = 0
  private outputGeneration = 0
  onPosition: (position: number) => void = () => {}
  onNeed: (queued: number) => void = () => {}
  onEnded: () => void = () => {}
  onInput: (samples: Float32Array) => void = () => {}
  onDeviceLost: () => void = () => {}
  position = 0

  async open() {
    if (this.disposed) return
    if (!this.context) {
      this.context = new AudioContext({ sampleRate: 48000, latencyHint: 'interactive' })
      if (this.context.sampleRate !== 48000) throw new Error(t('error.browserSampleRate'))
      await this.context.audioWorklet.addModule('/audio-io.js')
      if (this.disposed) return
      this.output = new AudioWorkletNode(this.context, 'studio-output', {
        numberOfInputs: 0,
        outputChannelCount: [2],
      })
      this.output.connect(this.context.destination)
      this.output.port.onmessage = ({ data }) => {
        if (this.disposed || data.generation !== this.outputGeneration) return
        if (data.type === 'position') {
          this.position = data.position
          this.onPosition(data.position)
        }
        if (data.type === 'need') this.onNeed(data.queued)
        if (data.type === 'ended') this.onEnded()
      }
    }
    await this.context.resume()
    await this.applyOutputDevice(this.outputDevice)
  }
  clear(position = this.position, active = false) {
    this.outputGeneration++
    this.position = position
    this.output?.port.postMessage({
      type: 'clear',
      position,
      active,
      generation: this.outputGeneration,
    })
  }
  enqueue(samples: Float32Array, position: number) {
    this.output?.port.postMessage(
      { type: 'block', samples, position, generation: this.outputGeneration },
      [samples.buffer],
    )
  }
  finish() {
    this.output?.port.postMessage({ type: 'finished', generation: this.outputGeneration })
  }
  async prepareMicrophone(deviceId: string) {
    await this.open()
    await this.microphoneTransition(async () => {
      if (this.disposed) return
      await this.replaceMicrophone(deviceId)
      if (this.disposed || !this.source) return
      this.inputDevice = deviceId
      if (this.input) return
      this.input = new AudioWorkletNode(this.context!, 'studio-input')
      this.source!.connect(this.input)
      this.input.connect(this.context!.destination)
      this.input.port.onmessage = ({ data }) => {
        if (data.type === 'flushed') {
          this.flushInput?.()
          this.flushInput = null
        }
        if (data.type === 'input') this.onInput(data.samples)
      }
    })
  }
  private async replaceMicrophone(deviceId: string) {
    const generation = this.microphoneGeneration
    const stream = await navigator.mediaDevices.getUserMedia({
      audio: {
        deviceId: deviceId ? { exact: deviceId } : undefined,
        channelCount: 1,
        echoCancellation: false,
        noiseSuppression: false,
        autoGainControl: false,
      },
    })
    if (this.disposed || generation !== this.microphoneGeneration) {
      stream.getTracks().forEach((track) => track.stop())
      return
    }
    let source: MediaStreamAudioSourceNode
    try {
      source = this.context!.createMediaStreamSource(stream)
    } catch (error) {
      stream.getTracks().forEach((track) => track.stop())
      throw error
    }
    this.source?.disconnect()
    this.stream?.getTracks().forEach((track) => track.stop())
    this.stream = stream
    this.source = source
    if (this.input) source.connect(this.input)
    stream.getAudioTracks().forEach((track) => track.addEventListener('ended', this.onDeviceLost))
  }
  private microphoneTransition(action: () => Promise<void>) {
    const result = this.microphoneTransitions.then(action)
    this.microphoneTransitions = result.catch(() => {})
    return result
  }
  async setDevices(input: string, output: string, reconnectInput = false) {
    await this.applyOutputDevice(output)
    await this.microphoneTransition(async () => {
      if (this.disposed) return
      if (this.stream && (input !== this.inputDevice || reconnectInput)) {
        await this.replaceMicrophone(input)
      }
      this.inputDevice = input
    })
  }
  private async applyOutputDevice(deviceId: string) {
    const context = this.context as
      | (AudioContext & {
          sinkId?: string
          setSinkId?: (deviceId: string) => Promise<void>
        })
      | null
    if (context && context.sinkId !== deviceId) {
      if (context.setSinkId) await context.setSinkId(deviceId)
      else if (deviceId) throw new Error(t('error.outputDeviceUnsupported'))
    }
    this.outputDevice = deviceId
  }
  startMicrophone() {
    this.input?.port.postMessage('start')
  }
  async stopMicrophone() {
    await this.microphoneTransition(async () => {
      if (this.input) {
        await new Promise<void>((resolve) => {
          this.flushInput = resolve
          this.input!.port.postMessage('stop')
        })
        this.input?.disconnect()
        this.input = null
      }
      this.releaseMicrophone()
    })
  }
  releaseMicrophone() {
    this.microphoneGeneration++
    this.source?.disconnect()
    this.source = null
    this.stream?.getTracks().forEach((track) => track.stop())
    this.stream = null
  }
  close() {
    this.disposed = true
    this.flushInput?.()
    this.flushInput = null
    this.releaseMicrophone()
    this.input?.disconnect()
    this.input = null
    void this.context?.close()
  }
}

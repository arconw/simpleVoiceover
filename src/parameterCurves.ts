export type ParameterKind =
  | 'highpass'
  | 'lowMid'
  | 'presence'
  | 'lowpass'
  | 'threshold'
  | 'ratio'
  | 'attack'
  | 'release'
  | 'makeup'
  | 'gateThreshold'
  | 'gateReduction'
  | 'targetLufs'
  | 'truePeak'
  | 'volume'
  | 'pan'

export function parameterCurve(kind: ParameterKind, value: number, min: number, max: number) {
  const amount = Math.max(0, Math.min(1, (value - min) / (max - min)))
  const points = Array.from({ length: 113 }, (_, index) => index / 112)
  const waveform = (x: number) =>
    Math.sin(x * 58) * (0.12 + 0.75 * Math.exp(-(((x - 0.55) / 0.18) ** 2)))
  const response = (x: number) => {
    if (kind === 'highpass') return 1 - 1 / (1 + Math.exp((x - (0.05 + amount * 0.25)) * 22))
    if (kind === 'lowpass') return 1 / (1 + Math.exp((x - (0.6 + amount * 0.38)) * 22))
    if (kind === 'lowMid' || kind === 'presence')
      return 0.55 + (value / 14) * Math.exp(-(((x - (kind === 'lowMid' ? 0.3 : 0.72)) / 0.13) ** 2))
    return 0.55
  }
  const filter = ['highpass', 'lowpass', 'lowMid', 'presence'].includes(kind)
  const transfer = ['threshold', 'ratio'].includes(kind)
  const envelope = kind === 'attack' || kind === 'release'
  const path = (processed: boolean) =>
    points
      .map((x, index) => {
        let y = waveform(x)
        if (filter) y = processed ? response(x) : 0.55
        else if (transfer) {
          const threshold = kind === 'threshold' ? amount : 0.45
          y =
            !processed || x < threshold
              ? x
              : threshold + (x - threshold) / (kind === 'ratio' ? value : 3)
        } else if (envelope) {
          const edge = kind === 'attack' ? 0.25 : 0.65
          y =
            kind === 'attack'
              ? x < edge
                ? 0
                : processed
                  ? 1 - Math.exp(-(x - edge) / (0.02 + amount * 0.23))
                  : 1
              : x < edge
                ? 1
                : processed
                  ? Math.exp(-(x - edge) / (0.03 + amount * 0.25))
                  : 0
        } else if (processed) {
          if (kind === 'gateThreshold' || kind === 'gateReduction') {
            const threshold = kind === 'gateThreshold' ? 0.05 + amount * 0.3 : 0.18
            if (Math.abs(y) < threshold) y *= kind === 'gateReduction' ? 10 ** (-value / 20) : 0.35
          }
          if (kind === 'truePeak') y *= Math.min(1, 10 ** (value / 20) / 0.86)
          if (kind === 'makeup' || kind === 'volume') y *= Math.min(1.3, 10 ** (value / 20))
          if (kind === 'targetLufs') y *= 0.35 + amount * 0.95
          if (kind === 'pan') y *= Math.sqrt(1 - Math.max(0, value))
        }
        const plotted = filter || transfer || envelope ? 58 - y * 46 : 33 - y * 23
        return `${index ? 'L' : 'M'}${(8 + x * 224).toFixed(2)},${plotted.toFixed(2)}`
      })
      .join(' ')
  return { original: path(false), processed: path(true) }
}

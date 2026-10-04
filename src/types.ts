export type TrackKind = 'video' | 'voice' | 'audio'

export interface EffectSettings {
  highpass: number
  lowMid: number
  presence: number
  lowpass: number
  threshold: number
  ratio: number
  attack: number
  release: number
  makeup: number
  gateThreshold: number
  gateReduction: number
}

export interface MediaAsset {
  id: string
  name: string
  kind: 'video' | 'audio'
  url: string
  duration: number
  size: number
  sampleRate: number
  waveform: [number, number][]
  peakFrames: number
}

export interface Clip {
  id: string
  assetId: string
  name: string
  start: number
  offset: number
  duration: number
}

export interface Track {
  id: string
  name: string
  kind: TrackKind
  color: string
  mute: boolean
  solo: boolean
  locked: boolean
  armed: boolean
  fxBypass: boolean
  volume: number
  pan: number
  effects: EffectSettings
  clips: Clip[]
}

export const neutralEffects: EffectSettings = {
  highpass: 20,
  lowMid: 0,
  presence: 0,
  lowpass: 20000,
  threshold: -24,
  ratio: 1,
  attack: 12,
  release: 160,
  makeup: 0,
  gateThreshold: -48,
  gateReduction: 0,
}

export const voiceEffects: EffectSettings = {
  highpass: 70,
  lowMid: -1.5,
  presence: 1,
  lowpass: 15000,
  threshold: -24,
  ratio: 2.3,
  attack: 12,
  release: 160,
  makeup: 2.9,
  gateThreshold: -48,
  gateReduction: 0,
}

export const formatTime = (value: number, precise = false) => {
  const seconds = Math.max(0, value)
  const minutes = Math.floor(seconds / 60)
    .toString()
    .padStart(2, '0')
  const remainder = Math.floor(seconds % 60)
    .toString()
    .padStart(2, '0')
  return `${minutes}:${remainder}${
    precise
      ? `.${Math.floor((seconds % 1) * 1000)
          .toString()
          .padStart(3, '0')}`
      : ''
  }`
}

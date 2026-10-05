import { neutralEffects, voiceEffects, type EffectSettings } from './types'
import type { TranslationKey } from './i18n'

export interface EffectPreset {
  id: string
  name: string
  effects: EffectSettings
  fxBypass: boolean
}

export const builtInPresets: (EffectPreset & { name: TranslationKey })[] = [
  { id: 'natural', name: 'effects.preset', effects: voiceEffects, fxBypass: false },
  { id: 'neutral', name: 'presets.neutral', effects: neutralEffects, fxBypass: true },
  {
    id: 'warm',
    name: 'presets.warm',
    effects: {
      ...voiceEffects,
      highpass: 60,
      lowMid: 2,
      presence: 0.5,
      lowpass: 14000,
      ratio: 2,
      makeup: 2,
    },
    fxBypass: false,
  },
  {
    id: 'clear',
    name: 'presets.clear',
    effects: {
      ...voiceEffects,
      highpass: 100,
      lowMid: -2,
      presence: 3,
      lowpass: 17000,
      ratio: 2.5,
      makeup: 3,
    },
    fxBypass: false,
  },
  {
    id: 'podcast',
    name: 'presets.podcast',
    effects: {
      ...voiceEffects,
      highpass: 80,
      lowMid: 0.5,
      presence: 2,
      threshold: -22,
      ratio: 3.5,
      attack: 8,
      release: 120,
      makeup: 4,
    },
    fxBypass: false,
  },
  {
    id: 'noise',
    name: 'presets.noise',
    effects: {
      ...voiceEffects,
      highpass: 110,
      lowpass: 12000,
      gateThreshold: -44,
      gateReduction: 6,
    },
    fxBypass: false,
  },
]

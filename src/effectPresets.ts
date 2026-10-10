import { neutralEffects, voiceEffects, type EffectSettings, type Track } from './types'
import type { TranslationKey } from './i18n'

export interface EffectPreset {
  id: string
  name: string
  effects: EffectSettings
  fxBypass: boolean
}

export function matchesPreset(track: Track, preset: EffectPreset) {
  return (
    track.fxBypass === preset.fxBypass &&
    (Object.keys(voiceEffects) as (keyof EffectSettings)[]).every((key) => {
      const actual = track.effects[key] ?? neutralEffects[key]
      const expected = preset.effects[key] ?? neutralEffects[key]
      return typeof actual === 'number' && typeof expected === 'number'
        ? Math.abs(actual - expected) < 0.00001
        : actual === expected
    })
  )
}

export function matchingPreset(track: Track, presets: EffectPreset[], preferredId?: string) {
  const preferred = presets.find((preset) => preset.id === preferredId)
  return preferred && matchesPreset(track, preferred)
    ? preferred
    : presets.find((preset) => matchesPreset(track, preset))
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

import { describe, expect, it } from 'vitest'
import { neutralEffects, type Track } from '../src/types'
import { builtInPresets } from '../src/effectPresets'
import {
  canMove,
  copySelection,
  previewMove,
  selectInterval,
  selectionFromRegions,
  wholeRegion,
} from '../src/timeline/selection'

const tracks: Track[] = ['video', 'voice', 'audio', 'audio'].map((kind, index) => ({
  id: `track-${index}`,
  name: `Track ${index}`,
  kind: kind as Track['kind'],
  color: '#fff',
  mute: false,
  solo: false,
  locked: false,
  armed: kind === 'voice',
  fxBypass: true,
  volume: 0,
  pan: 0,
  effects: neutralEffects,
  clips:
    index === 1 || index === 2
      ? [
          {
            id: `clip-${index}`,
            assetId: 'audio',
            name: 'take.wav',
            start: index,
            offset: 1,
            duration: 8,
          },
        ]
      : [],
}))

describe('timeline sections', () => {
  it('copies intersections while preserving leading silence and relative tracks', () => {
    const selected = selectInterval(tracks, ['track-1', 'track-2'], 0.5, 5)
    const clipboard = copySelection('project', tracks, selected)
    expect(clipboard.duration).toBe(4.5)
    expect(clipboard.trackCount).toBe(2)
    expect(
      clipboard.clips.map((entry) => [
        entry.trackOffset,
        entry.clip.start,
        entry.clip.offset,
        entry.clip.duration,
      ]),
    ).toEqual([
      [0, 0.5, 1, 4],
      [1, 1.5, 1, 3],
    ])
  })
  it('previews partial moves without changing originals or losing outer audio', () => {
    const selected = selectInterval(tracks, ['track-1'], 3, 5)
    const result = previewMove(tracks, selected.regions, 7, 1)
    expect(tracks[1].clips).toHaveLength(1)
    expect(result[1].clips.map((clip) => [clip.start, clip.offset, clip.duration])).toEqual([
      [1, 1, 2],
      [5, 5, 4],
    ])
    expect(result[2].clips.at(-1)).toMatchObject({
      id: 'clip-1',
      start: 10,
      offset: 3,
      duration: 2,
    })
  })
  it('moves voice clips to audio tracks and protects locked or incompatible targets', () => {
    const regions = [wholeRegion(tracks[1], tracks[1].clips[0])]
    expect(canMove(tracks, regions, 1)).toBe(true)
    expect(canMove(tracks, regions, -1)).toBe(false)
    expect(canMove(tracks, regions, 3)).toBe(false)
    expect(
      canMove(
        tracks.map((track, index) => (index === 2 ? { ...track, locked: true } : track)),
        regions,
        1,
      ),
    ).toBe(false)
    expect(selectionFromRegions(tracks, regions)?.start).toBe(1)
    expect(selectionFromRegions(tracks, [])).toBeNull()
  })
})

describe('microphone presets', () => {
  it('provides distinct, bounded processing settings', () => {
    expect(new Set(builtInPresets.map((preset) => preset.id)).size).toBe(6)
    expect(new Set(builtInPresets.map((preset) => JSON.stringify(preset.effects))).size).toBe(6)
    const bounds = {
      highpass: [20, 180],
      lowMid: [-6, 6],
      presence: [-6, 6],
      lowpass: [6000, 20000],
      threshold: [-48, 0],
      ratio: [1, 8],
      attack: [1, 80],
      release: [50, 500],
      makeup: [0, 8],
      gateThreshold: [-70, -25],
      gateReduction: [0, 12],
    }
    for (const preset of builtInPresets)
      for (const [key, [min, max]] of Object.entries(bounds)) {
        const value = preset.effects[key as keyof typeof bounds]
        expect(value).toBeGreaterThanOrEqual(min)
        expect(value).toBeLessThanOrEqual(max)
      }
  })
})

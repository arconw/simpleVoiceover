import { describe, expect, it } from 'vitest'
import { builtInPresets, matchingPreset, matchesPreset } from '../src/effectPresets'
import { soloSources } from '../src/trackState'
import { clipBoundaries, snapDelta } from '../src/timeline/snapping'
import { previewMove, wholeRegion } from '../src/timeline/selection'
import { neutralEffects, type Track } from '../src/types'

const track: Track = {
  id: 'voice',
  name: 'track.voice',
  kind: 'voice',
  color: '#fff',
  mute: false,
  solo: false,
  locked: false,
  armed: true,
  fxBypass: true,
  volume: 0,
  pan: 0,
  effects: neutralEffects,
  clips: [
    { id: 'moving', assetId: 'source', name: 'Take', start: 2, offset: 0, duration: 3 },
    { id: 'other', assetId: 'source', name: 'Take', start: 8, offset: 0, duration: 2 },
  ],
}

describe('clip snapping', () => {
  it('joins either edge to the closest stationary clip boundary', () => {
    const targets = clipBoundaries([track], [track.id], ['moving'])
    expect(targets).toEqual([0, 8, 10])
    expect(snapDelta(2.96, [2, 5], targets, 100)).toBe(3)
    expect(snapDelta(7.97, [2, 5], targets, 100)).toBe(8)
    expect(snapDelta(2.8, [2, 5], targets, 100)).toBe(2.8)
  })
  it('uses a consistent pixel tolerance at different zoom levels', () => {
    expect(snapDelta(2.4, [5], [8], 10)).toBe(3)
    expect(snapDelta(2.4, [5], [8], 100)).toBe(2.4)
    expect(snapDelta(2.94, [5], [8], 100)).toBe(3)
  })
  it('preserves relative positions when a group snaps on its destination track', () => {
    const target: Track = {
      ...track,
      id: 'destination',
      armed: false,
      clips: [{ ...track.clips[1], id: 'target', start: 12 }],
    }
    const regions = track.clips.map((clip) => wholeRegion(track, clip))
    const delta = snapDelta(
      1.95,
      regions.flatMap((region) => [region.from, region.to]),
      clipBoundaries(
        [track, target],
        [target.id],
        regions.map((region) => region.clipId),
      ),
      100,
      -2,
    )
    const result = previewMove([track, target], regions, delta, 1)
    expect(delta).toBe(2)
    expect(result[0].clips).toHaveLength(0)
    expect(
      result[1].clips.filter((clip) => clip.id !== 'target').map((clip) => clip.start),
    ).toEqual([4, 10])
    expect(result[1].clips.find((clip) => clip.id === 'other')!.start + 2).toBe(12)
  })
  it('respects timeline zero, minimum trim duration and source bounds', () => {
    expect(snapDelta(-1.96, [2, 5], [0], 100, -2)).toBe(-2)
    expect(snapDelta(2.95, [5], [8], 100, -2, 2.99)).toBe(2.95)
    expect(snapDelta(-2.01, [2, 5], [-0.03], 100, -2)).toBe(-2.01)
  })
})

describe('preset detection', () => {
  it('recognizes Rust float precision and treats edits and bypass changes as custom', () => {
    const preset = builtInPresets[0]
    const current = {
      ...track,
      fxBypass: false,
      effects: {
        ...preset.effects,
        ratio: Math.fround(preset.effects.ratio),
        makeup: Math.fround(preset.effects.makeup),
      },
    }
    expect(matchesPreset(current, preset)).toBe(true)
    expect(matchingPreset(current, builtInPresets)?.id).toBe('natural')
    expect(
      matchingPreset({ ...current, effects: { ...current.effects, ratio: 3 } }, builtInPresets),
    ).toBeUndefined()
    expect(matchesPreset({ ...current, fxBypass: true }, preset)).toBe(false)
  })
  it('recognizes a named saved preset and honors the selection for identical settings', () => {
    const saved = {
      id: 'saved',
      name: 'My voice',
      fxBypass: false,
      effects: { ...builtInPresets[0].effects, presence: 4 },
    }
    const current = { ...track, fxBypass: false, effects: saved.effects }
    expect(matchingPreset(current, [...builtInPresets, saved])?.name).toBe('My voice')
    const identical = { ...builtInPresets[0], id: 'duplicate', name: 'Named copy' }
    expect(
      matchingPreset(
        { ...current, effects: identical.effects },
        [...builtInPresets, identical],
        'duplicate',
      )?.id,
    ).toBe('duplicate')
  })
})

describe('Solo indicators', () => {
  it('dims only tracks outside the complete Solo group and leaves mute independent', () => {
    const solo = { ...track, id: 'solo', solo: true }
    const second = { ...solo, id: 'second', mute: true }
    expect(soloSources(track, [track])).toEqual([])
    expect(soloSources(track, [track, solo, second])).toEqual([solo, second])
    expect(soloSources(solo, [track, solo, second])).toEqual([])
    expect(soloSources(second, [track, solo, second])).toEqual([])
    expect(track.mute).toBe(false)
  })
})

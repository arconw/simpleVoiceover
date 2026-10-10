import type { Track } from '../types'

export function clipBoundaries(tracks: Track[], trackIds: string[], excludedIds: string[]) {
  const excluded = new Set(excludedIds)
  return [
    0,
    ...tracks
      .filter((track) => trackIds.includes(track.id))
      .flatMap((track) =>
        track.clips
          .filter((clip) => !excluded.has(clip.id))
          .flatMap((clip) => [clip.start, clip.start + clip.duration]),
      ),
  ]
}

export function snapDelta(
  delta: number,
  edges: number[],
  targets: number[],
  pixelsPerSecond: number,
  minimum = -Infinity,
  maximum = Infinity,
) {
  let nearest = delta
  let distance = 8 / pixelsPerSecond
  for (const edge of edges)
    for (const target of targets) {
      const candidate = target - edge
      const difference = Math.abs(candidate - delta)
      if (candidate >= minimum && candidate <= maximum && difference < distance) {
        nearest = candidate
        distance = difference
      }
    }
  return nearest
}

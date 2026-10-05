import type { Clip, Track } from '../types'

export interface ClipRegion {
  trackId: string
  clipId: string
  from: number
  to: number
}

export interface TimelineSelection {
  start: number
  end: number
  trackIds: string[]
  regions: ClipRegion[]
}

export interface ClipClipboard {
  projectId: string
  duration: number
  trackCount: number
  clips: { trackOffset: number; clip: Clip }[]
}

export const wholeRegion = (track: Track, clip: Clip): ClipRegion => ({
  trackId: track.id,
  clipId: clip.id,
  from: clip.start,
  to: clip.start + clip.duration,
})

export function selectInterval(
  tracks: Track[],
  trackIds: string[],
  start: number,
  end: number,
): TimelineSelection {
  const regions = tracks
    .filter((track) => trackIds.includes(track.id))
    .flatMap((track) =>
      track.clips.flatMap((clip) => {
        const from = Math.max(start, clip.start)
        const to = Math.min(end, clip.start + clip.duration)
        return to - from >= 1 / 48000 ? [{ trackId: track.id, clipId: clip.id, from, to }] : []
      }),
    )
  return { start, end, trackIds, regions }
}

export function selectionFromRegions(
  tracks: Track[],
  regions: ClipRegion[],
): TimelineSelection | null {
  if (!regions.length) return null
  return {
    start: Math.min(...regions.map((region) => region.from)),
    end: Math.max(...regions.map((region) => region.to)),
    trackIds: tracks
      .filter((track) => regions.some((region) => region.trackId === track.id))
      .map((track) => track.id),
    regions,
  }
}

export function copySelection(
  projectId: string,
  tracks: Track[],
  selection: TimelineSelection,
): ClipClipboard {
  const first = tracks.findIndex((track) => track.id === selection.trackIds[0])
  return {
    projectId,
    duration: selection.end - selection.start,
    trackCount: selection.trackIds.length,
    clips: selection.regions.flatMap((region) => {
      const index = tracks.findIndex((track) => track.id === region.trackId)
      const clip = tracks[index]?.clips.find((clip) => clip.id === region.clipId)
      return clip
        ? [
            {
              trackOffset: index - first,
              clip: {
                ...clip,
                start: region.from - selection.start,
                offset: clip.offset + region.from - clip.start,
                duration: region.to - region.from,
              },
            },
          ]
        : []
    }),
  }
}

export function previewMove(
  tracks: Track[],
  regions: ClipRegion[],
  delta: number,
  shift: number,
): Track[] {
  const result = tracks.map((track) => ({ ...track, clips: [...track.clips] }))
  for (const region of regions) {
    const source = tracks.findIndex((track) => track.id === region.trackId)
    const clip = tracks[source]?.clips.find((clip) => clip.id === region.clipId)
    if (!clip || !result[source + shift]) continue
    result[source].clips = result[source].clips.filter((item) => item.id !== clip.id)
    if (region.from > clip.start + 1e-8)
      result[source].clips.push({
        ...clip,
        id: `${clip.id}-left`,
        duration: region.from - clip.start,
      })
    if (region.to < clip.start + clip.duration - 1e-8)
      result[source].clips.push({
        ...clip,
        id: `${clip.id}-right`,
        start: region.to,
        offset: clip.offset + region.to - clip.start,
        duration: clip.start + clip.duration - region.to,
      })
    result[source + shift].clips.push({
      ...clip,
      start: region.from + delta,
      offset: clip.offset + region.from - clip.start,
      duration: region.to - region.from,
    })
  }
  return result
}

export function canMove(tracks: Track[], regions: ClipRegion[], shift: number): boolean {
  return regions.every((region) => {
    const source = tracks.findIndex((track) => track.id === region.trackId)
    const target = tracks[source + shift]
    return (
      source >= 0 &&
      !tracks[source].locked &&
      !!target &&
      !target.locked &&
      (tracks[source].kind === 'video') === (target.kind === 'video')
    )
  })
}

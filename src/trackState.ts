import type { Track } from './types'

export function soloSources(track: Track, tracks: Track[]) {
  return track.solo ? [] : tracks.filter((entry) => entry.solo)
}

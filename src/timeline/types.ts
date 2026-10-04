import type { Track, MediaAsset } from '../types'
export interface TimelineProps {
  tracks: Track[]
  assets: MediaAsset[]
  selectedTrackId: string
  selectedClipId: string | null
  onSelectTrack: (id: string) => void
  onSelectClip: (id: string | null) => void
  onUpdateTrack: (id: string, patch: Partial<Track>) => void
  onEditClip: (trackId: string, clipId: string, mode: string, delta: number) => void
  onSplitClip: (trackId: string, clipId: string, position: number) => void
  onSeek: (time: number) => void
  position: number
  duration: number
  recording: boolean
  recordStart: number
  tool: 'select' | 'split'
  onToolChange: (tool: 'select' | 'split') => void
  onAddTrack: () => void
  onRemoveClip: () => void
  getLevel: (id: string) => number
}

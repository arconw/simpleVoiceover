import type { Track, MediaAsset } from '../types'
import type { ReactNode } from 'react'
import type { ClipRegion, TimelineSelection } from './selection'
export interface TimelineProps {
  transport?: ReactNode
  tracks: Track[]
  assets: MediaAsset[]
  selectedTrackId: string
  selectedClipId: string | null
  selection: TimelineSelection | null
  onSelectRegion: (selection: TimelineSelection | null) => void
  onMoveRegions: (regions: ClipRegion[], delta: number, shift: number) => Promise<boolean>
  onCopy: () => void
  onPaste: () => void
  canPaste: boolean
  onSelectTrack: (id: string) => void
  onSelectClip: (id: string | null) => void
  onUpdateTrack: (id: string, patch: Partial<Track>) => void
  onEditClip: (trackId: string, clipId: string, mode: string, delta: number) => Promise<boolean>
  onSplitClip: (trackId: string, clipId: string, position: number) => void
  onSeek: (time: number) => void
  position: number
  duration: number
  recording: boolean
  operationPending: boolean
  recordStart: number
  tool: 'select' | 'split'
  onToolChange: (tool: 'select' | 'split') => void
  snapping: boolean
  onSnappingChange: (enabled: boolean) => void
  onAddTrack: () => void
  onRemoveTrack: (track: Track) => void
  onRemoveClip: () => void
  getLevel: (id: string) => number
}

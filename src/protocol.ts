import { t } from './i18n'
import type { Track, MediaAsset } from './types'
import type { LanguagePreference } from './i18n'
import type { EffectPreset } from './effectPresets'
import type { ClipRegion, ClipClipboard } from './timeline/selection'

export interface OperationProgress {
  label: string
  percent: number
}

export interface Snapshot {
  project: { id: string; name: string; tracks: Track[]; assets: Omit<MediaAsset, 'url'>[] }
  config: {
    workingDirectory: string
    projectFile: string | null
    dirty: boolean
    language?: LanguagePreference
    effectPresets?: EffectPreset[]
  }
  duration: number
  canUndo: boolean
  canRedo: boolean
}
export interface CommandPayloads {
  snapshot: Record<string, never>
  preferences_patch: { preference: LanguagePreference }
  preset_save: Omit<EffectPreset, 'id'>
  regions_edit: { regions: ClipRegion[]; delta: number; trackOffset: number; remove?: boolean }
  clips_paste: {
    projectId: string
    clips: ClipClipboard['clips']
    trackId: string
    position: number
  }
  working_directory: { path?: string }
  save: { saveAs?: boolean }
  open: Record<string, never>
  import_native: { trackId: string; position: number }
  import_paths: { paths: string[]; trackId: string; position: number }
  export: { trackId: string | null; format: 'wav' | 'mp3' }
  track_patch: { trackId: string; patch: Partial<Track> }
  track_add: Record<string, never>
  clip_remove: { trackId: string; clipId: string | null }
  clip_place: { trackId: string; assetId: string; position: number }
  asset_remove: { assetId: string }
  clip_edit: { trackId: string; clipId: string; mode: string; delta: number }
  clip_split: { trackId: string; clipId: string; position: number }
  undo: Record<string, never>
  redo: Record<string, never>
  play: { position: number }
  pause: Record<string, never>
  monitor: { enabled: boolean }
  record_begin: { position: number }
  record_end: Record<string, never>
}
export type CommandName = keyof CommandPayloads
export interface CommandResult {
  saved?: boolean
  path?: string
  snapshot?: Snapshot
  regions?: ClipRegion[]
}

export function decodeAudioPacket(data: ArrayBuffer) {
  if (data.byteLength < 16 || (data.byteLength - 16) % 8 !== 0)
    throw new Error(t('error.audioPacketCorrupt'))
  const view = new DataView(data)
  if (
    view.getUint32(0, true) !== 0x504f5653 ||
    view.getUint32(4, true) * 8 + 16 !== data.byteLength
  )
    throw new Error(t('error.audioPacketFormat'))
  return { samples: new Float32Array(data.slice(16)), position: view.getFloat64(8, true) }
}

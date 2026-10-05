import {
  setLanguagePreference,
  t,
  useLanguagePreference,
  validPreference,
  type LanguagePreference,
} from './i18n'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { StudioClient } from './services/StudioClient'
import type { CommandName, CommandPayloads, Snapshot, OperationProgress } from './protocol'
import type { MediaAsset, Track } from './types'
import { useTransport } from './hooks/useTransport'
import { useDesktopLifecycle } from './hooks/useDesktopLifecycle'
import { useStudioShortcuts } from './hooks/useStudioShortcuts'
import { useNativeDrop } from './hooks/useNativeDrop'
import { useOperationIndicator } from './hooks/useOperationIndicator'
import {
  copySelection,
  selectionFromRegions,
  wholeRegion,
  type ClipClipboard,
  type ClipRegion,
  type TimelineSelection,
} from './timeline/selection'

export function useStudio() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null)
  const [selectedTrackId, setSelectedTrackId] = useState('')
  const [selectedClipId, setSelectedClipId] = useState<string | null>(null)
  const [selection, setSelection] = useState<TimelineSelection | null>(null)
  const [clipboard, setClipboard] = useState<ClipClipboard | null>(null)
  const [tab, setTab] = useState('files')
  const [tool, setTool] = useState<'select' | 'split'>('select')
  const [busy, setBusy] = useState('')
  const visibleBusy = useOperationIndicator(busy)
  const [progress, setProgress] = useState<OperationProgress | null>(null)
  const [notice, setNotice] = useState('')
  const [error, setError] = useState('')
  const [help, setHelp] = useState(false)
  const [settings, setSettings] = useState(false)
  const languagePreference = useLanguagePreference()
  const engineRef = useRef<StudioClient | null>(null)
  const tracks = snapshot?.project.tracks ?? []
  const assets = useMemo<MediaAsset[]>(
    () =>
      (snapshot?.project.assets ?? []).map((asset) => ({ ...asset, url: `/media/${asset.id}` })),
    [snapshot],
  )
  const duration = snapshot?.duration ?? 0
  const selectedTrack = tracks.find((track) => track.id === selectedTrackId) ?? tracks[0]
  const selectClip = (id: string | null) => {
    setSelectedClipId(id)
    setSelection(null)
  }
  const selectRegion = (value: TimelineSelection | null) => {
    setSelection(value)
    setSelectedClipId(null)
  }
  const armedTrack = tracks.find((track) => track.armed)
  const showError = useCallback(
    (reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)),
    [],
  )
  const request = useCallback(
    <K extends CommandName>(command: K, payload: CommandPayloads[K] = {} as CommandPayloads[K]) => {
      if (!engineRef.current) return Promise.reject(new Error(t('error.studioConnecting')))
      return engineRef.current.request(command, payload)
    },
    [],
  )
  useEffect(() => {
    const client = new StudioClient()
    engineRef.current = client
    client.onSnapshot = (value) => {
      setLanguagePreference(
        validPreference(value.config.language) ? value.config.language : 'system',
      )
      setSnapshot(value)
      setSelectedTrackId((previous) =>
        value.project.tracks.some((track) => track.id === previous)
          ? previous
          : (value.project.tracks.find((track) => track.armed)?.id ??
            value.project.tracks[0]?.id ??
            ''),
      )
    }
    client.onError = showError
    client.onProgress = setProgress
    void client.connect().catch(showError)
    return () => {
      client.close()
      engineRef.current = null
    }
  }, [showError])
  const transport = useTransport({
    engineRef,
    busy,
    setBusy,
    setProgress,
    setNotice,
    showError,
    duration,
    armedTrack,
    assets,
    tracks,
  })
  const action = async <K extends CommandName>(
    label: string,
    command: K,
    payload: CommandPayloads[K] = {} as CommandPayloads[K],
  ) => {
    if (busy || transport.recording) return
    setBusy(label)
    setProgress(null)
    setError('')
    try {
      await transport.pause()
      return await request(command, payload)
    } catch (reason) {
      showError(reason)
    } finally {
      setBusy('')
      setProgress(null)
    }
  }
  const updateTrack = (id: string, patch: Partial<Track>) => {
    if (busy || (transport.recording && 'armed' in patch)) return
    void request('track_patch', { trackId: id, patch }).catch(showError)
  }
  const changeLanguage = async (preference: LanguagePreference) => {
    if (busy) return
    const previous = languagePreference
    setLanguagePreference(preference)
    try {
      await request('preferences_patch', { preference })
    } catch (reason) {
      setLanguagePreference(previous)
      showError(reason)
    }
  }
  const savePreset = async (name: string, track: Track) => {
    if (busy) return false
    try {
      await request('preset_save', { name, effects: track.effects, fxBypass: track.fxBypass })
      setNotice(t('presets.saved', { name }))
      return true
    } catch (reason) {
      showError(reason)
      return false
    }
  }
  const importNative = async () => {
    await action(t('action.openMedia'), 'import_native', {
      trackId: selectedTrackId,
      position: transport.position,
    })
  }
  const importPaths = async (paths: string[]) => {
    await action(t('action.import'), 'import_paths', {
      paths,
      trackId: selectedTrackId,
      position: transport.position,
    })
  }
  const draggingFiles = useNativeDrop(importPaths, showError)
  const chooseWorkingDirectory = async () => {
    await action(t('action.workingDirectory'), 'working_directory')
  }
  const saveProject = async (saveAs = false) => {
    const result = await action(t('action.save'), 'save', { saveAs })
    if (result?.saved) setNotice(t('notice.saved', { value0: result.path ?? '' }))
    return result?.saved === true
  }
  const openProject = async () => {
    const result = await action(t('action.openProject'), 'open')
    if (result) {
      transport.seek(0)
      setSelectedClipId(null)
      setSelection(null)
      setClipboard(null)
    }
  }
  const lifecycle = useDesktopLifecycle({
    dirty: snapshot?.config.dirty ?? true,
    busy,
    recording: transport.recording,
    saveProject,
    openProject,
    pause: transport.pause,
    showError,
  })
  const exportAudio = async (trackId: string | null = null, format: 'wav' | 'mp3' = 'wav') => {
    const result = await action(t('action.export', { value0: format.toUpperCase() }), 'export', {
      trackId,
      format,
    })
    if (result?.saved)
      setNotice(t('notice.exported', { value0: format.toUpperCase(), value1: result.path ?? '' }))
  }
  const removeClip = () => {
    if (selection?.regions.length) {
      void action(t('action.removeClip'), 'regions_edit', {
        regions: selection.regions,
        delta: 0,
        trackOffset: 0,
        remove: true,
      }).then((result) => {
        if (result) setSelection(null)
      })
      return
    }
    const track = tracks.find((track) => track.clips.some((clip) => clip.id === selectedClipId))
    if (track)
      void action(t('action.removeClip'), 'clip_remove', {
        trackId: track.id,
        clipId: selectedClipId,
      })
    setSelectedClipId(null)
  }
  const undo = () => {
    void action(t('action.undo'), 'undo').then((result) => {
      if (result) selectClip(null)
    })
  }
  const redo = () => {
    void action(t('action.redo'), 'redo').then((result) => {
      if (result) selectClip(null)
    })
  }
  const copyClips = () => {
    if (!snapshot || busy || transport.recording) return
    const track = tracks.find((track) => track.clips.some((clip) => clip.id === selectedClipId))
    const clip = track?.clips.find((clip) => clip.id === selectedClipId)
    const selected =
      selection ?? (track && clip ? selectionFromRegions(tracks, [wholeRegion(track, clip)]) : null)
    if (!selected?.regions.length) return
    setClipboard(copySelection(snapshot.project.id, tracks, selected))
    setNotice(t('selection.copied'))
  }
  const pasteClips = async () => {
    if (!clipboard || clipboard.projectId !== snapshot?.project.id) return
    const result = await action(t('selection.paste'), 'clips_paste', {
      projectId: clipboard.projectId,
      clips: clipboard.clips,
      trackId: selectedTrackId,
      position: transport.position,
    })
    if (result?.snapshot) setSnapshot(result.snapshot)
    if (result?.regions) {
      const first = tracks.findIndex((track) => track.id === selectedTrackId)
      selectRegion({
        start: transport.position,
        end: transport.position + clipboard.duration,
        trackIds: tracks.slice(first, first + clipboard.trackCount).map((track) => track.id),
        regions: result.regions,
      })
    }
  }
  const moveRegions = async (regions: ClipRegion[], delta: number, trackOffset: number) => {
    const result = await action(t('action.editClip'), 'regions_edit', {
      regions,
      delta,
      trackOffset,
    })
    if (result?.snapshot) setSnapshot(result.snapshot)
    if (result?.regions) {
      const original = selection ?? selectionFromRegions(tracks, regions)
      selectRegion(
        original
          ? {
              ...original,
              start: original.start + delta,
              end: original.end + delta,
              trackIds: original.trackIds
                .map((id) => tracks[tracks.findIndex((track) => track.id === id) + trackOffset]?.id)
                .filter((id): id is string => !!id),
              regions: result.regions,
            }
          : null,
      )
    }
    return !!result
  }
  const placeAsset = (asset: MediaAsset) => {
    void action(t('action.placeClip'), 'clip_place', {
      trackId: selectedTrackId,
      assetId: asset.id,
      position: transport.position,
    })
  }
  const removeAsset = (asset: MediaAsset) => {
    void action(t('action.removeAsset'), 'asset_remove', { assetId: asset.id }).then((result) => {
      if (result) selectClip(null)
    })
  }
  const editClip = (trackId: string, clipId: string, mode: string, delta: number) => {
    return action(t('action.editClip'), 'clip_edit', { trackId, clipId, mode, delta }).then(Boolean)
  }
  const splitClip = (trackId: string, clipId: string, position: number) => {
    void action(t('action.splitClip'), 'clip_split', { trackId, clipId, position })
  }
  const addTrack = () => {
    void action(t('action.addTrack'), 'track_add')
  }
  const getLevel = useCallback((id: string) => engineRef.current?.getLevel(id) ?? 0, [])
  useStudioShortcuts({
    play: transport.play,
    record: transport.record,
    saveProject,
    undo,
    redo,
    removeClip,
    copyClips,
    pasteClips,
    seek: transport.seek,
    setTool,
    blocked: !!lifecycle.confirmMode || help || settings,
  })
  return {
    snapshot,
    tracks,
    assets,
    selectedTrackId,
    setSelectedTrackId,
    selectedClipId,
    setSelectedClipId: selectClip,
    selection,
    setSelection: selectRegion,
    copyClips,
    pasteClips,
    moveRegions,
    canPaste: !!clipboard && clipboard.projectId === snapshot?.project.id,
    tab,
    setTab,
    tool,
    setTool,
    busy: visibleBusy,
    operationPending: !!busy,
    progress,
    notice,
    setNotice,
    error,
    setError,
    help,
    setHelp,
    settings,
    setSettings,
    languagePreference,
    changeLanguage,
    savePreset,
    effectPresets: snapshot?.config.effectPresets ?? [],
    draggingFiles,
    engineRef,
    duration,
    selectedTrack,
    armedTrack,
    updateTrack,
    saveProject,
    exportAudio,
    removeClip,
    undo,
    redo,
    getLevel,
    placeAsset,
    removeAsset,
    showError,
    editClip,
    splitClip,
    addTrack,
    chooseWorkingDirectory,
    importNative,
    openNative: lifecycle.requestOpen,
    ...transport,
    ...lifecycle,
  }
}

export type StudioController = ReturnType<typeof useStudio>

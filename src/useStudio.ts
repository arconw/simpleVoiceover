import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { StudioClient } from './services/StudioClient'
import type { CommandName, CommandPayloads, Snapshot } from './protocol'
import type { MediaAsset, Track } from './types'
import { useTransport } from './hooks/useTransport'
import { useDesktopLifecycle } from './hooks/useDesktopLifecycle'
import { useStudioShortcuts } from './hooks/useStudioShortcuts'
import { useNativeDrop } from './hooks/useNativeDrop'

export function useStudio() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null)
  const [selectedTrackId, setSelectedTrackId] = useState('')
  const [selectedClipId, setSelectedClipId] = useState<string | null>(null)
  const [tab, setTab] = useState('files')
  const [tool, setTool] = useState<'select' | 'split'>('select')
  const [busy, setBusy] = useState('')
  const [notice, setNotice] = useState('')
  const [error, setError] = useState('')
  const [help, setHelp] = useState(false)
  const engineRef = useRef<StudioClient | null>(null)
  const tracks = snapshot?.project.tracks ?? []
  const assets = useMemo<MediaAsset[]>(
    () =>
      (snapshot?.project.assets ?? []).map((asset) => ({ ...asset, url: `/media/${asset.id}` })),
    [snapshot],
  )
  const duration = snapshot?.duration ?? 0
  const selectedTrack = tracks.find((track) => track.id === selectedTrackId) ?? tracks[0]
  const armedTrack = tracks.find((track) => track.armed)
  const showError = useCallback(
    (reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)),
    [],
  )
  const request = useCallback(
    <K extends CommandName>(command: K, payload: CommandPayloads[K] = {} as CommandPayloads[K]) => {
      if (!engineRef.current) return Promise.reject(new Error('Студия ещё подключается.'))
      return engineRef.current.request(command, payload)
    },
    [],
  )
  useEffect(() => {
    const client = new StudioClient()
    engineRef.current = client
    client.onSnapshot = (value) => {
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
    setError('')
    try {
      await transport.pause()
      return await request(command, payload)
    } catch (reason) {
      showError(reason)
    } finally {
      setBusy('')
    }
  }
  const updateTrack = (id: string, patch: Partial<Track>) => {
    if (busy || (transport.recording && 'armed' in patch)) return
    void request('track_patch', { trackId: id, patch }).catch(showError)
  }
  const importNative = async () => {
    await action('Открываю медиа…', 'import_native', {
      trackId: selectedTrackId,
      position: transport.position,
    })
  }
  const importPaths = async (paths: string[]) => {
    await action('Импортирую медиа с диска…', 'import_paths', {
      paths,
      trackId: selectedTrackId,
      position: transport.position,
    })
  }
  const draggingFiles = useNativeDrop(importPaths, showError)
  const chooseWorkingDirectory = async () => {
    await action('Выбираю рабочий каталог…', 'working_directory')
  }
  const saveProject = async (saveAs = false) => {
    const result = await action('Сохраняю проект и кэш…', 'save', { saveAs })
    if (result?.saved)
      setNotice(`Проект сохранён: ${result.path}. Кэш и история изменений очищены.`)
    return result?.saved === true
  }
  const openProject = async () => {
    const result = await action('Открываю проект…', 'open')
    if (result) {
      transport.seek(0)
      setSelectedClipId(null)
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
    const result = await action(`Рендерю ${format.toUpperCase()} в Rust…`, 'export', {
      trackId,
      format,
    })
    if (result?.saved) setNotice(`${format.toUpperCase()} сохранён: ${result.path}`)
  }
  const removeClip = () => {
    const track = tracks.find((track) => track.clips.some((clip) => clip.id === selectedClipId))
    if (track)
      void action('Убираю клип…', 'clip_remove', { trackId: track.id, clipId: selectedClipId })
    setSelectedClipId(null)
  }
  const undo = () => {
    void action('Отменяю изменение…', 'undo')
  }
  const redo = () => {
    void action('Возвращаю изменение…', 'redo')
  }
  const placeAsset = (asset: MediaAsset) => {
    void action('Добавляю клип…', 'clip_place', {
      trackId: selectedTrackId,
      assetId: asset.id,
      position: transport.position,
    })
  }
  const editClip = (trackId: string, clipId: string, mode: string, delta: number) => {
    void action('Монтирую клип…', 'clip_edit', { trackId, clipId, mode, delta })
  }
  const splitClip = (trackId: string, clipId: string, position: number) => {
    void action('Разрезаю клип…', 'clip_split', { trackId, clipId, position })
  }
  const addTrack = () => {
    void action('Добавляю дорожку…', 'track_add')
  }
  const getLevel = useCallback((id: string) => engineRef.current?.getLevel(id) ?? 0, [])
  useStudioShortcuts({
    play: transport.play,
    record: transport.record,
    saveProject,
    undo,
    redo,
    removeClip,
    seek: transport.seek,
    setTool,
    blocked: !!lifecycle.confirmMode || help,
  })
  return {
    snapshot,
    tracks,
    assets,
    selectedTrackId,
    setSelectedTrackId,
    selectedClipId,
    setSelectedClipId,
    tab,
    setTab,
    tool,
    setTool,
    busy,
    notice,
    setNotice,
    error,
    setError,
    help,
    setHelp,
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

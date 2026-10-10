import { t } from '../i18n'
import { useEffect, useRef, useState, type MutableRefObject } from 'react'
import type { StudioClient } from '../services/StudioClient'
import type { MediaAsset, Track } from '../types'
import type { OperationProgress } from '../protocol'
import { prepareVideoPosition, synchronizeVideo } from '../video/playback'

interface Options {
  engineRef: MutableRefObject<StudioClient | null>
  busy: string
  setBusy: (value: string) => void
  setProgress: (value: OperationProgress | null) => void
  setNotice: (value: string) => void
  showError: (reason: unknown) => void
  duration: number
  armedTrack?: Track
  assets: MediaAsset[]
  tracks: Track[]
}

export function useTransport({
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
}: Options) {
  const [position, setPosition] = useState(0)
  const [seekRevision, setSeekRevision] = useState(0)
  const seekSequence = useRef(0)
  const [playing, setPlaying] = useState(false)
  const [seekPending, setSeekPending] = useState(false)
  const seekRequest = useRef(0)
  const [recording, setRecording] = useState(false)
  const [recordStart, setRecordStart] = useState(0)
  const [monitor, setMonitorState] = useState(false)
  const [inputLevel, setInputLevel] = useState(0)
  const videoRef = useRef<HTMLVideoElement>(null)
  const engine = () => {
    if (!engineRef.current) throw new Error(t('error.audioConnecting'))
    return engineRef.current
  }
  useEffect(() => {
    const client = engineRef.current
    if (!client) return
    client.onPosition = setPosition
    client.onEnded = () => setPlaying(false)
    client.onInput = setInputLevel
  }, [engineRef])
  const pause = async () => {
    seekRequest.current++
    setSeekPending(false)
    setPlaying(false)
    videoRef.current?.pause()
    if (engineRef.current) await engineRef.current.pause()
  }
  const preparePreview = async (time: number, request: number, revision: number) => {
    const clip = tracks
      .find((track) => track.kind === 'video')
      ?.clips.find((clip) => time >= clip.start && time < clip.start + clip.duration)
    const asset = assets.find((asset) => asset.id === clip?.assetId)
    if (!clip || !asset) return
    await new Promise<void>((resolve) => setTimeout(resolve, 0))
    const video = videoRef.current
    if (!video || !clip || video.getAttribute('src') !== asset?.url) return
    await prepareVideoPosition(
      video,
      time - clip.start + clip.offset,
      showError,
      revision,
      () => request === seekRequest.current && video === videoRef.current,
    )
  }
  const seek = (time: number) => {
    if (recording || busy) return
    const next = Math.max(0, time)
    setPosition(next)
    const sequence = ++seekSequence.current
    setSeekRevision(sequence)
    if (playing) {
      const revision = ++seekRequest.current
      setSeekPending(true)
      videoRef.current?.pause()
      void engine()
        .pause()
        .then(() => preparePreview(next, revision, sequence))
        .then(() => {
          if (revision === seekRequest.current) return engine().play(next)
        })
        .catch(showError)
        .finally(() => {
          if (revision === seekRequest.current) setSeekPending(false)
        })
    }
  }
  const play = async () => {
    if (busy || recording) return
    try {
      if (playing) {
        await pause()
        return
      }
      if (!duration) {
        setNotice(t('notice.addMedia'))
        return
      }
      setBusy(t('action.preparePlayback'))
      setProgress(null)
      const next = position >= duration ? 0 : position
      setPosition(next)
      await engine().openAudio()
      await preparePreview(next, seekRequest.current, seekSequence.current)
      await engine().play(next)
      setPlaying(true)
    } catch (reason) {
      showError(reason)
    } finally {
      setBusy('')
      setProgress(null)
    }
  }
  const record = async () => {
    if (busy) return
    if (recording) {
      setBusy(t('action.saveTake'))
      try {
        await engine().stopRecording()
        setNotice(t('notice.takeSaved'))
      } catch (reason) {
        showError(reason)
      } finally {
        setRecording(false)
        setPlaying(false)
        setBusy('')
        setInputLevel(0)
        videoRef.current?.pause()
      }
      return
    }
    if (!armedTrack) {
      setNotice(t('error.armOneTrack'))
      return
    }
    setBusy(t('action.connectMicrophone'))
    try {
      await pause()
      await engine().prepareMicrophone()
      await preparePreview(position, seekRequest.current, seekSequence.current)
      await engine().startRecording(position, monitor)
      setRecordStart(position)
      setRecording(true)
      setPlaying(true)
    } catch (reason) {
      showError(reason)
      await engine()
        .stopRecording()
        .catch(() => {})
      setPlaying(false)
    } finally {
      setBusy('')
    }
  }
  const setMonitor = (enabled: boolean) => {
    setMonitorState(enabled)
    engineRef.current?.setMonitor(enabled)
  }
  const previewPosition = duration > 0 && position >= duration ? duration - 0.001 : position
  const videoClip = tracks
    .find((track) => track.kind === 'video')
    ?.clips.find(
      (clip) => previewPosition >= clip.start && previewPosition < clip.start + clip.duration,
    )
  const videoAsset = assets.find((asset) => asset.id === videoClip?.assetId)
  const videoFiles = assets.filter((asset) => asset.kind === 'video')
  const previewPlaying = playing && !seekPending
  useEffect(() => {
    const video = videoRef.current
    if (!video || !videoClip) return
    const desired = previewPosition - videoClip.start + videoClip.offset
    synchronizeVideo(video, desired, previewPlaying, showError, seekRevision)
  }, [previewPosition, previewPlaying, videoClip, showError, seekRevision])
  return {
    position,
    seekRevision,
    playing,
    previewPlaying,
    recording,
    recordStart,
    monitor,
    setMonitor,
    inputLevel,
    videoRef,
    previewPosition,
    videoClip,
    videoAsset,
    videoFiles,
    pause,
    seek,
    play,
    record,
  }
}

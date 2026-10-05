import { t } from '../i18n'
import { useEffect, useRef, useState, type MutableRefObject } from 'react'
import type { StudioClient } from '../services/StudioClient'
import type { MediaAsset, Track } from '../types'
import type { OperationProgress } from '../protocol'

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
  const [playing, setPlaying] = useState(false)
  const [recording, setRecording] = useState(false)
  const [recordStart, setRecordStart] = useState(0)
  const [monitor, setMonitorState] = useState(false)
  const [inputLevel, setInputLevel] = useState(0)
  const [devices, setDevices] = useState<MediaDeviceInfo[]>([])
  const [deviceId, setDeviceId] = useState('')
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
    setPlaying(false)
    videoRef.current?.pause()
    if (engineRef.current) await engineRef.current.pause()
  }
  const seek = (time: number) => {
    if (recording || busy) return
    const next = Math.max(0, time)
    setPosition(next)
    if (playing) void engine().play(next).catch(showError)
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
      await engine().play(position >= duration ? 0 : position)
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
      await engine().prepareMicrophone(deviceId)
      setDevices(
        (await navigator.mediaDevices.enumerateDevices()).filter(
          (device) => device.kind === 'audioinput',
        ),
      )
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
  useEffect(() => {
    const video = videoRef.current
    if (!video || !videoClip) return
    const desired = previewPosition - videoClip.start + videoClip.offset
    if (Math.abs(video.currentTime - desired) > 0.12) video.currentTime = desired
    if (playing && video.paused) void video.play().catch(() => {})
    if (!playing && !video.paused) video.pause()
  }, [previewPosition, playing, videoClip])
  return {
    position,
    playing,
    recording,
    recordStart,
    monitor,
    setMonitor,
    inputLevel,
    devices,
    deviceId,
    setDeviceId,
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

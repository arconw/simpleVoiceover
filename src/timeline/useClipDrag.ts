import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import type { Clip, Track, MediaAsset } from '../types'
import type { TimelineProps } from './types'
import type { TimelineViewport } from './useTimelineViewport'
import { clamp } from './viewport'
interface ClipDrag {
  pointerId: number
  originX: number
  pixelsPerSecond: number
  trackId: string
  clip: Clip
  mode: 'move' | 'left' | 'right'
  sourceDuration: number
  moved: boolean
  next: Clip
}

const minimumClipDuration = 1 / 48000

export function useClipDrag(
  props: TimelineProps,
  viewport: TimelineViewport,
  assetMap: Map<string, MediaAsset>,
) {
  const {
    tracks,
    recording,
    operationPending,
    tool,
    onSelectTrack,
    onSelectClip,
    onSplitClip,
    onEditClip,
  } = props
  const { rulerRef, view, pixelsPerSecond } = viewport
  const [preview, setPreview] = useState<{ trackId: string; clip: Clip } | null>(null)
  const drag = useRef<ClipDrag | null>(null)
  const pending = useRef<ClipDrag | null>(null)
  useEffect(() => {
    const current = pending.current
    if (!current) return
    const clip = tracks
      .find((track) => track.id === current.trackId)
      ?.clips.find((clip) => clip.id === current.clip.id)
    if (
      !clip ||
      ['start', 'offset', 'duration'].every(
        (field) =>
          Math.abs(
            clip[field as keyof Pick<Clip, 'start' | 'offset' | 'duration'>] -
              current.next[field as keyof Pick<Clip, 'start' | 'offset' | 'duration'>],
          ) < 1e-8,
      )
    ) {
      pending.current = null
      setPreview(null)
    }
  }, [tracks])
  const beginClipDrag = (event: ReactPointerEvent<HTMLDivElement>, track: Track, clip: Clip) => {
    if (event.button !== 0) return
    event.stopPropagation()
    onSelectTrack(track.id)
    onSelectClip(clip.id)
    if (track.locked || recording || operationPending || pending.current) return
    const edge = (event.target as HTMLElement).closest<HTMLElement>('[data-trim]')?.dataset.trim
    if (tool === 'split' && !edge) {
      const left = rulerRef.current?.getBoundingClientRect().left ?? 0
      const time = view.start + (event.clientX - left) / pixelsPerSecond
      onSplitClip(track.id, clip.id, time)
      return
    }
    event.preventDefault()
    event.currentTarget.setPointerCapture(event.pointerId)
    drag.current = {
      pointerId: event.pointerId,
      originX: event.clientX,
      pixelsPerSecond,
      trackId: track.id,
      clip,
      mode: edge === 'left' || edge === 'right' ? edge : 'move',
      sourceDuration: assetMap.get(clip.assetId)?.duration ?? clip.offset + clip.duration,
      moved: false,
      next: clip,
    }
  }

  const moveClip = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = drag.current
    if (!current || event.pointerId !== current.pointerId) return
    if (!current.moved && Math.abs(event.clientX - current.originX) < 3) return
    current.moved = true
    const delta = (event.clientX - current.originX) / current.pixelsPerSecond
    const original = current.clip
    let next = { ...original }
    if (current.mode === 'move') next.start = Math.max(0, original.start + delta)
    if (current.mode === 'left') {
      const adjusted = clamp(
        delta,
        -Math.min(original.start, original.offset),
        original.duration - minimumClipDuration,
      )
      next = {
        ...next,
        start: original.start + adjusted,
        offset: original.offset + adjusted,
        duration: original.duration - adjusted,
      }
    }
    if (current.mode === 'right')
      next.duration = clamp(
        original.duration + delta,
        minimumClipDuration,
        current.sourceDuration - original.offset,
      )
    current.next = next
    setPreview({ trackId: current.trackId, clip: next })
  }

  const finishClip = (event: ReactPointerEvent<HTMLDivElement>, cancelled = false) => {
    const current = drag.current
    if (!current || event.pointerId !== current.pointerId) return
    const delta =
      current.mode === 'right'
        ? current.next.duration - current.clip.duration
        : current.next.start - current.clip.start
    if (current.moved && !cancelled && delta !== 0) {
      pending.current = current
      void onEditClip(current.trackId, current.clip.id, current.mode, delta).then((saved) => {
        if (!saved && pending.current === current) {
          pending.current = null
          setPreview(null)
        }
      })
    } else setPreview(null)
    drag.current = null
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId)
  }

  return { preview, drag, beginClipDrag, moveClip, finishClip }
}
export type ClipDragging = ReturnType<typeof useClipDrag>

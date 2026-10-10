import { useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import type { Clip, Track } from '../types'
import type { TimelineProps } from './types'
import type { TimelineViewport } from './useTimelineViewport'
import {
  canMove,
  previewMove,
  selectInterval,
  selectionFromRegions,
  wholeRegion,
  type TimelineSelection,
} from './selection'
import { clamp } from './viewport'
import { clipBoundaries, snapDelta } from './snapping'

interface Gesture {
  pointerId: number
  x: number
  y: number
  trackIndex: number
  moved: boolean
  mode: 'marquee' | 'move'
  selection: TimelineSelection | null
  time: number
  tracks: Track[]
  delta: number
  shift: number
  pixelsPerSecond: number
}

export function useTimelineSelection(props: TimelineProps, viewport: TimelineViewport) {
  const gesture = useRef<Gesture | null>(null)
  const pending = useRef(false)
  const [preview, setPreview] = useState<{ tracks: Track[]; selection: TimelineSelection } | null>(
    null,
  )
  const timeAt = (x: number) => {
    const bounds = viewport.rulerRef.current?.getBoundingClientRect()
    return (
      Math.round(
        (viewport.view.start +
          clamp(x - (bounds?.left ?? 0), 0, viewport.laneWidth) / viewport.pixelsPerSecond) *
          48000,
      ) / 48000
    )
  }
  const indexAt = (y: number) => {
    const lanes = viewport.timelineRef.current?.querySelectorAll<HTMLElement>('.tl-lane')
    if (!lanes?.length) return 0
    for (let index = 0; index < lanes.length; index++)
      if (y < lanes[index].getBoundingClientRect().bottom) return index
    return lanes.length - 1
  }
  const begin = (
    event: ReactPointerEvent<HTMLElement>,
    track: Track,
    mode: Gesture['mode'],
    selection: TimelineSelection | null,
  ) => {
    if (event.button !== 0 || props.recording || props.operationPending || pending.current) return
    event.preventDefault()
    event.stopPropagation()
    viewport.timelineRef.current?.setPointerCapture(event.pointerId)
    gesture.current = {
      pointerId: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      trackIndex: props.tracks.findIndex((item) => item.id === track.id),
      moved: false,
      mode,
      selection,
      time: timeAt(event.clientX),
      tracks: props.tracks,
      delta: 0,
      shift: 0,
      pixelsPerSecond: viewport.pixelsPerSecond,
    }
  }
  const beginMarquee = (event: ReactPointerEvent<HTMLElement>, track: Track) => {
    if (event.button !== 0) return
    props.onSelectTrack(track.id)
    props.onSelectClip(null)
    viewport.seekAt(event.clientX)
    if (props.tool === 'select') begin(event, track, 'marquee', null)
  }
  const beginMove = (
    event: ReactPointerEvent<HTMLElement>,
    track: Track,
    selection: TimelineSelection,
  ) => {
    if (!canMove(props.tracks, selection.regions, 0)) return
    begin(event, track, 'move', selection)
  }
  const beginClipMove = (event: ReactPointerEvent<HTMLElement>, track: Track, clip: Clip) => {
    if (event.button !== 0) return
    event.stopPropagation()
    props.onSelectTrack(track.id)
    if (event.ctrlKey || event.metaKey) {
      const regions =
        props.selection?.regions ??
        props.tracks.flatMap((item) =>
          item.clips
            .filter((entry) => entry.id === props.selectedClipId)
            .map((entry) => wholeRegion(item, entry)),
        )
      const selected = regions.some((region) => region.clipId === clip.id)
      props.onSelectRegion(
        selectionFromRegions(
          props.tracks,
          selected
            ? regions.filter((region) => region.clipId !== clip.id)
            : [...regions, wholeRegion(track, clip)],
        ),
      )
      return
    }
    const current = props.selection?.regions.some((region) => region.clipId === clip.id)
      ? props.selection
      : null
    if (!current) props.onSelectClip(clip.id)
    const selected = current ?? selectionFromRegions(props.tracks, [wholeRegion(track, clip)])
    if (selected) beginMove(event, track, selected)
  }
  const captureMarquee = (event: ReactPointerEvent<HTMLElement>) => {
    if (!event.shiftKey || event.button !== 0 || props.tool !== 'select') return
    const lane = (event.target as Element).closest<HTMLElement>('[data-track-id]')
    const track = props.tracks.find((track) => track.id === lane?.dataset.trackId)
    if (track) beginMarquee(event, track)
  }
  const move = (event: ReactPointerEvent<HTMLElement>) => {
    const current = gesture.current
    if (!current || current.pointerId !== event.pointerId) return
    if (!current.moved && Math.hypot(event.clientX - current.x, event.clientY - current.y) < 3)
      return
    current.moved = true
    const index = indexAt(event.clientY)
    if (current.mode === 'marquee') {
      const time = timeAt(event.clientX)
      const trackIds = current.tracks
        .slice(Math.min(index, current.trackIndex), Math.max(index, current.trackIndex) + 1)
        .map((track) => track.id)
      props.onSelectRegion(
        selectInterval(
          current.tracks,
          trackIds,
          Math.min(current.time, time),
          Math.max(current.time, time),
        ),
      )
      return
    }
    const selected = current.selection!
    let delta = Math.max(-selected.start, (event.clientX - current.x) / current.pixelsPerSecond)
    const proposed = index - current.trackIndex
    const shift = canMove(current.tracks, selected.regions, proposed) ? proposed : current.shift
    const trackIds = selected.trackIds
      .map((id) => current.tracks[current.tracks.findIndex((track) => track.id === id) + shift]?.id)
      .filter((id): id is string => !!id)
    if (props.snapping)
      delta = snapDelta(
        delta,
        selected.regions.flatMap((region) => [region.from, region.to]),
        clipBoundaries(
          current.tracks,
          trackIds,
          selected.regions.map((region) => region.clipId),
        ),
        current.pixelsPerSecond,
        -selected.start,
      )
    current.delta = delta
    current.shift = shift
    setPreview({
      tracks: previewMove(current.tracks, selected.regions, delta, shift),
      selection: {
        ...selected,
        start: Math.max(0, selected.start + delta),
        end: selected.end + delta,
        trackIds,
        regions: selected.regions.map((region) => ({
          ...region,
          from: region.from + delta,
          to: region.to + delta,
          trackId:
            current.tracks[current.tracks.findIndex((track) => track.id === region.trackId) + shift]
              .id,
        })),
      },
    })
  }
  const finish = (event: ReactPointerEvent<HTMLElement>, cancelled = false) => {
    const current = gesture.current
    if (!current || current.pointerId !== event.pointerId) return
    gesture.current = null
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId)
    if (
      current.mode === 'move' &&
      current.moved &&
      !cancelled &&
      (current.delta !== 0 || current.shift !== 0)
    ) {
      pending.current = true
      void props
        .onMoveRegions(current.selection!.regions, current.delta, current.shift)
        .finally(() => {
          pending.current = false
          setPreview(null)
        })
    } else setPreview(null)
    if (cancelled && current.mode === 'marquee') props.onSelectRegion(null)
  }
  return { preview, beginMarquee, beginMove, beginClipMove, captureMarquee, move, finish }
}

export type TimelineSelecting = ReturnType<typeof useTimelineSelection>

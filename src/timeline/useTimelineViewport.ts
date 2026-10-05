import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from 'react'
import { clamp, thumbGeometry, dragViewport, zoomViewport, type ViewWindow } from './viewport'
import { shortcutFor } from '../shortcuts'
import type { TimelineProps } from './types'

export function useTimelineViewport({
  duration,
  position,
  recording,
  onSeek,
}: Pick<TimelineProps, 'duration' | 'position' | 'recording' | 'onSeek'>) {
  const [view, setView] = useState<ViewWindow>({ start: 0, span: 60 })
  const [laneWidth, setLaneWidth] = useState(800)
  const rulerRef = useRef<HTMLDivElement>(null)
  const timelineRef = useRef<HTMLElement>(null)
  const scrollbarRef = useRef<HTMLDivElement>(null)
  const panDrag = useRef<{
    pointerId: number
    x: number
    view: ViewWindow
    width: number
    extent: number
  } | null>(null)
  const [panning, setPanning] = useState(false)
  const scrollDrag = useRef<{
    x: number
    view: ViewWindow
    extent: number
    width: number
    mode: 'pan' | 'left' | 'right'
    thumbWidth: number
  } | null>(null)
  const extent = Math.max(
    60,
    duration > 0 ? duration + 5 : 0,
    recording ? position + 5 : 0,
    view.start + view.span,
  )
  const pixelsPerSecond = laneWidth / view.span
  useEffect(() => {
    const ruler = rulerRef.current
    if (!ruler) return
    const observer = new ResizeObserver((entries) => setLaneWidth(entries[0].contentRect.width))
    observer.observe(ruler)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    if (recording && position > view.start + view.span - 0.3) {
      setView((current) => ({ ...current, start: Math.max(0, position - current.span * 0.2) }))
    }
  }, [recording, position, view.start, view.span])

  const tickStep = useMemo(() => {
    const desired = view.span / Math.max(2, laneWidth / 82)
    return (
      [0.1, 0.2, 0.5, 1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 1800, 3600].find(
        (step) => step >= desired,
      ) ?? 3600
    )
  }, [view.span, laneWidth])
  const ticks = useMemo(() => {
    const values: number[] = []
    for (
      let index = Math.ceil(view.start / tickStep);
      index * tickStep < view.start + view.span;
      index++
    )
      values.push(Math.round(index * tickStep * 1000000) / 1000000)
    return values
  }, [view, tickStep])

  const seekAt = (clientX: number) => {
    if (recording || !rulerRef.current) return
    const x = clamp(clientX - rulerRef.current.getBoundingClientRect().left, 0, laneWidth)
    onSeek(Math.max(0, view.start + x / pixelsPerSecond))
  }

  const zoom = useCallback(
    (factor: number, pointerFraction?: number) => {
      setView((current) =>
        zoomViewport(current, factor, Math.max(3600, duration * 2), position, pointerFraction),
      )
    },
    [duration, position],
  )

  useEffect(() => {
    const timeline = timelineRef.current
    if (!timeline) return
    const blocked = (target: EventTarget | null) =>
      document.querySelector('[role="dialog"]') ||
      (target instanceof Element && target.closest('input,textarea,select,[contenteditable=true]'))
    const wheel = (event: WheelEvent) => {
      if (!event.ctrlKey || event.defaultPrevented || blocked(event.target) || event.deltaY === 0)
        return
      event.preventDefault()
      const ruler = rulerRef.current
      if (!ruler) return
      const bounds = ruler.getBoundingClientRect()
      zoom(
        event.deltaY < 0 ? 1 / 1.2 : 1.2,
        (event.clientX - bounds.left) / Math.max(1, bounds.width),
      )
    }
    const keydown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || blocked(event.target)) return
      const shortcut = shortcutFor(event)
      if (shortcut !== 'zoomIn' && shortcut !== 'zoomOut') return
      event.preventDefault()
      zoom(shortcut === 'zoomIn' ? 1 / 1.5 : 1.5)
    }
    timeline.addEventListener('wheel', wheel, { passive: false })
    window.addEventListener('keydown', keydown)
    return () => {
      timeline.removeEventListener('wheel', wheel)
      window.removeEventListener('keydown', keydown)
    }
  }, [zoom])

  const { width: thumbWidth, left: thumbLeft } = thumbGeometry(view, extent, laneWidth)
  const beginScroll = (event: ReactPointerEvent<HTMLElement>, mode: 'pan' | 'left' | 'right') => {
    if (event.button !== 0) return
    event.preventDefault()
    event.stopPropagation()
    event.currentTarget.setPointerCapture(event.pointerId)
    scrollDrag.current = { x: event.clientX, view, extent, width: laneWidth, mode, thumbWidth }
  }
  const moveScroll = (event: ReactPointerEvent<HTMLElement>) => {
    const current = scrollDrag.current
    if (!current) return
    setView(
      dragViewport(
        current.view,
        current.extent,
        current.width,
        current.mode,
        event.clientX - current.x,
      ),
    )
  }

  const endScroll = (event: ReactPointerEvent<HTMLElement>) => {
    scrollDrag.current = null
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId)
  }

  const beginPan = (event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 1 || !(event.target as Element).closest('.tl-lane')) return
    event.preventDefault()
    event.stopPropagation()
    event.currentTarget.setPointerCapture(event.pointerId)
    panDrag.current = {
      pointerId: event.pointerId,
      x: event.clientX,
      view,
      width: laneWidth,
      extent,
    }
    setPanning(true)
  }
  const movePan = (event: ReactPointerEvent<HTMLElement>) => {
    const current = panDrag.current
    if (!current || current.pointerId !== event.pointerId) return
    event.preventDefault()
    event.stopPropagation()
    setView({
      ...current.view,
      start: clamp(
        current.view.start - ((event.clientX - current.x) * current.view.span) / current.width,
        0,
        Math.max(0, current.extent - current.view.span),
      ),
    })
  }
  const endPan = (event: ReactPointerEvent<HTMLElement>) => {
    if (panDrag.current?.pointerId !== event.pointerId) return
    panDrag.current = null
    setPanning(false)
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId)
  }

  return {
    view,
    setView,
    laneWidth,
    rulerRef,
    timelineRef,
    scrollbarRef,
    extent,
    pixelsPerSecond,
    tickStep,
    ticks,
    seekAt,
    zoom,
    thumbWidth,
    thumbLeft,
    beginScroll,
    moveScroll,
    endScroll,
    panning,
    beginPan,
    movePan,
    endPan,
  }
}
export type TimelineViewport = ReturnType<typeof useTimelineViewport>

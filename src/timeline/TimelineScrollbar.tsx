import { t } from '../i18n'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { formatTime } from '../types'
import { clamp, minimumSpan } from './viewport'
import type { TimelineProps } from './types'
import type { TimelineViewport } from './useTimelineViewport'
type Props = Pick<TimelineProps, 'recording' | 'tool'> & { viewport: TimelineViewport }
export default function TimelineScrollbar({ recording, tool, viewport }: Props) {
  const {
    scrollbarRef,
    setView,
    laneWidth,
    extent,
    view,
    thumbLeft,
    thumbWidth,
    beginScroll,
    moveScroll,
    endScroll,
  } = viewport
  return (
    <div className="tl-bottom-row">
      <div className="tl-timeline-hint">
        {recording
          ? t('timeline.recording')
          : tool === 'split'
            ? t('timeline.splitHint')
            : t('timeline.moveHint')}
      </div>
      <div
        ref={scrollbarRef}
        className="tl-scroll-track"
        onPointerDown={(event) => {
          if (event.target !== event.currentTarget) return
          const x = event.clientX - event.currentTarget.getBoundingClientRect().left
          setView((current) => ({
            ...current,
            start: clamp((x / laneWidth) * extent - current.span / 2, 0, extent - current.span),
          }))
        }}
      >
        <div
          className="tl-scroll-thumb"
          role="slider"
          aria-label={t('timeline.viewport')}
          aria-valuemin={0}
          aria-valuemax={Math.max(0, extent - view.span)}
          aria-valuenow={view.start}
          aria-valuetext={`${formatTime(view.start)} — ${formatTime(view.start + view.span)}`}
          tabIndex={0}
          style={{ left: thumbLeft, width: thumbWidth }}
          onPointerDown={(event) => beginScroll(event, 'pan')}
          onPointerMove={moveScroll}
          onPointerUp={endScroll}
          onPointerCancel={endScroll}
          onKeyDown={(event) => {
            if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
              event.preventDefault()
              setView((current) => ({
                ...current,
                start: clamp(
                  current.start + ((event.key === 'ArrowRight' ? 1 : -1) * current.span) / 10,
                  0,
                  extent - current.span,
                ),
              }))
            }
          }}
        >
          <button
            className="tl-scroll-handle tl-scroll-handle-left"
            title={t('zoom.dragEdge')}
            aria-label={t('zoom.leftEdge')}
            onPointerDown={(event) => beginScroll(event, 'left')}
            onPointerMove={moveScroll}
            onPointerUp={endScroll}
            onPointerCancel={endScroll}
            onKeyDown={(event) => {
              if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
                event.preventDefault()
                event.stopPropagation()
                const start = clamp(
                  view.start + ((event.key === 'ArrowRight' ? 1 : -1) * view.span) / 10,
                  0,
                  view.start + view.span - minimumSpan,
                )
                setView({ start, span: view.start + view.span - start })
              }
            }}
          >
            <ChevronLeft size={9} />
          </button>
          <span className="tl-scroll-grip" />
          <button
            className="tl-scroll-handle tl-scroll-handle-right"
            title={t('zoom.dragEdge')}
            aria-label={t('zoom.rightEdge')}
            onPointerDown={(event) => beginScroll(event, 'right')}
            onPointerMove={moveScroll}
            onPointerUp={endScroll}
            onPointerCancel={endScroll}
            onKeyDown={(event) => {
              if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
                event.preventDefault()
                event.stopPropagation()
                setView({
                  ...view,
                  span: clamp(
                    view.span * (event.key === 'ArrowRight' ? 1.1 : 0.9),
                    minimumSpan,
                    extent - view.start,
                  ),
                })
              }
            }}
          >
            <ChevronRight size={9} />
          </button>
        </div>
      </div>
    </div>
  )
}

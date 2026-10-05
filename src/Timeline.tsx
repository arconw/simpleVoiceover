import { t } from './i18n'
import { useMemo } from 'react'
import type { TimelineProps } from './timeline/types'
import { useTimelineViewport } from './timeline/useTimelineViewport'
import { useClipDrag } from './timeline/useClipDrag'
import TimelineToolbar from './timeline/TimelineToolbar'
import TimelineRuler from './timeline/TimelineRuler'
import TimelineTrack from './timeline/TimelineTrack'
import TimelineScrollbar from './timeline/TimelineScrollbar'
import { useTimelineSelection } from './timeline/useTimelineSelection'
import './timeline.css'

export default function Timeline(props: TimelineProps) {
  const { tracks, assets, selectedClipId, recording, position, tool } = props
  const viewport = useTimelineViewport(props)
  const assetMap = useMemo(() => new Map(assets.map((asset) => [asset.id, asset])), [assets])
  const clipDragging = useClipDrag(props, viewport, assetMap)
  const selecting = useTimelineSelection(props, viewport)
  const selectedTrack = tracks.find((track) =>
    track.clips.some((clip) => clip.id === selectedClipId),
  )
  const canDelete =
    !recording &&
    !props.operationPending &&
    (props.selection?.regions.length
      ? props.selection.regions.every(
          (region) => !tracks.find((track) => track.id === region.trackId)?.locked,
        )
      : !!selectedClipId && !!selectedTrack && !selectedTrack.locked)
  const playheadLeft = (position - viewport.view.start) * viewport.pixelsPerSecond
  const playheadVisible = playheadLeft >= 0 && playheadLeft <= viewport.laneWidth
  return (
    <section
      ref={viewport.timelineRef}
      className={`tl-timeline ${tool === 'split' ? 'tl-split-mode' : ''} ${viewport.panning ? 'is-panning' : ''}`}
      onPointerDownCapture={(event) => {
        viewport.beginPan(event)
        if (event.button === 0) selecting.captureMarquee(event)
      }}
      onPointerMoveCapture={viewport.movePan}
      onPointerUpCapture={viewport.endPan}
      onPointerCancelCapture={viewport.endPan}
      onPointerMove={selecting.move}
      onPointerUp={(event) => selecting.finish(event)}
      onPointerCancel={(event) => selecting.finish(event, true)}
      onLostPointerCapture={(event) => {
        viewport.endPan(event)
        selecting.finish(event, true)
      }}
      onAuxClick={(event) => {
        if (event.button === 1) event.preventDefault()
      }}
      aria-label={t('timeline.editor')}
    >
      <TimelineToolbar
        {...props}
        viewport={viewport}
        canDelete={canDelete}
        selectedTrack={selectedTrack}
      />
      <div className="tl-track-area">
        <TimelineRuler {...props} viewport={viewport} />
        {(selecting.preview?.tracks ?? tracks).map((track, index) => (
          <TimelineTrack
            key={track.id}
            {...props}
            track={track}
            index={index}
            viewport={viewport}
            clipDragging={clipDragging}
            selecting={selecting}
            selection={selecting.preview?.selection ?? props.selection}
            assetMap={assetMap}
          />
        ))}
        {playheadVisible && (
          <div
            className={`tl-playhead ${recording ? 'is-recording' : ''}`}
            style={{ left: `calc(var(--tl-header-width) + ${playheadLeft}px)` }}
          >
            <span />
          </div>
        )}
      </div>
      <TimelineScrollbar {...props} viewport={viewport} />
    </section>
  )
}

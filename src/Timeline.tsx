import { useMemo } from 'react'
import type { TimelineProps } from './timeline/types'
import { useTimelineViewport } from './timeline/useTimelineViewport'
import { useClipDrag } from './timeline/useClipDrag'
import TimelineToolbar from './timeline/TimelineToolbar'
import TimelineRuler from './timeline/TimelineRuler'
import TimelineTrack from './timeline/TimelineTrack'
import TimelineScrollbar from './timeline/TimelineScrollbar'
import './timeline.css'

export default function Timeline(props: TimelineProps) {
  const { tracks, assets, selectedClipId, recording, position, tool } = props
  const viewport = useTimelineViewport(props)
  const assetMap = useMemo(() => new Map(assets.map((asset) => [asset.id, asset])), [assets])
  const clipDragging = useClipDrag(props, viewport, assetMap)
  const selectedTrack = tracks.find((track) =>
    track.clips.some((clip) => clip.id === selectedClipId),
  )
  const canDelete = !!selectedClipId && !!selectedTrack && !selectedTrack.locked && !recording
  const playheadLeft = (position - viewport.view.start) * viewport.pixelsPerSecond
  const playheadVisible = playheadLeft >= 0 && playheadLeft <= viewport.laneWidth
  return (
    <section
      className={`tl-timeline ${tool === 'split' ? 'tl-split-mode' : ''}`}
      aria-label="Монтаж звуковых дорожек"
    >
      <TimelineToolbar
        {...props}
        viewport={viewport}
        canDelete={canDelete}
        selectedTrack={selectedTrack}
      />
      <div className="tl-track-area">
        <TimelineRuler {...props} viewport={viewport} />
        {tracks.map((track, index) => (
          <TimelineTrack
            key={track.id}
            {...props}
            track={track}
            index={index}
            viewport={viewport}
            clipDragging={clipDragging}
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

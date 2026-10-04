import type { CSSProperties } from 'react'
import { LockKeyhole } from 'lucide-react'
import { formatTime, type Track, type MediaAsset } from '../types'
import type { TimelineProps } from './types'
import type { TimelineViewport } from './useTimelineViewport'
import type { ClipDragging } from './useClipDrag'
import TrackHeader from './TrackHeader'
import Waveform from './Waveform'
import { clamp } from './viewport'
type Props = Pick<
  TimelineProps,
  | 'selectedTrackId'
  | 'selectedClipId'
  | 'recording'
  | 'onSelectTrack'
  | 'onSelectClip'
  | 'onUpdateTrack'
  | 'getLevel'
  | 'position'
  | 'recordStart'
> & {
  track: Track
  index: number
  viewport: TimelineViewport
  clipDragging: ClipDragging
  assetMap: Map<string, MediaAsset>
}
export default function TimelineTrack({
  selectedTrackId,
  selectedClipId,
  recording,
  onSelectTrack,
  onSelectClip,
  onUpdateTrack,
  getLevel,
  position,
  recordStart,
  track,
  index,
  viewport,
  clipDragging,
  assetMap,
}: Props) {
  const { ticks, view, pixelsPerSecond, laneWidth, seekAt } = viewport
  const { preview, drag, beginClipDrag, moveClip, finishClip } = clipDragging
  return (
    <div
      key={track.id}
      className={`tl-track-row ${track.id === selectedTrackId ? 'is-selected' : ''}`}
      style={{ '--tl-track-color': track.color } as CSSProperties}
    >
      <TrackHeader
        track={track}
        index={index}
        recording={recording}
        onSelectTrack={onSelectTrack}
        onUpdateTrack={onUpdateTrack}
        getLevel={getLevel}
      />
      <div
        className={`tl-lane ${track.locked ? 'is-locked' : ''}`}
        onPointerDown={(event) => {
          if (event.button === 0) {
            onSelectTrack(track.id)
            onSelectClip(null)
            seekAt(event.clientX)
          }
        }}
      >
        {ticks.map((tick) => (
          <span
            key={tick}
            className="tl-gridline"
            style={{ left: (tick - view.start) * pixelsPerSecond }}
          />
        ))}
        {track.clips.length === 0 && !(recording && track.armed) && (
          <span className="tl-empty-lane">
            {track.kind === 'video'
              ? 'Звуковая дорожка появится после импорта видео'
              : track.armed
                ? 'Дорожка готова к записи микрофона'
                : 'Добавьте аудио или выберите R для записи'}
          </span>
        )}
        {track.clips.map((original) => {
          const clip =
            preview?.trackId === track.id && preview.clip.id === original.id
              ? preview.clip
              : original
          const left = (clip.start - view.start) * pixelsPerSecond
          const right = left + clip.duration * pixelsPerSecond
          if ((right <= 0 || left >= laneWidth) && drag.current?.clip.id !== clip.id) return null
          const visibleLeft = clamp(left, 0, laneWidth)
          const visibleRight = clamp(right, 0, laneWidth)
          const visibleWidth = Math.max(1, visibleRight - visibleLeft)
          const crop = (visibleLeft - left) / pixelsPerSecond
          const asset = assetMap.get(clip.assetId)
          return (
            <div
              key={clip.id}
              className={`tl-clip ${selectedClipId === clip.id ? 'is-selected' : ''} ${track.mute ? 'is-muted' : ''} ${preview?.clip.id === clip.id ? 'is-dragging' : ''}`}
              style={{ left: visibleLeft, width: visibleWidth }}
              tabIndex={0}
              role="button"
              aria-label={`${clip.name}, с ${formatTime(clip.start, true)}, длительность ${formatTime(clip.duration, true)}`}
              title={`${clip.name} · ${formatTime(clip.start, true)} — ${formatTime(clip.start + clip.duration, true)}`}
              onPointerDown={(event) => beginClipDrag(event, track, original)}
              onPointerMove={moveClip}
              onPointerUp={(event) => finishClip(event)}
              onPointerCancel={(event) => finishClip(event, true)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault()
                  event.stopPropagation()
                  onSelectTrack(track.id)
                  onSelectClip(clip.id)
                }
              }}
            >
              <div className="tl-clip-heading">
                <span>{clip.name}</span>
                {track.locked && <LockKeyhole size={9} />}
              </div>
              <Waveform
                asset={asset}
                offset={clip.offset + crop}
                duration={visibleWidth / pixelsPerSecond}
                width={visibleWidth}
                color={track.color}
              />
              {!track.locked && !recording && left >= 0 && (
                <span
                  className="tl-trim tl-trim-left"
                  data-trim="left"
                  title="Обрезать начало клипа"
                />
              )}
              {!track.locked && !recording && right <= laneWidth && (
                <span
                  className="tl-trim tl-trim-right"
                  data-trim="right"
                  title="Обрезать конец клипа"
                />
              )}
            </div>
          )
        })}
        {recording &&
          track.armed &&
          position >= view.start &&
          recordStart <= view.start + view.span && (
            <div
              className="tl-recording-clip"
              style={{
                left: Math.max(0, (recordStart - view.start) * pixelsPerSecond),
                width: Math.max(
                  2,
                  (Math.min(position, view.start + view.span) - Math.max(view.start, recordStart)) *
                    pixelsPerSecond,
                ),
              }}
            >
              <span>● Запись</span>
            </div>
          )}
      </div>
    </div>
  )
}

import { localizedName, t } from '../i18n'
import { soloSources } from '../trackState'
import type { CSSProperties } from 'react'
import { LockKeyhole } from 'lucide-react'
import { formatTime, type Track, type MediaAsset } from '../types'
import type { TimelineProps } from './types'
import type { TimelineViewport } from './useTimelineViewport'
import type { ClipDragging } from './useClipDrag'
import TrackHeader from './TrackHeader'
import Waveform from './Waveform'
import { clamp } from './viewport'
import type { TimelineSelecting } from './useTimelineSelection'
type Props = Pick<
  TimelineProps,
  | 'selectedTrackId'
  | 'selectedClipId'
  | 'recording'
  | 'operationPending'
  | 'tracks'
  | 'onRemoveTrack'
  | 'onSelectTrack'
  | 'onSelectClip'
  | 'onUpdateTrack'
  | 'getLevel'
  | 'position'
  | 'recordStart'
  | 'selection'
  | 'tool'
> & {
  track: Track
  index: number
  viewport: TimelineViewport
  clipDragging: ClipDragging
  selecting: TimelineSelecting
  assetMap: Map<string, MediaAsset>
}
export default function TimelineTrack({
  selectedTrackId,
  selectedClipId,
  recording,
  operationPending,
  tracks,
  onRemoveTrack,
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
  selecting,
  selection,
  tool: propsTool,
}: Props) {
  const { ticks, view, pixelsPerSecond, laneWidth } = viewport
  const { preview, drag, beginClipDrag, moveClip, finishClip } = clipDragging
  const soloTracks = soloSources(track, tracks)
  const soloHint = soloTracks.length
    ? t('track.silencedBySolo', {
        names: soloTracks.map((entry) => localizedName(entry.name)).join(', '),
      })
    : undefined
  return (
    <div
      key={track.id}
      className={`tl-track-row ${track.id === selectedTrackId ? 'is-selected' : ''} ${soloHint ? 'is-silenced' : ''}`}
      title={soloHint}
      style={{ '--tl-track-color': track.color } as CSSProperties}
    >
      <TrackHeader
        track={track}
        index={index}
        recording={recording}
        operationPending={operationPending}
        onRemoveTrack={onRemoveTrack}
        soloHint={soloHint}
        onSelectTrack={onSelectTrack}
        onUpdateTrack={onUpdateTrack}
        getLevel={getLevel}
      />
      <div
        data-track-id={track.id}
        className={`tl-lane ${track.locked ? 'is-locked' : ''}`}
        onPointerDown={(event) => {
          if (event.button === 0) {
            selecting.beginMarquee(event, track)
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
              ? t('timeline.emptyVideo')
              : track.armed
                ? t('timeline.emptyArmed')
                : t('timeline.emptyAudio')}
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
              className={`tl-clip ${selectedClipId === clip.id || selection?.regions.some((region) => region.clipId === clip.id) ? 'is-selected' : ''} ${track.mute || soloHint ? 'is-muted' : ''} ${preview?.clip.id === clip.id || selecting.preview?.selection.regions.some((region) => region.clipId === clip.id) ? 'is-dragging' : ''}`}
              style={{ left: visibleLeft, width: visibleWidth }}
              tabIndex={0}
              role="button"
              aria-label={t('clip.description', {
                value0: clip.name,
                value1: formatTime(clip.start, true),
                value2: formatTime(clip.duration, true),
              })}
              title={`${soloHint ? `${soloHint}\n` : ''}${clip.name} · ${formatTime(clip.start, true)} — ${formatTime(clip.start + clip.duration, true)}`}
              onPointerDown={(event) => {
                if ((event.target as Element).closest('[data-trim]') || propsTool === 'split')
                  beginClipDrag(event, track, original)
                else selecting.beginClipMove(event, track, original)
              }}
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
                  title={t('clip.trimStart')}
                />
              )}
              {!track.locked && !recording && right <= laneWidth && (
                <span
                  className="tl-trim tl-trim-right"
                  data-trim="right"
                  title={t('clip.trimEnd')}
                />
              )}
            </div>
          )
        })}
        {selection?.trackIds.includes(track.id) && selection.end > selection.start && (
          <div
            className="tl-range-selection"
            style={{
              left: clamp((selection.start - view.start) * pixelsPerSecond, 0, laneWidth),
              width: Math.max(
                0,
                clamp((selection.end - view.start) * pixelsPerSecond, 0, laneWidth) -
                  clamp((selection.start - view.start) * pixelsPerSecond, 0, laneWidth),
              ),
            }}
          >
            {selection.regions.length > 0 && !track.locked && (
              <div
                className="tl-selection-grip"
                title={t('selection.move')}
                onPointerDown={(event) => selecting.beginMove(event, track, selection)}
              />
            )}
          </div>
        )}
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
              <span>{t('clip.recording')}</span>
            </div>
          )}
      </div>
    </div>
  )
}

import { localizedName, t } from '../i18n'
import {
  AudioLines,
  LockKeyhole,
  Mic,
  Trash2,
  UnlockKeyhole,
  Video,
  WandSparkles,
} from 'lucide-react'
import type { Track } from '../types'
import TrackMeter from './TrackMeter'

interface Props {
  track: Track
  index: number
  recording: boolean
  operationPending: boolean
  onRemoveTrack: (track: Track) => void
  soloHint?: string
  onSelectTrack: (id: string) => void
  onUpdateTrack: (id: string, patch: Partial<Track>) => void
  getLevel: (id: string) => number
}
export default function TrackHeader({
  track,
  index,
  recording,
  operationPending,
  onRemoveTrack,
  soloHint,
  onSelectTrack,
  onUpdateTrack,
  getLevel,
}: Props) {
  return (
    <div className="tl-track-header" onClick={() => onSelectTrack(track.id)}>
      <div className="tl-track-heading">
        <span className="tl-track-number">{(index + 1).toString().padStart(2, '0')}</span>
        {track.kind === 'video' ? (
          <Video size={13} />
        ) : track.kind === 'voice' ? (
          <Mic size={13} />
        ) : (
          <AudioLines size={13} />
        )}
        <button
          className="tl-track-name"
          onClick={() => onSelectTrack(track.id)}
          title={soloHint ?? localizedName(track.name)}
        >
          {localizedName(track.name)}
        </button>
        <span className="tl-track-db">
          {track.volume > 0 ? '+' : ''}
          {track.volume.toFixed(1)}
        </span>
      </div>
      <div className="tl-track-bottom">
        <div className="tl-circles">
          <button
            className={`tl-circle ${track.mute ? 'is-muted' : ''}`}
            title={t('track.mute')}
            aria-label={t('track.muteLabel', { value0: localizedName(track.name) })}
            aria-pressed={track.mute}
            onClick={() => onUpdateTrack(track.id, { mute: !track.mute })}
          >
            M
          </button>
          <button
            className={`tl-circle ${track.solo ? 'is-solo' : ''}`}
            title={t('track.solo')}
            aria-label={t('track.soloLabel', { value0: localizedName(track.name) })}
            aria-pressed={track.solo}
            onClick={() => onUpdateTrack(track.id, { solo: !track.solo })}
          >
            S
          </button>
          <button
            className={`tl-circle ${track.armed ? 'is-armed' : ''}`}
            title={track.kind === 'video' ? t('track.selectAudio') : t('track.recordHere')}
            aria-label={t('track.armLabel', { value0: localizedName(track.name) })}
            aria-pressed={track.armed}
            disabled={track.kind === 'video' || recording}
            onClick={() => onUpdateTrack(track.id, { armed: !track.armed })}
          >
            R
          </button>
          <button
            className={`tl-circle ${track.locked ? 'is-active' : ''}`}
            title={track.locked ? t('track.unlock') : t('track.lock')}
            aria-label={t('track.lockLabel', { value0: localizedName(track.name) })}
            aria-pressed={track.locked}
            onClick={() => onUpdateTrack(track.id, { locked: !track.locked })}
          >
            {track.locked ? <LockKeyhole size={9} /> : <UnlockKeyhole size={9} />}
          </button>
          <button
            className={`tl-circle ${!track.fxBypass ? 'is-fx' : ''}`}
            title={track.fxBypass ? t('track.enableFx') : t('track.bypassFx')}
            aria-label={t('track.fxLabel', { value0: localizedName(track.name) })}
            aria-pressed={!track.fxBypass}
            onClick={() => onUpdateTrack(track.id, { fxBypass: !track.fxBypass })}
          >
            <WandSparkles size={9} />
          </button>
          {Array.from({ length: 7 }, (_, slot) => (
            <button
              key={slot}
              className="tl-circle tl-reserved"
              disabled
              title={t('track.reserved')}
              aria-label={t('track.reserved')}
            />
          ))}
        </div>
        <div className="tl-track-level">
          <TrackMeter id={track.id} getLevel={getLevel} />
          <div className="tl-track-footer">
            <span>
              {track.locked
                ? t('track.locked')
                : track.armed
                  ? t('track.microphone')
                  : track.kind === 'video'
                    ? t('track.videoAudio')
                    : t('track.audioTrack')}
            </span>
            <button
              className="tl-track-delete"
              title={t('track.delete', { name: localizedName(track.name) })}
              aria-label={t('track.delete', { name: localizedName(track.name) })}
              disabled={recording || operationPending || track.locked}
              onClick={(event) => {
                event.stopPropagation()
                onRemoveTrack(track)
              }}
            >
              <Trash2 size={11} />
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

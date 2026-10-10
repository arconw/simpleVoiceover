import { localizedName, t } from '../i18n'
import { Headphones, Mic, Trash2, Volume2 } from 'lucide-react'
import { soloSources } from '../trackState'
import type { StudioController } from '../useStudio'
import { Slider } from '../EffectsPanel'
type Props = Pick<
  StudioController,
  | 'tracks'
  | 'selectedTrackId'
  | 'setSelectedTrackId'
  | 'recording'
  | 'monitor'
  | 'setMonitor'
  | 'busy'
  | 'updateTrack'
  | 'exportAudio'
  | 'requestRemoveTrack'
  | 'operationPending'
>

export default function MixerPanel({
  tracks,
  selectedTrackId,
  setSelectedTrackId,
  recording,
  monitor,
  setMonitor,
  busy,
  updateTrack,
  exportAudio,
  requestRemoveTrack,
  operationPending,
}: Props) {
  return (
    <div className="mixer-panel">
      <div className="panel-heading">
        <span className="eyebrow">{t('mixer.heading')}</span>
        <Volume2 size={14} />
      </div>
      {tracks.map((track) => {
        const soloTracks = soloSources(track, tracks)
        const soloHint = soloTracks.length
          ? t('track.silencedBySolo', {
              names: soloTracks.map((entry) => localizedName(entry.name)).join(', '),
            })
          : undefined
        return (
          <div
            className={`mixer-strip ${track.id === selectedTrackId ? 'selected' : ''} ${soloHint ? 'is-silenced' : ''}`}
            title={soloHint}
            key={track.id}
            onClick={() => setSelectedTrackId(track.id)}
          >
            <div className="mixer-title">
              <i style={{ background: track.color }} />
              <strong>{localizedName(track.name)}</strong>
              <button
                className={track.mute ? 'active-mute' : ''}
                aria-label={t('mixer.muteLabel', { value0: localizedName(track.name) })}
                aria-pressed={track.mute}
                onClick={() => updateTrack(track.id, { mute: !track.mute })}
              >
                M
              </button>
              <button
                className={track.solo ? 'active-solo' : ''}
                aria-label={t('mixer.soloLabel', { value0: localizedName(track.name) })}
                aria-pressed={track.solo}
                onClick={() => updateTrack(track.id, { solo: !track.solo })}
              >
                S
              </button>
            </div>
            <Slider
              label={t('mixer.volumeLabel', { value0: localizedName(track.name) })}
              help="volume"
              value={track.volume}
              min={-60}
              max={12}
              step={0.5}
              unit={t('units.db')}
              onChange={(v) => updateTrack(track.id, { volume: v })}
            />
            <Slider
              label={t('mixer.panLabel', { value0: localizedName(track.name) })}
              help="pan"
              value={track.pan}
              min={-1}
              max={1}
              step={0.05}
              onChange={(v) => updateTrack(track.id, { pan: v })}
            />
            <div className="pan-labels">
              <span>L</span>
              <span>C</span>
              <span>R</span>
            </div>
            <div className="track-exports">
              <span>{t('mixer.exportTrack')}</span>
              <button
                disabled={!track.clips.length || recording || !!busy}
                onClick={() => void exportAudio(track.id)}
              >
                WAV
              </button>
              <button
                disabled={!track.clips.length || recording || !!busy}
                onClick={() => void exportAudio(track.id, 'mp3')}
              >
                MP3
              </button>
              <button
                className="track-delete"
                title={t('track.delete', { name: localizedName(track.name) })}
                aria-label={t('track.delete', { name: localizedName(track.name) })}
                disabled={recording || operationPending || track.locked}
                onClick={(event) => {
                  event.stopPropagation()
                  requestRemoveTrack(track)
                }}
              >
                <Trash2 size={13} />
              </button>
            </div>
          </div>
        )
      })}
      <div className="mic-settings">
        <h3>
          <Mic size={15} /> {t('mixer.input')}{' '}
        </h3>
        <button
          className={`monitor-toggle ${monitor ? 'enabled' : ''}`}
          aria-pressed={monitor}
          onClick={() => {
            setMonitor(!monitor)
          }}
        >
          <Headphones size={16} />
          {t('mixer.monitor')}
          <span>{monitor ? t('common.on') : t('common.off')}</span>
        </button>
        <p className="hint muted">{t('mixer.monitorHint')} </p>
      </div>
    </div>
  )
}

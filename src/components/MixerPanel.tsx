import { localizedName, t } from '../i18n'
import { Headphones, Mic, Volume2 } from 'lucide-react'
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
  | 'devices'
  | 'deviceId'
  | 'setDeviceId'
  | 'busy'
  | 'updateTrack'
  | 'exportAudio'
>

export default function MixerPanel({
  tracks,
  selectedTrackId,
  setSelectedTrackId,
  recording,
  monitor,
  setMonitor,
  devices,
  deviceId,
  setDeviceId,
  busy,
  updateTrack,
  exportAudio,
}: Props) {
  return (
    <div className="mixer-panel">
      <div className="panel-heading">
        <span className="eyebrow">{t('mixer.heading')}</span>
        <Volume2 size={14} />
      </div>
      {tracks.map((track) => (
        <div
          className={`mixer-strip ${track.id === selectedTrackId ? 'selected' : ''}`}
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
          </div>
        </div>
      ))}
      <div className="mic-settings">
        <h3>
          <Mic size={15} /> {t('mixer.input')}{' '}
        </h3>
        <select
          aria-label={t('mixer.device')}
          value={deviceId}
          disabled={recording}
          onChange={(e) => setDeviceId(e.target.value)}
        >
          <option value="">{t('mixer.defaultDevice')}</option>
          {devices
            .filter((d) => d.deviceId !== 'default')
            .map((d) => (
              <option key={d.deviceId} value={d.deviceId}>
                {d.label || t('mixer.microphone')}
              </option>
            ))}
        </select>
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

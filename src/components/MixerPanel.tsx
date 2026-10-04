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
        <span className="eyebrow">ЗВУК ДОРОЖЕК</span>
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
            <strong>{track.name}</strong>
            <button
              className={track.mute ? 'active-mute' : ''}
              aria-label={`Мут ${track.name}`}
              aria-pressed={track.mute}
              onClick={() => updateTrack(track.id, { mute: !track.mute })}
            >
              M
            </button>
            <button
              className={track.solo ? 'active-solo' : ''}
              aria-label={`Соло ${track.name}`}
              aria-pressed={track.solo}
              onClick={() => updateTrack(track.id, { solo: !track.solo })}
            >
              S
            </button>
          </div>
          <Slider
            label={`Громкость · ${track.name}`}
            value={track.volume}
            min={-60}
            max={12}
            step={0.5}
            unit="дБ"
            onChange={(v) => updateTrack(track.id, { volume: v })}
          />
          <Slider
            label={`Панорама · ${track.name}`}
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
            <span>Экспорт дорожки</span>
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
          <Mic size={15} /> Вход микрофона
        </h3>
        <select
          aria-label="Устройство микрофона"
          value={deviceId}
          disabled={recording}
          onChange={(e) => setDeviceId(e.target.value)}
        >
          <option value="">Микрофон по умолчанию</option>
          {devices
            .filter((d) => d.deviceId !== 'default')
            .map((d) => (
              <option key={d.deviceId} value={d.deviceId}>
                {d.label || 'Микрофон'}
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
          Слышать свой голос<span>{monitor ? 'Вкл' : 'Выкл'}</span>
        </button>
        <p className="hint muted">
          Для мониторинга используй наушники, чтобы звук не возвращался в микрофон. Список устройств
          появится после первого разрешения на запись.
        </p>
      </div>
    </div>
  )
}

import { Headphones, Mic, Pause, Play, SkipBack, Square } from 'lucide-react'
import { formatTime } from '../types'
import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  | 'position'
  | 'playing'
  | 'recording'
  | 'monitor'
  | 'setMonitor'
  | 'busy'
  | 'duration'
  | 'armedTrack'
  | 'pause'
  | 'seek'
  | 'play'
  | 'record'
  | 'inputLevel'
>

export default function Transport({
  position,
  playing,
  recording,
  monitor,
  setMonitor,
  busy,
  duration,
  armedTrack,
  pause,
  seek,
  play,
  record,
  inputLevel,
}: Props) {
  return (
    <section className="transport" aria-label="Воспроизведение и запись">
      <div className="transport-controls">
        <button
          className="icon-button"
          title="В начало · Home"
          aria-label="В начало"
          disabled={recording || !!busy}
          onClick={() => {
            void pause()
            seek(0)
          }}
        >
          <SkipBack size={14} />
        </button>
        <button
          className="play-button"
          title="Воспроизведение / пауза · Space"
          aria-label={playing ? 'Пауза' : 'Воспроизвести'}
          disabled={recording || !!busy}
          onClick={() => void play()}
        >
          {playing ? (
            <Pause size={15} fill="currentColor" />
          ) : (
            <Play size={15} fill="currentColor" />
          )}
        </button>
        <button
          className="icon-button"
          title="Стоп"
          aria-label="Стоп"
          disabled={!!busy}
          onClick={() => (recording ? void record() : void pause())}
        >
          <Square size={12} fill="currentColor" />
        </button>
        <button
          className={`record-button ${recording ? 'recording' : ''}`}
          disabled={!!busy}
          onClick={() => void record()}
        >
          <span className="record-circle" />
          {recording ? 'Стоп записи' : 'Запись'}
          <kbd>R</kbd>
        </button>
        <button
          className={`icon-button monitor-button ${monitor ? 'enabled' : ''}`}
          aria-label="Мониторинг микрофона"
          aria-pressed={monitor}
          title="Слышать микрофон в наушниках"
          onClick={() => setMonitor(!monitor)}
        >
          <Headphones size={15} />
        </button>
      </div>
      <div className="transport-readout">
        <span className="time-display">
          {formatTime(position, true)}
          <span>/ {formatTime(duration, true)}</span>
        </span>
        <span className="record-target" title={armedTrack?.name ?? 'Выбери дорожку R'}>
          <Mic size={13} />
          <span className="mini-meter" aria-label="Уровень микрофона">
            <i style={{ width: `${Math.min(100, inputLevel * 180)}%` }} />
          </span>
        </span>
      </div>
    </section>
  )
}

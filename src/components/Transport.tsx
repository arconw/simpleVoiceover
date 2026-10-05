import { localizedName, t } from '../i18n'
import { Headphones, Mic, Pause, Play, SkipBack, Square } from 'lucide-react'
import { formatTime } from '../types'
import type { StudioController } from '../useStudio'
import type { ReactNode } from 'react'

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
> & { preview: ReactNode }

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
  preview,
}: Props) {
  return (
    <section className="transport" aria-label={t('transport.controls')}>
      <div className="transport-controls">
        <button
          className="icon-button"
          title={t('transport.homeTitle')}
          aria-label={t('transport.home')}
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
          title={t('transport.playTitle')}
          aria-label={playing ? t('transport.pause') : t('transport.play')}
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
          title={t('transport.stop')}
          aria-label={t('transport.stop')}
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
          {recording ? t('transport.stopRecording') : t('transport.record')}
          <kbd>R</kbd>
        </button>
        <button
          className={`icon-button monitor-button ${monitor ? 'enabled' : ''}`}
          aria-label={t('transport.monitor')}
          aria-pressed={monitor}
          title={t('transport.monitorTitle')}
          onClick={() => setMonitor(!monitor)}
        >
          <Headphones size={15} />
        </button>
      </div>
      {preview}
      <div className="transport-readout">
        <span className="time-display">
          {formatTime(position, true)}
          <span>/ {formatTime(duration, true)}</span>
        </span>
        <span
          className="record-target"
          title={armedTrack ? localizedName(armedTrack.name) : t('transport.armHint')}
        >
          <Mic size={13} />
          <span className="mini-meter" aria-label={t('transport.inputLevel')}>
            <i style={{ width: `${Math.min(100, inputLevel * 180)}%` }} />
          </span>
        </span>
      </div>
    </section>
  )
}

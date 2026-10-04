import { AudioLines, HelpCircle, Monitor, Plus } from 'lucide-react'
import { formatTime } from '../types'
import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  | 'position'
  | 'recording'
  | 'recordStart'
  | 'busy'
  | 'setHelp'
  | 'importNative'
  | 'videoRef'
  | 'previewPosition'
  | 'videoClip'
  | 'videoAsset'
  | 'videoFiles'
>

export default function VideoPreview({
  position,
  recording,
  recordStart,
  busy,
  setHelp,
  importNative,
  videoRef,
  previewPosition,
  videoClip,
  videoAsset,
  videoFiles,
}: Props) {
  return (
    <section className="preview-panel">
      <div className="preview-heading">
        <span>
          <Monitor size={14} /> ПРЕДПРОСМОТР
        </span>
        <small>{videoAsset ? videoAsset.name : 'ВИДЕО + ТВОЙ ГОЛОС'}</small>
        <button
          className="icon-button"
          title="Как пользоваться"
          aria-label="Как пользоваться"
          onClick={() => setHelp(true)}
        >
          <HelpCircle size={16} />
        </button>
      </div>
      <div className={`video-stage ${videoAsset ? 'has-video' : ''}`}>
        {videoAsset && videoClip ? (
          <video
            key={videoAsset.id}
            ref={videoRef}
            src={videoAsset.url}
            muted
            playsInline
            onLoadedData={() => {
              if (videoRef.current)
                videoRef.current.currentTime = previewPosition - videoClip.start + videoClip.offset
            }}
          />
        ) : (
          <div className="preview-empty">
            <div className="preview-glyph">
              <span />
              <AudioLines size={44} strokeWidth={1.4} />
              <span />
            </div>
            <div className="preview-empty-copy">
              <span className="eyebrow">ТВОЯ МАЛЕНЬКАЯ СТУДИЯ</span>
              <h1>
                Картинка. Голос.
                <br />
                <em>Всё на своих дорожках.</em>
              </h1>
              <p>
                {videoFiles.length
                  ? 'Перемести курсор на видеоклип для просмотра.'
                  : 'Добавь видео, надень наушники и запиши свою историю.'}
              </p>
              <button
                className="button secondary"
                disabled={recording || !!busy}
                onClick={() => void importNative()}
              >
                <Plus size={15} />
                Открыть медиа
              </button>
            </div>
            <div className="preview-corner top-left" />
            <div className="preview-corner bottom-right" />
          </div>
        )}
        {recording && (
          <div className="recording-badge">
            <span />
            REC {formatTime(position - recordStart)}
          </div>
        )}
      </div>
    </section>
  )
}

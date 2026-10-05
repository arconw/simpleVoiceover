import { t } from '../i18n'
import { AudioLines, Plus } from 'lucide-react'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { formatTime } from '../types'
import { useRef } from 'react'
import type { StudioController } from '../useStudio'
import OperationProgress from './OperationProgress'

type Props = Pick<
  StudioController,
  | 'position'
  | 'recording'
  | 'recordStart'
  | 'busy'
  | 'progress'
  | 'showError'
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
  progress,
  showError,
  importNative,
  videoRef,
  previewPosition,
  videoClip,
  videoAsset,
  videoFiles,
}: Props) {
  const dragOrigin = useRef<{ x: number; y: number } | null>(null)
  const stageRef = useRef<HTMLDivElement>(null)
  return (
    <section className="preview-panel">
      <div
        ref={stageRef}
        title={t('preview.fullscreen')}
        className={`video-stage ${videoAsset ? 'has-video' : ''}`}
        onMouseDown={(event) => {
          if (
            event.button !== 0 ||
            !isTauri() ||
            document.fullscreenElement ||
            (event.target as Element).closest('button,input,select')
          )
            return
          event.preventDefault()
          dragOrigin.current = { x: event.clientX, y: event.clientY }
        }}
        onMouseMove={(event) => {
          const origin = dragOrigin.current
          if (!origin || !(event.buttons & 1)) {
            dragOrigin.current = null
            return
          }
          if (Math.hypot(event.clientX - origin.x, event.clientY - origin.y) < 5) return
          dragOrigin.current = null
          void getCurrentWindow().startDragging().catch(showError)
        }}
        onMouseUp={() => {
          dragOrigin.current = null
        }}
        onMouseLeave={() => {
          dragOrigin.current = null
        }}
        onDoubleClick={(event) => {
          if ((event.target as Element).closest('button,input,select')) return
          dragOrigin.current = null
          const operation = document.fullscreenElement
            ? document.exitFullscreen()
            : stageRef.current?.requestFullscreen()
          void operation?.catch(showError)
        }}
      >
        <OperationProgress busy={busy} progress={progress} />
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
              <span className="eyebrow">{t('preview.eyebrow')}</span>
              <h1>
                {t('preview.headline')} <br />
                <em>{t('preview.tagline')}</em>
              </h1>
              <p>{videoFiles.length ? t('preview.seekHint') : t('preview.importHint')}</p>
              <button
                className="button secondary"
                disabled={recording || !!busy}
                onClick={() => void importNative()}
              >
                <Plus size={15} />
                {t('media.open')}{' '}
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

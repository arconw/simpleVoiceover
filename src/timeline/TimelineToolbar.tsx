import { Maximize2, Minus, MousePointer2, Plus, Scissors, Trash2 } from 'lucide-react'
import type { Track } from '../types'
import type { TimelineProps } from './types'
import type { TimelineViewport } from './useTimelineViewport'
import { minimumSpan } from './viewport'

type Props = Pick<
  TimelineProps,
  | 'tracks'
  | 'tool'
  | 'onToolChange'
  | 'recording'
  | 'selectedClipId'
  | 'onSplitClip'
  | 'position'
  | 'onRemoveClip'
  | 'duration'
  | 'onAddTrack'
> & { viewport: TimelineViewport; canDelete: boolean; selectedTrack?: Track }
export default function TimelineToolbar({
  tracks,
  tool,
  onToolChange,
  recording,
  selectedClipId,
  onSplitClip,
  position,
  onRemoveClip,
  duration,
  onAddTrack,
  viewport,
  canDelete,
  selectedTrack,
}: Props) {
  const { view, setView, zoom } = viewport
  return (
    <div className="tl-toolbar">
      <div className="tl-toolbar-group">
        <span className="tl-toolbar-title">
          ДОРОЖКИ <span>{tracks.length.toString().padStart(2, '0')}</span>
        </span>
        <span className="tl-divider" />
        <button
          className={`tl-tool ${tool === 'select' ? 'is-active' : ''}`}
          title="Выделить и переместить (V)"
          aria-label="Выделить и переместить"
          aria-pressed={tool === 'select'}
          onClick={() => onToolChange('select')}
        >
          <MousePointer2 size={15} />
        </button>
        <button
          className={`tl-tool ${tool === 'split' ? 'is-active' : ''}`}
          title="Разрезать клип (X)"
          aria-label="Разрезать клип"
          aria-pressed={tool === 'split'}
          onClick={() => onToolChange('split')}
          disabled={recording}
        >
          <Scissors size={15} />
        </button>
        <button
          className="tl-tool"
          title="Разрезать выбранный клип в позиции курсора"
          aria-label="Разрезать в позиции курсора"
          disabled={!canDelete}
          onClick={() => {
            if (selectedTrack && selectedClipId)
              onSplitClip(selectedTrack.id, selectedClipId, position)
          }}
        >
          <Scissors size={12} />
        </button>
        <button
          className="tl-tool"
          title="Удалить выделенный клип (Delete)"
          aria-label="Удалить выделенный клип"
          disabled={!canDelete}
          onClick={onRemoveClip}
        >
          <Trash2 size={14} />
        </button>
      </div>
      <div className="tl-toolbar-group">
        <span className="tl-zoom-label">
          {view.span < 60
            ? `${Math.round(view.span)} сек`
            : `${Math.round((view.span / 60) * 10) / 10} мин`}
        </span>
        <button
          className="tl-tool"
          title="Уменьшить масштаб"
          aria-label="Уменьшить масштаб"
          onClick={() => zoom(1.5)}
        >
          <Minus size={15} />
        </button>
        <button
          className="tl-tool"
          title="Увеличить масштаб"
          aria-label="Увеличить масштаб"
          disabled={view.span <= minimumSpan}
          onClick={() => zoom(1 / 1.5)}
        >
          <Plus size={15} />
        </button>
        <button
          className="tl-tool"
          title="Показать весь проект"
          aria-label="Показать весь проект"
          onClick={() =>
            setView({ start: 0, span: Math.max(5, duration > 0 ? duration * 1.05 : 60) })
          }
        >
          <Maximize2 size={14} />
        </button>
        <span className="tl-divider" />
        <button className="tl-add-track" onClick={onAddTrack} disabled={recording}>
          <Plus size={14} /> Дорожка
        </button>
      </div>
    </div>
  )
}

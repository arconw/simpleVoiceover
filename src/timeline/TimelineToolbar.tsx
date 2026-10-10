import { t } from '../i18n'
import {
  Copy,
  ClipboardPaste,
  Maximize2,
  Magnet,
  Minus,
  MousePointer2,
  Plus,
  Scissors,
  Trash2,
} from 'lucide-react'
import type { Track } from '../types'
import type { TimelineProps } from './types'
import type { TimelineViewport } from './useTimelineViewport'
import { minimumSpan } from './viewport'

type Props = Pick<
  TimelineProps,
  | 'tracks'
  | 'transport'
  | 'tool'
  | 'onToolChange'
  | 'snapping'
  | 'onSnappingChange'
  | 'recording'
  | 'selectedClipId'
  | 'onSplitClip'
  | 'position'
  | 'onRemoveClip'
  | 'duration'
  | 'onAddTrack'
  | 'onCopy'
  | 'onPaste'
  | 'canPaste'
  | 'selection'
  | 'operationPending'
> & { viewport: TimelineViewport; canDelete: boolean; selectedTrack?: Track }
export default function TimelineToolbar({
  tracks,
  tool,
  onToolChange,
  snapping,
  onSnappingChange,
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
  transport,
  onCopy,
  onPaste,
  canPaste,
  selection,
  operationPending,
}: Props) {
  const { view, setView, zoom } = viewport
  return (
    <div className="tl-toolbar">
      <div className="tl-toolbar-group">
        <span className="tl-toolbar-title">
          {t('timeline.tracks')} <span>{tracks.length.toString().padStart(2, '0')}</span>
        </span>
        <span className="tl-divider" />
        <button
          className={`tl-tool ${tool === 'select' ? 'is-active' : ''}`}
          title={t('tools.selectTitle')}
          aria-label={t('tools.selectMove')}
          aria-pressed={tool === 'select'}
          onClick={() => onToolChange('select')}
        >
          <MousePointer2 size={15} />
        </button>
        <button
          className={`tl-tool ${tool === 'split' ? 'is-active' : ''}`}
          title={t('tools.splitTitle')}
          aria-label={t('tools.splitClip')}
          aria-pressed={tool === 'split'}
          onClick={() => onToolChange('split')}
          disabled={recording}
        >
          <Scissors size={15} />
        </button>
        <button
          className="tl-tool"
          title={t('tools.splitAtPlayheadTitle')}
          aria-label={t('tools.splitAtPlayhead')}
          disabled={!canDelete}
          onClick={() => {
            if (selectedTrack && selectedClipId)
              onSplitClip(selectedTrack.id, selectedClipId, position)
          }}
        >
          <Scissors size={12} />
        </button>
        <button
          className={`tl-tool ${snapping ? 'is-active' : ''}`}
          title={`${t('tools.snap')} · ${snapping ? t('common.on') : t('common.off')}`}
          aria-label={t('tools.snap')}
          aria-pressed={snapping}
          onClick={() => onSnappingChange(!snapping)}
        >
          <Magnet size={15} />
        </button>
        <button
          className="tl-tool"
          title={t('tools.deleteTitle')}
          aria-label={t('tools.delete')}
          disabled={!canDelete}
          onClick={onRemoveClip}
        >
          <Trash2 size={14} />
        </button>
        <button
          className="tl-tool"
          title={t('selection.copy')}
          aria-label={t('selection.copy')}
          disabled={
            recording || operationPending || (!selectedClipId && !selection?.regions.length)
          }
          onClick={onCopy}
        >
          <Copy size={13} />
        </button>
        <button
          className="tl-tool"
          title={t('selection.paste')}
          aria-label={t('selection.paste')}
          disabled={recording || operationPending || !canPaste}
          onClick={onPaste}
        >
          <ClipboardPaste size={13} />
        </button>
      </div>
      {transport}
      <div className="tl-toolbar-group">
        <span className="tl-zoom-label">
          {view.span < 60
            ? t('units.secondsValue', { value0: Math.round(view.span) })
            : t('units.minutesValue', { value0: Math.round((view.span / 60) * 10) / 10 })}
        </span>
        <button
          className="tl-tool"
          title={t('zoom.outTitle')}
          aria-label={t('zoom.out')}
          onClick={() => zoom(1.5)}
        >
          <Minus size={15} />
        </button>
        <button
          className="tl-tool"
          title={t('zoom.inTitle')}
          aria-label={t('zoom.in')}
          disabled={view.span <= minimumSpan}
          onClick={() => zoom(1 / 1.5)}
        >
          <Plus size={15} />
        </button>
        <button
          className="tl-tool"
          title={t('zoom.fit')}
          aria-label={t('zoom.fit')}
          onClick={() =>
            setView({ start: 0, span: Math.max(5, duration > 0 ? duration * 1.05 : 60) })
          }
        >
          <Maximize2 size={14} />
        </button>
        <span className="tl-divider" />
        <button className="tl-add-track" onClick={onAddTrack} disabled={recording}>
          <Plus size={14} /> {t('timeline.addTrack')}{' '}
        </button>
      </div>
    </div>
  )
}

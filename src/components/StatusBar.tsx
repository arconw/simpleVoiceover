import { t } from '../i18n'
import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  'tracks' | 'assets' | 'recording' | 'busy' | 'undo' | 'redo' | 'snapshot'
>

export default function StatusBar({
  tracks,
  assets,
  recording,
  busy,
  undo,
  redo,
  snapshot,
}: Props) {
  return (
    <footer className="statusbar">
      <span>
        <span className="small-dot" />
        {busy || t('status.summary', { value0: tracks.length, value1: assets.length })}
      </span>
      <span>{t('status.format')}</span>
      <button onClick={undo} disabled={!snapshot?.canUndo || recording || !!busy}>
        {t('history.undo')} <kbd>Ctrl Z</kbd>
      </button>
      <button onClick={redo} disabled={!snapshot?.canRedo || recording || !!busy}>
        {t('history.redo')} <kbd>Ctrl Shift Z</kbd>
      </button>
      <span className="status-hint">
        {t('status.playShortcut')}
        <span>{t('status.recordShortcut')}</span>
      </span>
    </footer>
  )
}

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
        {busy || `${tracks.length} дорожки · ${assets.length} файлов`}
      </span>
      <span>WAV / MP3 · 48 кГц · Стерео</span>
      <button onClick={undo} disabled={!snapshot?.canUndo || recording || !!busy}>
        Отменить <kbd>Ctrl Z</kbd>
      </button>
      <button onClick={redo} disabled={!snapshot?.canRedo || recording || !!busy}>
        Вернуть <kbd>Ctrl Shift Z</kbd>
      </button>
      <span className="status-hint">
        Space — слушать<span>R — записывать</span>
      </span>
    </footer>
  )
}

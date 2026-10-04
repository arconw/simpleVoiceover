import { useEffect, useRef, useState } from 'react'
import { AudioLines, Download, FolderOpen, Keyboard, Menu, Plus, Save, X } from 'lucide-react'
import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  | 'recording'
  | 'busy'
  | 'importNative'
  | 'duration'
  | 'selectedTrack'
  | 'saveProject'
  | 'exportAudio'
  | 'openNative'
  | 'snapshot'
  | 'setHelp'
  | 'chooseWorkingDirectory'
>

export default function StudioHeader({
  recording,
  busy,
  importNative,
  duration,
  selectedTrack,
  saveProject,
  exportAudio,
  openNative,
  snapshot,
  setHelp,
  chooseWorkingDirectory,
}: Props) {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (!open) return
    const outside = (event: PointerEvent) => {
      if (!ref.current?.contains(event.target as Node)) setOpen(false)
    }
    const escape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }
    document.addEventListener('pointerdown', outside)
    document.addEventListener('keydown', escape)
    return () => {
      document.removeEventListener('pointerdown', outside)
      document.removeEventListener('keydown', escape)
    }
  }, [open])
  const run = (action: () => unknown) => {
    setOpen(false)
    void action()
  }
  const disabled = recording || !!busy
  return (
    <div className="studio-menu" ref={ref}>
      <button
        className="menu-toggle"
        aria-label="Меню проекта"
        aria-expanded={open}
        aria-controls="project-menu"
        onClick={() => setOpen(!open)}
      >
        {open ? <X size={17} /> : <Menu size={17} />}
        {snapshot?.config.dirty && <span className="unsaved-dot" />}
      </button>
      {open && (
        <nav id="project-menu" className="project-menu" aria-label="Действия проекта">
          <div className="menu-brand">
            <AudioLines size={21} />
            <strong>simpleVoiceover</strong>
          </div>
          <p className="menu-state">
            {snapshot?.config.dirty ? 'Есть несохранённые изменения' : 'Проект сохранён'}
          </p>
          <button disabled={disabled} onClick={() => run(importNative)}>
            <Plus size={16} />
            Добавить медиа
          </button>
          <button disabled={disabled} onClick={() => run(openNative)}>
            <FolderOpen size={16} />
            Открыть проект
          </button>
          <button disabled={disabled} onClick={() => run(() => saveProject())}>
            <Save size={16} />
            Сохранить<kbd>Ctrl S</kbd>
          </button>
          <button disabled={disabled} onClick={() => run(() => saveProject(true))}>
            <Save size={16} />
            Сохранить как…
          </button>
          <button disabled={disabled} onClick={() => run(chooseWorkingDirectory)}>
            <FolderOpen size={16} />
            Рабочий каталог
          </button>
          <span className="menu-section">ЭКСПОРТ МИКСА</span>
          <div className="menu-formats">
            <button disabled={disabled || !duration} onClick={() => run(() => exportAudio())}>
              <Download size={14} />
              WAV
            </button>
            <button
              disabled={disabled || !duration}
              onClick={() => run(() => exportAudio(null, 'mp3'))}
            >
              <Download size={14} />
              MP3
            </button>
          </div>
          <span className="menu-section">ДОРОЖКА · {selectedTrack.name}</span>
          <div className="menu-formats">
            <button
              disabled={disabled || !selectedTrack.clips.length}
              onClick={() => run(() => exportAudio(selectedTrack.id))}
            >
              WAV
            </button>
            <button
              disabled={disabled || !selectedTrack.clips.length}
              onClick={() => run(() => exportAudio(selectedTrack.id, 'mp3'))}
            >
              MP3
            </button>
          </div>
          <button onClick={() => run(() => setHelp(true))}>
            <Keyboard size={16} />
            Справка и клавиши
          </button>
        </nav>
      )}
    </div>
  )
}

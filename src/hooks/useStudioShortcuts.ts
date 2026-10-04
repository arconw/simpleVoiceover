import { useEffect } from 'react'
import { shortcutFor } from '../shortcuts'

interface Actions {
  play: () => Promise<void>
  record: () => Promise<void>
  saveProject: (saveAs?: boolean) => Promise<boolean>
  undo: () => void
  redo: () => void
  removeClip: () => void
  seek: (position: number) => void
  setTool: (tool: 'select' | 'split') => void
  blocked: boolean
}

export function useStudioShortcuts(actions: Actions) {
  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if (
        actions.blocked ||
        event.defaultPrevented ||
        (event.target as HTMLElement).closest(
          'input,textarea,select,[contenteditable=true],[role="dialog"]',
        )
      )
        return
      const shortcut = shortcutFor(event)
      if (!shortcut) return
      event.preventDefault()
      if (event.repeat) return
      if (shortcut === 'play') void actions.play()
      if (shortcut === 'record') void actions.record()
      if (shortcut === 'save' || shortcut === 'saveAs')
        void actions.saveProject(shortcut === 'saveAs')
      if (shortcut === 'undo') actions.undo()
      if (shortcut === 'redo') actions.redo()
      if (shortcut === 'delete') actions.removeClip()
      if (shortcut === 'home') actions.seek(0)
      if (shortcut === 'select' || shortcut === 'split') actions.setTool(shortcut)
    }
    window.addEventListener('keydown', keydown)
    return () => window.removeEventListener('keydown', keydown)
  }, [actions])
}

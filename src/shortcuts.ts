type Shortcut =
  'play' | 'record' | 'save' | 'saveAs' | 'undo' | 'redo' | 'delete' | 'home' | 'select' | 'split'

export function shortcutFor(
  event: Pick<KeyboardEvent, 'code' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>,
): Shortcut | null {
  if (event.altKey) return null
  if (event.ctrlKey || event.metaKey) {
    if (event.code === 'KeyS') return event.shiftKey ? 'saveAs' : 'save'
    if (event.code === 'KeyZ') return event.shiftKey ? 'redo' : 'undo'
    return null
  }
  const shortcuts: Record<string, Shortcut> = {
    Space: 'play',
    KeyR: 'record',
    KeyV: 'select',
    KeyX: 'split',
    Delete: 'delete',
    Home: 'home',
  }
  return shortcuts[event.code] ?? null
}

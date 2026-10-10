import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useCallback, useEffect, useRef, useState } from 'react'

export function usePreviewFullscreen(showError: (reason: unknown) => void) {
  const [fullscreen, setFullscreen] = useState(false)
  const current = useRef(false)
  const transition = useRef<Promise<void>>(Promise.resolve())
  const change = useCallback(
    (value: boolean | 'toggle') => {
      const operation = transition.current.then(async () => {
        const enabled = value === 'toggle' ? !current.current : value
        if (enabled === current.current) return
        if (isTauri()) await getCurrentWindow().setFullscreen(enabled)
        current.current = enabled
        setFullscreen(enabled)
      })
      transition.current = operation.catch(showError)
    },
    [showError],
  )
  useEffect(() => {
    if (!fullscreen) return
    const escape = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.preventDefault()
      event.stopPropagation()
      change(false)
    }
    document.addEventListener('keydown', escape, true)
    return () => document.removeEventListener('keydown', escape, true)
  }, [fullscreen, change])
  return { fullscreen, toggleFullscreen: () => change('toggle') }
}

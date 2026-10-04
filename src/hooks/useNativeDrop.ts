import { useEffect, useRef, useState } from 'react'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { isTauri } from '@tauri-apps/api/core'

export function useNativeDrop(
  importPaths: (paths: string[]) => Promise<void>,
  showError: (reason: unknown) => void,
) {
  const [dragging, setDragging] = useState(false)
  const current = useRef(importPaths)
  current.current = importPaths
  useEffect(() => {
    if (!isTauri()) return
    const subscription = getCurrentWebview().onDragDropEvent((event) => {
      setDragging(event.payload.type === 'over' || event.payload.type === 'enter')
      if (event.payload.type === 'drop') void current.current(event.payload.paths).catch(showError)
    })
    return () => {
      void subscription.then((unlisten) => unlisten())
    }
  }, [showError])
  return dragging
}

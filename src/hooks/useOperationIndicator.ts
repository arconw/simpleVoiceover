import { useEffect, useState } from 'react'

export function useOperationIndicator(busy: string) {
  const [visible, setVisible] = useState(false)
  const pending = !!busy
  useEffect(() => {
    if (!pending) {
      setVisible(false)
      return
    }
    const timer = window.setTimeout(() => setVisible(true), 300)
    return () => window.clearTimeout(timer)
  }, [pending])
  return pending && visible ? busy : ''
}

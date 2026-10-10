import { useCallback, useEffect, useState } from 'react'

export function useTransientMessage() {
  const [message, update] = useState({ text: '', revision: 0 })
  const setMessage = useCallback((text: string) => {
    update((previous) => ({ text, revision: previous.revision + 1 }))
  }, [])
  useEffect(() => {
    if (!message.text) return
    const timer = setTimeout(() => setMessage(''), 3000)
    return () => clearTimeout(timer)
  }, [message, setMessage])
  return [message.text, setMessage] as const
}

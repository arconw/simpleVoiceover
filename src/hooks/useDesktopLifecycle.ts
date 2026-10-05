import { t } from '../i18n'
import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface Options {
  dirty: boolean
  busy: string
  recording: boolean
  saveProject: () => Promise<boolean>
  openProject: () => Promise<void>
  pause: () => Promise<void>
  showError: (reason: unknown) => void
}

export function useDesktopLifecycle(options: Options) {
  const [confirmMode, setConfirmMode] = useState<'close' | 'open' | null>(null)
  const current = useRef(options)
  current.current = options
  const proceed = async (mode: 'close' | 'open', discard: boolean) => {
    try {
      await current.current.pause()
      if (mode === 'close') await invoke('close_studio', { withoutSaving: discard })
      else await current.current.openProject()
      setConfirmMode(null)
    } catch (reason) {
      current.current.showError(reason)
    }
  }
  const request = (mode: 'close' | 'open') => {
    const state = current.current
    if (state.busy || state.recording) {
      state.showError(
        new Error(state.recording ? t('error.recordingBeforeClose') : t('error.waitBeforeClose')),
      )
      return
    }
    if (state.dirty) setConfirmMode(mode)
    else void proceed(mode, false)
  }
  useEffect(() => {
    const subscription = listen('studio-close-requested', () => request('close'))
    return () => {
      void subscription.then((unlisten) => unlisten())
    }
  }, [])
  const confirmSave = async () => {
    if (confirmMode && (await current.current.saveProject())) await proceed(confirmMode, false)
  }
  const confirmDiscard = () => {
    if (confirmMode) void proceed(confirmMode, true)
  }
  return {
    confirmMode,
    confirmSave,
    confirmDiscard,
    cancelConfirm: () => setConfirmMode(null),
    requestOpen: () => request('open'),
    requestClose: () => request('close'),
  }
}

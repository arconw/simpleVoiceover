import { useEffect, useRef } from 'react'
import { localizedName, t } from '../i18n'
import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  'trackToRemove' | 'confirmRemoveTrack' | 'cancelRemoveTrack' | 'operationPending'
>

export default function RemoveTrackDialog({
  trackToRemove,
  confirmRemoveTrack,
  cancelRemoveTrack,
  operationPending,
}: Props) {
  const dialog = useRef<HTMLElement>(null)
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null
    dialog.current?.querySelector<HTMLButtonElement>('button')?.focus()
    return () => previous?.focus({ preventScroll: true })
  }, [])
  return (
    <div className="modal-backdrop">
      <section
        ref={dialog}
        className="help-modal"
        role="dialog"
        aria-modal="true"
        aria-label={t('track.deleteConfirmTitle')}
        onKeyDown={(event) => {
          if (event.key === 'Escape' && !operationPending) {
            event.preventDefault()
            event.stopPropagation()
            cancelRemoveTrack()
          }
          if (event.key === 'Tab') {
            const buttons = [
              ...event.currentTarget.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'),
            ]
            const next =
              buttons[
                (buttons.indexOf(document.activeElement as HTMLButtonElement) +
                  (event.shiftKey ? -1 : 1) +
                  buttons.length) %
                  buttons.length
              ]
            event.preventDefault()
            next?.focus()
          }
        }}
      >
        <h2>{t('track.deleteConfirmTitle')}</h2>
        <p className="hint muted">
          {t('track.deleteConfirmBody', { name: localizedName(trackToRemove?.name ?? '') })}
        </p>
        <div className="dialog-actions">
          <button
            className="button secondary"
            disabled={operationPending}
            onClick={cancelRemoveTrack}
          >
            {t('common.cancel')}
          </button>
          <button
            className="button danger"
            disabled={operationPending}
            onClick={() => void confirmRemoveTrack()}
          >
            {t('track.deleteConfirm')}
          </button>
        </div>
      </section>
    </div>
  )
}

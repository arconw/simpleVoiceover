import { t } from '../i18n'
import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  'confirmMode' | 'confirmSave' | 'confirmDiscard' | 'cancelConfirm' | 'operationPending'
>

export default function CloseDialog({
  confirmMode,
  confirmSave,
  confirmDiscard,
  cancelConfirm,
  operationPending,
}: Props) {
  return (
    <div className="modal-backdrop">
      <section
        className="help-modal"
        role="dialog"
        aria-modal="true"
        aria-label={t('close.dialog')}
      >
        <h2>{t('close.title')}</h2>
        <p className="hint muted">
          {t('close.unsaved')}{' '}
          {confirmMode === 'close' ? t('close.stopEngine') : t('close.openAnother')}
        </p>
        <div className="dialog-actions">
          <button className="button secondary" disabled={operationPending} onClick={cancelConfirm}>
            {t('common.cancel')}{' '}
          </button>
          <button className="button secondary" disabled={operationPending} onClick={confirmDiscard}>
            {confirmMode === 'close' ? t('close.discard') : t('close.openDiscard')}
          </button>
          <button
            className="button accent"
            disabled={operationPending}
            onClick={() => void confirmSave()}
          >
            {t('project.save')}{' '}
          </button>
        </div>
      </section>
    </div>
  )
}

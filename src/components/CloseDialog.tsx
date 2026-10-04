import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  'confirmMode' | 'confirmSave' | 'confirmDiscard' | 'cancelConfirm' | 'busy'
>

export default function CloseDialog({
  confirmMode,
  confirmSave,
  confirmDiscard,
  cancelConfirm,
  busy,
}: Props) {
  return (
    <div className="modal-backdrop">
      <section
        className="help-modal"
        role="dialog"
        aria-modal="true"
        aria-label="Несохранённые изменения"
      >
        <h2>Сохранить изменения?</h2>
        <p className="hint muted">
          В проекте есть несохранённые изменения.{' '}
          {confirmMode === 'close'
            ? 'После закрытия приложения аудиодвижок остановится.'
            : 'Перед открытием другого проекта можно сохранить текущий.'}
        </p>
        <div className="dialog-actions">
          <button className="button secondary" disabled={!!busy} onClick={cancelConfirm}>
            Отмена
          </button>
          <button className="button secondary" disabled={!!busy} onClick={confirmDiscard}>
            {confirmMode === 'close' ? 'Закрыть без сохранения' : 'Открыть без сохранения'}
          </button>
          <button className="button accent" disabled={!!busy} onClick={() => void confirmSave()}>
            Сохранить
          </button>
        </div>
      </section>
    </div>
  )
}

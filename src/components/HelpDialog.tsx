import { t } from '../i18n'
import { X } from 'lucide-react'
import type { StudioController } from '../useStudio'

type Props = Pick<StudioController, 'setHelp'>

export default function HelpDialog({ setHelp }: Props) {
  return (
    <div className="modal-backdrop" onClick={() => setHelp(false)}>
      <section
        className="help-modal"
        role="dialog"
        aria-modal="true"
        aria-label={t('help.dialog')}
        onClick={(e) => e.stopPropagation()}
      >
        <button className="modal-close" aria-label={t('help.close')} onClick={() => setHelp(false)}>
          <X size={18} />
        </button>
        <span className="eyebrow">{t('help.eyebrow')}</span>
        <h2>{t('help.title')}</h2>
        <ol>
          <li>
            <strong>{t('help.videoTitle')}</strong> {t('help.videoBody')}{' '}
          </li>
          <li>
            <strong>{t('help.armTitle')}</strong> {t('help.armBody')}{' '}
          </li>
          <li>
            <strong>{t('help.recordTitle')}</strong> {t('help.recordBody')}{' '}
          </li>
          <li>
            <strong>{t('help.effectsTitle')}</strong> {t('help.effectsBody')}{' '}
          </li>
          <li>
            <strong>{t('help.saveTitle')}</strong> {t('help.saveBody')}{' '}
          </li>
        </ol>
        <div className="shortcut-grid">
          <span>
            <kbd>Space</kbd>
            {t('help.playShortcut')}{' '}
          </span>
          <span>
            <kbd>R</kbd>
            {t('help.recordShortcut')}{' '}
          </span>
          <span>
            <kbd>V</kbd>
            {t('tools.select')}{' '}
          </span>
          <span>
            <kbd>X</kbd>
            {t('tools.split')}{' '}
          </span>
          <span>
            <kbd>Delete</kbd>
            {t('tools.remove')}{' '}
          </span>
          <span>
            <kbd>Ctrl Z</kbd>
            {t('history.undo')}{' '}
          </span>
          <span>
            <kbd>Ctrl Shift Z</kbd>
            {t('history.redo')}{' '}
          </span>
          <span>
            <kbd>Ctrl S</kbd>
            {t('help.saveShortcut')}{' '}
          </span>
          <span>
            <kbd>Ctrl C</kbd>
            {t('help.copyShortcut')}
          </span>
          <span>
            <kbd>Ctrl V</kbd>
            {t('help.pasteShortcut')}
          </span>
          <span>
            <kbd>Ctrl + / −</kbd>
            {t('help.zoomShortcut')}{' '}
          </span>
          <span>
            <kbd>{t('help.wheelKeys')}</kbd>
            {t('help.wheelShortcut')}{' '}
          </span>
        </div>
        <p className="muted hint">{t('help.editing')} </p>
        <p className="muted hint">{t('help.selection')}</p>
      </section>
    </div>
  )
}

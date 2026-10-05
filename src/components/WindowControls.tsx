import { t } from '../i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Maximize2, Minus, X } from 'lucide-react'
import type { StudioController } from '../useStudio'

type Props = Pick<StudioController, 'requestClose' | 'showError'>

export default function WindowControls({ requestClose, showError }: Props) {
  return (
    <div className="window-controls" aria-label={t('window.controls')}>
      <button
        title={t('window.minimize')}
        aria-label={t('window.minimizeLabel')}
        onClick={() => void getCurrentWindow().minimize().catch(showError)}
      >
        <Minus size={16} />
      </button>
      <button
        title={t('window.maximize')}
        aria-label={t('window.maximizeLabel')}
        onClick={() => void getCurrentWindow().toggleMaximize().catch(showError)}
      >
        <Maximize2 size={14} />
      </button>
      <button
        className="window-close"
        title={t('common.close')}
        aria-label={t('window.closeLabel')}
        onClick={requestClose}
      >
        <X size={17} />
      </button>
    </div>
  )
}

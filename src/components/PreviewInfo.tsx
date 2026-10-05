import { t } from '../i18n'
import { HelpCircle, Monitor } from 'lucide-react'
import type { StudioController } from '../useStudio'

type Props = Pick<StudioController, 'videoAsset' | 'setHelp'>

export default function PreviewInfo({ videoAsset, setHelp }: Props) {
  const name = videoAsset?.name ?? t('preview.placeholderName')
  const extensionIndex = videoAsset ? name.lastIndexOf('.') : -1
  const basename = extensionIndex > 0 ? name.slice(0, extensionIndex) : name
  const extension = extensionIndex > 0 ? name.slice(extensionIndex) : ''
  return (
    <div className="preview-heading">
      <Monitor size={14} />
      <div className="preview-details">
        <span title={t('preview.title')}>{t('preview.title')}</span>
        <small title={name}>
          <span className="preview-filename">{basename}</span>
          {extension && <span className="preview-extension">{extension}</span>}
        </small>
      </div>
      <button
        className="icon-button"
        title={t('help.howTo')}
        aria-label={t('help.howTo')}
        onClick={() => setHelp(true)}
      >
        <HelpCircle size={16} />
      </button>
    </div>
  )
}

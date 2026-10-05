import { t, translateMessage } from '../i18n'
import { Check, HelpCircle, X } from 'lucide-react'
import type { StudioController } from '../useStudio'

type Props = Pick<StudioController, 'notice' | 'error' | 'setNotice' | 'setError'>

export default function Notice({ notice, error, setNotice, setError }: Props) {
  if (!notice && !error) return null
  return (
    <div className={`toast ${error ? 'error' : ''}`} role={error ? 'alert' : 'status'}>
      {error ? <HelpCircle size={17} /> : <Check size={17} />}
      <span className="notice-content">{translateMessage(error || notice)}</span>
      <button
        aria-label={t('notice.close')}
        onClick={() => {
          setNotice('')
          setError('')
        }}
      >
        <X size={15} />
      </button>
    </div>
  )
}

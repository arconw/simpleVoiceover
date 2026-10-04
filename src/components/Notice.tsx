import { Check, HelpCircle, X } from 'lucide-react'
import type { StudioController } from '../useStudio'

type Props = Pick<StudioController, 'notice' | 'error' | 'busy' | 'setNotice' | 'setError'>

export default function Notice({ notice, error, busy, setNotice, setError }: Props) {
  if (!notice && !error && !busy) return null
  return (
    <div className={`toast ${error ? 'error' : ''}`} role={error ? 'alert' : 'status'}>
      {busy && !error ? (
        <span className="spinner" />
      ) : error ? (
        <HelpCircle size={17} />
      ) : (
        <Check size={17} />
      )}
      <span>{error || busy || notice}</span>
      {!busy && (
        <button
          aria-label="Закрыть уведомление"
          onClick={() => {
            setNotice('')
            setError('')
          }}
        >
          <X size={15} />
        </button>
      )}
    </div>
  )
}

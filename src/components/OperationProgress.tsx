import type { StudioController } from '../useStudio'
import { translateMessage } from '../i18n'

type Props = Pick<StudioController, 'busy' | 'progress'>

export default function OperationProgress({ busy, progress }: Props) {
  if (!busy) return null
  const label = translateMessage(progress?.label ?? busy)
  return (
    <div className="operation-progress" role="status" aria-live="polite">
      <span className="spinner" />
      <span className="notice-content">
        {label}
        {progress && ` · ${Math.floor(progress.percent)}%`}
        {progress && <progress max={100} value={progress.percent} aria-label={label} />}
      </span>
    </div>
  )
}

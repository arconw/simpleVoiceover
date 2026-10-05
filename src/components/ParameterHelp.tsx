import { useLayoutEffect, useId, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { t } from '../i18n'
import { parameterCurve, type ParameterKind } from '../parameterCurves'

interface Props {
  kind: ParameterKind
  label: string
  value: number
  min: number
  max: number
}

export default function ParameterHelp({ kind, label, value, min, max }: Props) {
  const [open, setOpen] = useState(false)
  const [position, setPosition] = useState({ left: 0, top: 0 })
  const buttonRef = useRef<HTMLButtonElement>(null)
  const tooltipRef = useRef<HTMLDivElement>(null)
  const id = useId()
  useLayoutEffect(() => {
    if (!open) return
    const place = () => {
      const bounds = buttonRef.current?.getBoundingClientRect()
      const tooltip = tooltipRef.current?.getBoundingClientRect()
      if (!bounds || !tooltip) return
      const below = bounds.bottom + 8
      const top =
        below + tooltip.height > window.innerHeight - 8 ? bounds.top - tooltip.height - 8 : below
      setPosition({
        left: Math.max(8, Math.min(window.innerWidth - tooltip.width - 8, bounds.left - 12)),
        top: Math.max(8, Math.min(window.innerHeight - tooltip.height - 8, top)),
      })
    }
    const close = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }
    place()
    window.addEventListener('resize', place)
    window.addEventListener('scroll', place, true)
    window.addEventListener('keydown', close)
    return () => {
      window.removeEventListener('resize', place)
      window.removeEventListener('scroll', place, true)
      window.removeEventListener('keydown', close)
    }
  }, [open, label, kind])
  const curve = parameterCurve(kind, value, min, max)
  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        className="parameter-help"
        aria-label={t('parameter.help', { name: label })}
        aria-describedby={open ? id : undefined}
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onClick={() => setOpen(!open)}
      >
        ?
      </button>
      {open &&
        createPortal(
          <div
            ref={tooltipRef}
            id={id}
            role="tooltip"
            className="parameter-tooltip"
            style={{ left: position.left, top: position.top }}
          >
            <strong>{label}</strong>
            <p>{t(`parameter.${kind}`)}</p>
            <svg viewBox="0 0 240 70" role="img" aria-label={t('parameter.diagram')}>
              <path d="M8 58H232M8 8V58" className="parameter-axis" />
              <path d={curve.original} className="parameter-source" />
              <path d={curve.processed} className="parameter-processed" />
            </svg>
            <div className="parameter-legend">
              <span>{t('parameter.before')}</span>
              <span>{t('parameter.after')}</span>
            </div>
            <small>{t('parameter.illustration')}</small>
          </div>,
          document.body,
        )}
    </>
  )
}

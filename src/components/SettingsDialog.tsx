import { useEffect } from 'react'
import { X } from 'lucide-react'
import { languages, systemLanguage, t, validPreference } from '../i18n'
import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  'languagePreference' | 'changeLanguage' | 'setSettings' | 'operationPending'
>

export default function SettingsDialog({
  languagePreference,
  changeLanguage,
  setSettings,
  operationPending,
}: Props) {
  useEffect(() => {
    const escape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setSettings(false)
    }
    document.addEventListener('keydown', escape)
    return () => document.removeEventListener('keydown', escape)
  }, [setSettings])
  const detected = languages.find(([language]) => language === systemLanguage())?.[1] ?? 'English'
  return (
    <div className="modal-backdrop" onClick={() => setSettings(false)}>
      <section
        className="help-modal settings-modal"
        role="dialog"
        aria-modal="true"
        aria-label={t('settings.title')}
        onClick={(event) => event.stopPropagation()}
      >
        <button
          className="modal-close"
          aria-label={t('common.close')}
          onClick={() => setSettings(false)}
        >
          <X size={18} />
        </button>
        <h2>{t('settings.title')}</h2>
        <label className="settings-field">
          <span>{t('settings.language')}</span>
          <select
            value={languagePreference}
            disabled={operationPending}
            onChange={(event) => {
              const value = event.target.value
              if (validPreference(value)) void changeLanguage(value)
            }}
          >
            <option value="system">{t('settings.system')}</option>
            {languages.map(([language, name]) => (
              <option value={language} key={language} lang={language}>
                {name}
              </option>
            ))}
          </select>
        </label>
        <p className="hint muted">{t('settings.systemLanguage', { language: detected })}</p>
        <p className="hint muted">{t('settings.description')}</p>
      </section>
    </div>
  )
}

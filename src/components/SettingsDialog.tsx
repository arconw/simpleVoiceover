import { useEffect } from 'react'
import { X } from 'lucide-react'
import { languages, systemLanguage, t, validPreference } from '../i18n'
import type { StudioController } from '../useStudio'

type Props = Pick<
  StudioController,
  | 'languagePreference'
  | 'changeLanguage'
  | 'setSettings'
  | 'operationPending'
  | 'audioDevices'
  | 'audioDevicesPending'
  | 'inputDevice'
  | 'outputDevice'
  | 'changeAudioDevices'
  | 'recording'
>

export default function SettingsDialog({
  languagePreference,
  changeLanguage,
  setSettings,
  operationPending,
  audioDevices,
  audioDevicesPending,
  inputDevice,
  outputDevice,
  changeAudioDevices,
  recording,
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
        {(['input', 'output'] as const).map((kind) => {
          const devices = kind === 'input' ? audioDevices.inputs : audioDevices.outputs
          const value = kind === 'input' ? inputDevice : outputDevice
          const system = devices.find((device) => device.isDefault)
          return (
            <label className="settings-field" key={kind}>
              <span>{t(kind === 'input' ? 'settings.inputDevice' : 'settings.outputDevice')}</span>
              <select
                value={value}
                disabled={
                  operationPending || audioDevicesPending || (kind === 'input' && recording)
                }
                onChange={(event) =>
                  void changeAudioDevices(
                    kind === 'input' ? event.target.value : inputDevice,
                    kind === 'output' ? event.target.value : outputDevice,
                  )
                }
              >
                <option value="">
                  {system
                    ? t('settings.systemDefaultDevice', { name: system.label })
                    : t('settings.systemDefault')}
                </option>
                {value && !devices.some((device) => device.id === value) && (
                  <option value={value} disabled>
                    {t('settings.deviceUnavailable')}
                  </option>
                )}
                {devices.map((device, index) => (
                  <option key={device.id} value={device.id}>
                    {device.label ||
                      t(kind === 'input' ? 'settings.inputFallback' : 'settings.outputFallback', {
                        number: index + 1,
                      })}
                  </option>
                ))}
              </select>
            </label>
          )
        })}
        <p className="hint muted">
          {t(audioDevices.available ? 'settings.audioHint' : 'settings.audioUnavailable')}
        </p>
      </section>
    </div>
  )
}

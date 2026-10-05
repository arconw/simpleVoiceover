import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { basename, extname } from 'node:path'
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  catalogs,
  currentLanguage,
  languages,
  localizedName,
  resolveLanguage,
  setLanguagePreference,
  t,
  translate,
  translateMessage,
  validPreference,
} from '../src/i18n'

const sourceFiles = execFileSync(
  'git',
  ['ls-files', '--cached', '--others', '--exclude-standard', '-z'],
  {
    encoding: 'utf8',
  },
)
  .split('\0')
  .filter((file) => {
    if (!file || file.startsWith('.ctx/') || file.startsWith('src/locales/')) return false
    const name = basename(file)
    if (/^(\.env(?:\..*)?|id_rsa|id_ed25519|credentials\.json|token.*|secrets?\..*)$/i.test(name))
      return false
    if (
      /(secret|password|apikey|api_key)/i.test(name) ||
      /\.(pem|key|p12|pfx|keystore)$/i.test(name)
    )
      return false
    return [
      '.ts',
      '.tsx',
      '.js',
      '.rs',
      '.json',
      '.md',
      '.html',
      '.css',
      '.toml',
      '.ps1',
      '.svg',
      '.sh',
      '',
    ].includes(extname(file))
  })
const parameters = (text: string) =>
  [...text.matchAll(/\{([A-Za-z0-9_]+)\}/g)].map((match) => match[1]).sort()

afterEach(() => {
  vi.unstubAllGlobals()
  setLanguagePreference('system')
})

describe('localization catalogs', () => {
  it('has every nonempty English key and identical parameters in every language', () => {
    const keys = Object.keys(catalogs.en).sort()
    expect(keys.length).toBeGreaterThan(300)
    expect(languages.map(([language]) => language).sort()).toEqual(Object.keys(catalogs).sort())
    for (const [language, catalog] of Object.entries(catalogs)) {
      expect(Object.keys(catalog).sort(), language).toEqual(keys)
      for (const key of keys) {
        const value = catalog[key as keyof typeof catalog]
        expect(value.trim().length, `${language}:${key}`).toBeGreaterThan(0)
        expect(parameters(value), `${language}:${key}`).toEqual(
          parameters(catalogs.en[key as keyof typeof catalogs.en]),
        )
        expect(value, `${language}:${key}`).not.toMatch(
          /__[Pp]\d+__|\uFFFD|[\u202a-\u202e\u2066-\u2069]/,
        )
      }
    }
  })
  it('uses complete English fallback for missing and empty translations', () => {
    expect(translate('fr', 'project.save', {}, {})).toBe(catalogs.en['project.save'])
    expect(translate('pl', 'project.save', {}, { 'project.save': '  ' })).toBe(
      catalogs.en['project.save'],
    )
    expect(translate('fr', 'progress.import', { name: 'voice.wav' }, {})).toBe(
      'Importing voice.wav',
    )
  })
  it('resolves supported regional system languages and falls back for unsupported languages', () => {
    expect(resolveLanguage(['fr-CA', 'en-US'])).toBe('fr')
    expect(resolveLanguage(['pt_BR'])).toBe('pt')
    expect(resolveLanguage(['zh-Hans-CN'])).toBe('zh-CN')
    expect(resolveLanguage(['ar-SA', 'pl-PL'])).toBe('pl')
    expect(resolveLanguage(['he-IL'])).toBe('en')
    vi.stubGlobal('navigator', { languages: ['de-DE'] })
    setLanguagePreference('system')
    expect(currentLanguage()).toBe('de')
    setLanguagePreference('es')
    expect(currentLanguage()).toBe('es')
    expect(validPreference('ar')).toBe(false)
    expect(validPreference('system')).toBe(true)
  })
  it('decodes backend keys and interpolates data without replacing user filenames or placeholders', () => {
    setLanguagePreference('ru')
    expect(translateMessage('[[error.trackLocked]]')).toBe(t('error.trackLocked'))
    const name = 'clip_{value1}_日本語.wav'
    const hex = Buffer.from(JSON.stringify({ name })).toString('hex')
    expect(translateMessage(`[[progress.import|${hex}]]`)).toBe(t('progress.import', { name }))
    expect(translateMessage('Native filesystem error')).toBe('Native filesystem error')
    expect(localizedName('track.voice')).toBe(t('track.voice'))
    expect(localizedName(catalogs.ru['track.voice'])).toBe(t('track.voice'))
    expect(localizedName('Custom track')).toBe('Custom track')
  })
  it('keeps Cyrillic text inside localization files', () => {
    const offenders = sourceFiles.filter((file) =>
      /\p{Script=Cyrillic}/u.test(readFileSync(file, 'utf8')),
    )
    expect(offenders).toEqual([])
  })
  it('covers every Rust localization key with the English fallback', () => {
    for (const file of sourceFiles.filter((file) => file.endsWith('.rs'))) {
      const source = readFileSync(file, 'utf8')
      const keys = [
        ...source.matchAll(/i18n::(?:message|formatted_message)\(\s*"([\w.]+)"/g),
        ...source.matchAll(/i18n::text\([^,]+,\s*"([\w.]+)"/g),
      ]
      for (const match of keys)
        expect(Object.hasOwn(catalogs.en, match[1]), `${file}:${match[1]}`).toBe(true)
    }
  })
})

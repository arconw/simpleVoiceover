import { useSyncExternalStore } from 'react'
import en from './locales/en.json'
import ru from './locales/ru.json'
import fr from './locales/fr.json'
import pl from './locales/pl.json'
import es from './locales/es.json'
import pt from './locales/pt.json'
import de from './locales/de.json'
import it from './locales/it.json'
import uk from './locales/uk.json'
import tr from './locales/tr.json'
import ja from './locales/ja.json'
import ko from './locales/ko.json'
import zh from './locales/zh-CN.json'
import languageNames from './locales/languages.json'

export const catalogs = { en, ru, fr, pl, es, pt, de, it, uk, tr, ja, ko, 'zh-CN': zh }
export type Language = keyof typeof catalogs
export type LanguagePreference = Language | 'system'
export type TranslationKey = keyof typeof en
export type TranslationParams = Record<string, string | number>
export const languages = Object.entries(languageNames) as [Language, string][]
const listeners = new Set<() => void>()
let preference: LanguagePreference = 'system'

export function resolveLanguage(candidates: readonly string[]): Language {
  for (const candidate of candidates) {
    const normalized = candidate.toLowerCase().replaceAll('_', '-')
    const exact = languages.find(([language]) => language.toLowerCase() === normalized)
    if (exact) return exact[0]
    const base = normalized.split('-')[0]
    if (base === 'zh') return 'zh-CN'
    const matching = languages.find(([language]) => language === base)
    if (matching) return matching[0]
  }
  return 'en'
}

export function systemLanguage(): Language {
  return resolveLanguage(typeof navigator === 'undefined' ? [] : navigator.languages)
}

export function currentLanguage(): Language {
  return preference === 'system' ? systemLanguage() : preference
}

export function validPreference(value: unknown): value is LanguagePreference {
  return value === 'system' || (typeof value === 'string' && Object.hasOwn(catalogs, value))
}

export function setLanguagePreference(value: LanguagePreference) {
  preference = value
  if (typeof document !== 'undefined') {
    document.documentElement.lang = currentLanguage()
    document.documentElement.dir = 'ltr'
    document.title = 'simpleVoiceover'
  }
  listeners.forEach((listener) => listener())
}

export function useLanguagePreference() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
    () => preference,
  )
}

export function translate(
  language: Language,
  key: TranslationKey,
  params: TranslationParams = {},
  catalog: Partial<Record<TranslationKey, string>> = catalogs[language],
): string {
  const text = catalog[key]?.trim() ? catalog[key] : en[key]
  return text.replace(/\{([A-Za-z0-9_]+)\}/g, (placeholder, name: string) =>
    Object.hasOwn(params, name) ? String(params[name]) : placeholder,
  )
}

export function t(key: TranslationKey, params: TranslationParams = {}): string {
  return translate(currentLanguage(), key, params)
}

export function localizedName(name: string): string {
  if (Object.hasOwn(en, name) && /^(track\.|project\.)/.test(name)) return t(name as TranslationKey)
  const key = (
    ['track.video', 'track.voice', 'track.music', 'track.audio', 'project.defaultName'] as const
  ).find((key) => name === ru[key])
  return key ? t(key) : name
}

export function translateMessage(message: string): string {
  return message.replace(
    /\[\[([A-Za-z0-9_.]+)(?:\|([0-9a-f]+))?\]\]/g,
    (encoded, key: string, hex?: string) => {
      if (!Object.hasOwn(en, key)) return encoded
      try {
        const params = hex
          ? JSON.parse(
              new TextDecoder().decode(
                Uint8Array.from(hex.match(/../g) ?? [], (byte) => Number.parseInt(byte, 16)),
              ),
            )
          : {}
        return t(key as TranslationKey, params)
      } catch {
        return t(key as TranslationKey)
      }
    },
  )
}

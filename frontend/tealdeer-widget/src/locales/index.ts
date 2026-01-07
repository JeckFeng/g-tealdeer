import { createI18n } from 'vue-i18n'
import en from './en.json'
import zh from './zh.json'

// Detect system language
function getDefaultLocale(): string {
  const browserLang = navigator.language.toLowerCase()
  if (browserLang.startsWith('zh')) return 'zh'
  return 'en'
}

// Load saved language preference
function getSavedLocale(): string {
  try {
    const saved = localStorage.getItem('app-language')
    if (saved && (saved === 'en' || saved === 'zh')) {
      return saved
    }
  } catch (e) {
    console.warn('Failed to load language preference:', e)
  }
  return getDefaultLocale()
}

const i18n = createI18n({
  legacy: false, // Use Composition API mode
  locale: getSavedLocale(), // Default language
  fallbackLocale: 'en', // Fallback language
  messages: {
    en,
    zh
  }
})

// Save language preference
export function saveLocale(locale: string) {
  try {
    localStorage.setItem('app-language', locale)
  } catch (e) {
    console.warn('Failed to save language preference:', e)
  }
}

export default i18n

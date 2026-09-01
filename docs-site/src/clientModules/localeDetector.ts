import ExecutionEnvironment from '@docusaurus/ExecutionEnvironment'

const STORAGE_KEY = 'ante_docs_preferred_locale'

function detectAndRedirect() {
  if (!ExecutionEnvironment.canUseDOM) {
    return
  }

  const { pathname, search, hash } = window.location

  // If already on Chinese path, mark preference as zh-Hans
  if (pathname === '/zh-Hans' || pathname.startsWith('/zh-Hans/')) {
    try {
      localStorage.setItem(STORAGE_KEY, 'zh-Hans')
    } catch {
      // Ignore storage errors (private browsing, etc.)
    }
    return
  }

  // User is on an English / default path
  try {
    const preferredLocale = localStorage.getItem(STORAGE_KEY)

    // User explicitly chose English before, do not redirect
    if (preferredLocale === 'en') {
      return
    }

    // User previously preferred zh-Hans, redirect
    if (preferredLocale === 'zh-Hans') {
      window.location.replace(`/zh-Hans${pathname}${search}${hash}`)
      return
    }

    // No stored preference: evaluate user languages in order of preference
    const languages =
      navigator.languages && navigator.languages.length > 0
        ? navigator.languages
        : [navigator.language || '']

    let matchedLocale: 'zh-Hans' | 'en' | null = null
    for (const lang of languages) {
      const lower = (lang || '').toLowerCase()
      if (lower.startsWith('zh')) {
        matchedLocale = 'zh-Hans'
        break
      }
      if (lower.startsWith('en')) {
        matchedLocale = 'en'
        break
      }
    }

    // If first matched supported language is Chinese, redirect to zh-Hans
    if (matchedLocale === 'zh-Hans') {
      localStorage.setItem(STORAGE_KEY, 'zh-Hans')
      window.location.replace(`/zh-Hans${pathname}${search}${hash}`)
    }
  } catch {
    // If localStorage or navigator access fails, gracefully continue in English
  }
}

// Run detection once DOM is ready on initial page load
if (ExecutionEnvironment.canUseDOM) {
  detectAndRedirect()
}

export function onRouteUpdate() {
  // Safe hook on client-side route transitions
}

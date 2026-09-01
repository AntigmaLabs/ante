import React from 'react'
import OriginalComponentTypes from '@theme-original/NavbarItem/ComponentTypes'
import LocaleDropdownNavbarItem from '@theme/NavbarItem/LocaleDropdownNavbarItem'

const STORAGE_KEY = 'ante_docs_preferred_locale'

function ConditionalLocaleDropdownNavbarItem(props: React.ComponentProps<typeof LocaleDropdownNavbarItem>) {
  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement | null
    // Only capture clicks on actual dropdown menu option links, not the dropdown trigger itself
    const anchor = target?.closest('a.dropdown__link, .dropdown__menu a') as HTMLAnchorElement | null
    if (!anchor) {
      return
    }

    const href = anchor.getAttribute('href') || ''
    const lang = (anchor.getAttribute('lang') || '').toLowerCase()

    // Ignore placeholder links or dropdown toggles
    if (!href || href === '#' || href.startsWith('javascript:')) {
      return
    }

    try {
      if (
        lang.startsWith('zh') ||
        href === '/zh-Hans' ||
        href.startsWith('/zh-Hans/')
      ) {
        localStorage.setItem(STORAGE_KEY, 'zh-Hans')
      } else {
        localStorage.setItem(STORAGE_KEY, 'en')
      }
    } catch {
      // Ignore storage errors
    }
  }

  return (
    <div onClick={handleClick} style={{ display: 'inline-flex' }}>
      <LocaleDropdownNavbarItem dropdownItemsBefore={[]} dropdownItemsAfter={[]} {...props} />
    </div>
  )
}

export default {
  ...OriginalComponentTypes,
  'custom-conditionalLocaleDropdown': ConditionalLocaleDropdownNavbarItem,
}

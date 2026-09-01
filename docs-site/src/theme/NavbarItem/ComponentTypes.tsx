import React from 'react'
import OriginalComponentTypes from '@theme-original/NavbarItem/ComponentTypes'
import LocaleDropdownNavbarItem from '@theme/NavbarItem/LocaleDropdownNavbarItem'

const STORAGE_KEY = 'ante_docs_preferred_locale'

function ConditionalLocaleDropdownNavbarItem(props: React.ComponentProps<typeof LocaleDropdownNavbarItem>) {
  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const target = e.target as HTMLElement | null
    const anchor = target?.closest('a')
    if (anchor) {
      const href = anchor.getAttribute('href') || ''
      try {
        if (href.startsWith('/zh-Hans/') || href === '/zh-Hans') {
          localStorage.setItem(STORAGE_KEY, 'zh-Hans')
        } else {
          localStorage.setItem(STORAGE_KEY, 'en')
        }
      } catch {
        // Ignore storage errors
      }
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

import React from 'react'
import { useLocation } from '@docusaurus/router'
import OriginalComponentTypes from '@theme-original/NavbarItem/ComponentTypes'
import LocaleDropdownNavbarItem from '@theme/NavbarItem/LocaleDropdownNavbarItem'

function ConditionalLocaleDropdownNavbarItem(props: React.ComponentProps<typeof LocaleDropdownNavbarItem>) {
  const { pathname } = useLocation()
  const isAntixPath = pathname === '/antix/introduction' ||
    pathname.startsWith('/antix/') ||
    pathname === '/zh-Hans/antix/introduction' ||
    pathname.startsWith('/zh-Hans/antix/')

  return isAntixPath
    ? <LocaleDropdownNavbarItem dropdownItemsBefore={[]} dropdownItemsAfter={[]} {...props} />
    : null
}

export default {
  ...OriginalComponentTypes,
  'custom-conditionalLocaleDropdown': ConditionalLocaleDropdownNavbarItem,
}

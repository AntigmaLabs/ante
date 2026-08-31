import React from 'react'
import OriginalComponentTypes from '@theme-original/NavbarItem/ComponentTypes'
import LocaleDropdownNavbarItem from '@theme/NavbarItem/LocaleDropdownNavbarItem'

function ConditionalLocaleDropdownNavbarItem(props: React.ComponentProps<typeof LocaleDropdownNavbarItem>) {
  return <LocaleDropdownNavbarItem dropdownItemsBefore={[]} dropdownItemsAfter={[]} {...props} />
}

export default {
  ...OriginalComponentTypes,
  'custom-conditionalLocaleDropdown': ConditionalLocaleDropdownNavbarItem,
}

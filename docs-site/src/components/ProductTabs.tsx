import React from 'react'
import Link from '@docusaurus/Link'
import { useLocation } from '@docusaurus/router'
import useDocusaurusContext from '@docusaurus/useDocusaurusContext'
import clsx from 'clsx'

/**
 * Second navbar row: one tab per product, like the section row on
 * code.claude.com. The product is derived from the URL so the tab stays
 * active across every page of that product's sidebar.
 */
export default function ProductTabs() {
  const { pathname } = useLocation()
  const {
    i18n: { currentLocale, defaultLocale },
  } = useDocusaurusContext()

  const isZh = currentLocale === 'zh-Hans'
  const prefix = currentLocale === defaultLocale ? '' : `/${currentLocale}`
  const path = pathname.startsWith(prefix) ? pathname.slice(prefix.length) : pathname
  const current = path.startsWith('/antix') ? 'antix' : 'ante'

  const tabs = [
    { id: 'ante', label: 'Ante', to: '/' },
    { id: 'antix', label: 'Antix', to: '/antix/introduction' },
  ]

  return (
    <div className="docs-product-tabs" role="navigation" aria-label="Products">
      {tabs.map((tab) => (
        <Link
          key={tab.id}
          to={tab.to}
          className={clsx('docs-product-tab', current === tab.id && 'docs-product-tab--active')}
          aria-current={current === tab.id ? 'page' : undefined}
        >
          {tab.label}
        </Link>
      ))}
      <Link href="https://antigma.ai/eval" className="docs-product-tab docs-product-tab--external">
        {isZh ? '实时评测' : 'Live Eval'}
        <span aria-hidden="true" className="docs-product-tab__arrow">↗</span>
      </Link>
    </div>
  )
}

import type { Config } from '@docusaurus/types'
import type * as Preset from '@docusaurus/preset-classic'

import { websiteIcon, discordIcon, githubIcon } from './src/components/icons'

const socialLinkHtml = (label: string, icon: string) =>
  `<span class="navbar-social-link"><span class="navbar-social-link__icon">${icon}</span><span>${label}</span></span>`

const isZhHansBuild = process.env.DOCUSAURUS_CURRENT_LOCALE === 'zh-Hans'

const docsFooterItems = isZhHansBuild
  ? [
      { label: 'Antix 简介', to: '/antix/introduction' },
      { label: '快速入门', to: '/antix/quickstart' },
      { label: '路由与 BYOK', to: '/antix/concepts/routing' },
      { label: '虚拟密钥与预算', to: '/antix/concepts/virtual-keys' },
      { label: '隐私与安全', to: '/antix/concepts/security' },
    ]
  : [
      { label: 'Overview', to: '/' },
      { label: 'Quickstart', to: '/start/quickstart' },
      { label: 'Benchmarks', to: '/benchmarks/eval' },
      { label: 'Local Models', to: '/local/overview' },
      { label: 'Providers', to: '/usage/providers' },
    ]

const legacyAnteRedirects = [
  { from: '/usage/offline', to: '/local/offline' },
  { from: '/experimental/offline', to: '/local/offline' },
  { from: '/configuration/providers', to: '/usage/providers' },
  { from: '/concepts/core-concepts', to: '/reference/core-concepts' },
  { from: '/concepts/architecture', to: '/reference/architecture' },
  { from: '/cookbook/login', to: '/usage/providers' },
  { from: '/usage/login', to: '/usage/providers' },
  { from: '/cookbook/providing-context', to: '/usage/providing-context' },
  { from: '/cookbook/models-and-thinking', to: '/usage/models-and-thinking' },
  { from: '/cookbook/steering', to: '/usage/steering' },
  { from: '/cookbook/approvals', to: '/usage/approvals' },
  { from: '/cookbook/web-browsing', to: '/usage/providing-context' },
  { from: '/usage/web-browsing', to: '/usage/providing-context' },
]

const config: Config = {
  title: 'Ante',
  tagline: 'a ghost in your shell: self-contained, self-organizing, benchmarked in public',
  favicon: 'assets/ante2.png',
  url: 'https://ante.run',
  baseUrl: '/',

  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'zh-Hans'],
    localeConfigs: {
      en: {
        label: 'English',
        htmlLang: 'en-US',
      },
      'zh-Hans': {
        label: '简体中文',
        htmlLang: 'zh-CN',
      },
    },
  },

  markdown: {
    mermaid: true,
  },
  themes: [
    '@docusaurus/theme-mermaid',
    [
      require.resolve('@easyops-cn/docusaurus-search-local'),
      {
        hashed: true,
        indexDocs: true,
        docsRouteBasePath: '/',
        language: ['en', 'zh'],
        highlightSearchTermsOnTargetPage: true,
        explicitSearchResultPath: true,
      },
    ],
  ],

  presets: [
    [
      'classic',
      {
        docs: {
          routeBasePath: '/',
          sidebarPath: './sidebars.ts',
          include: isZhHansBuild ? ['antix/**/*.{md,mdx}'] : ['**/*.{md,mdx}'],
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  // Soft-lands pre-restructure URLs. Old paths accumulate here; do not prune
  // entries just because the source page has moved again — chain them to the
  // current destination instead.
  plugins: [
    [
      '@docusaurus/plugin-client-redirects',
      {
        redirects: isZhHansBuild ? [] : legacyAnteRedirects,
      },
    ],
  ],

  themeConfig: {
    navbar: {
      title: 'Ante',
      logo: {
        alt: 'Ante',
        src: 'assets/ante.png',
        href: '/',
      },
      items: [
        {
          type: 'custom-conditionalLocaleDropdown',
          position: 'right',
          className: 'navbar-locale-switcher',
        },
        {
          type: 'docSidebar',
          sidebarId: 'antix',
          position: 'left',
          label: 'Antix',
        },
        {
          href: 'https://antigma.ai/eval',
          position: 'left',
          label: 'Live Eval',
        },
        {
          href: 'https://antigma.ai',
          html: socialLinkHtml('Home', websiteIcon),
          position: 'right',
        },
        {
          href: 'https://discord.gg/pqhj3DNGz2',
          html: socialLinkHtml('Discord', discordIcon),
          position: 'right',
        },
        {
          href: 'https://github.com/AntigmaLabs/ante',
          html: socialLinkHtml('GitHub', githubIcon),
          position: 'right',
        },
      ],
    },
    prism: {
      additionalLanguages: ['bash', 'json', 'rust', 'toml'],
    },
    colorMode: {
      defaultMode: 'light',
      respectPrefersColorScheme: true,
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: docsFooterItems,
        },
        {
          title: 'Community',
          items: [
            { label: 'Discord', href: 'https://discord.gg/pqhj3DNGz2' },
            { label: 'GitHub', href: 'https://github.com/AntigmaLabs/ante' },
          ],
        },
        {
          title: 'Company',
          items: [
            { label: 'Home', href: 'https://antigma.ai' },
            { label: 'Live Eval', href: 'https://antigma.ai/eval' },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Antigma Labs`,
    },
  } satisfies Preset.ThemeConfig,
}

export default config

import type { Config } from '@docusaurus/types'
import type * as Preset from '@docusaurus/preset-classic'

import { websiteIcon, discordIcon, githubIcon } from './src/components/icons'

const socialLinkHtml = (label: string, icon: string) =>
  `<span class="navbar-social-link"><span class="navbar-social-link__icon">${icon}</span><span>${label}</span></span>`

const isZhHansBuild = process.env.DOCUSAURUS_CURRENT_LOCALE === 'zh-Hans'

const docsFooterItems = isZhHansBuild
  ? [
      { label: '概览', to: '/' },
      { label: '快速入门', to: '/start/quickstart' },
      { label: '基准评测', to: '/benchmarks/eval' },
      { label: '本地模型', to: '/local/overview' },
      { label: '模型提供商', to: '/usage/providers' },
    ]
  : [
      { label: 'Overview', to: '/' },
      { label: 'Quickstart', to: '/start/quickstart' },
      { label: 'Benchmarks', to: '/benchmarks/eval' },
      { label: 'Local Models', to: '/local/overview' },
      { label: 'Providers', to: '/usage/providers' },
    ]

const footerCommunityItems = [
  { label: 'Discord', href: 'https://discord.gg/pqhj3DNGz2' },
  { label: 'GitHub', href: 'https://github.com/AntigmaLabs/ante' },
]

const footerCompanyItems = isZhHansBuild
  ? [
      { label: '官网', href: 'https://antigma.ai' },
      { label: '实时评测', href: 'https://antigma.ai/eval' },
    ]
  : [
      { label: 'Home', href: 'https://antigma.ai' },
      { label: 'Live Eval', href: 'https://antigma.ai/eval' },
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
          include: ['**/*.{md,mdx}'],
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
    // Generates llms.txt and llms-full.txt (llmstxt.org) at build time.
    // English build only: the plugin reads source markdown from docs/,
    // which the zh-Hans build neither includes nor serves.
    !isZhHansBuild && [
      'docusaurus-plugin-llms',
      {
        title: 'Ante',
        description:
          'Ante is a ghost in your shell: self-contained, self-organizing, benchmarked in public.',
        // Mirrors the docs preset's routeBasePath: '/' — without this the
        // plugin assumes docs are served under /docs and mislinks the root page.
        docsDir: [{ path: 'docs', routeBasePath: '/' }],
        includeOrder: [
          'start/**',
          'benchmarks/**',
          'usage/**',
          'local/**',
          'configuration/**',
          'extend/**',
          'reference/**',
          'experimental/**',
          'changelog*',
          'antix/**',
        ],
        includeUnmatchedLast: true,
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
          label: isZhHansBuild ? '实时评测' : 'Live Eval',
        },
        {
          href: 'https://antigma.ai',
          html: socialLinkHtml(isZhHansBuild ? '官网' : 'Home', websiteIcon),
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
          title: isZhHansBuild ? '文档' : 'Docs',
          items: docsFooterItems,
        },
        {
          title: isZhHansBuild ? '社区' : 'Community',
          items: footerCommunityItems,
        },
        {
          title: isZhHansBuild ? '公司' : 'Company',
          items: footerCompanyItems,
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Antigma Labs`,
    },
  } satisfies Preset.ThemeConfig,
}

export default config

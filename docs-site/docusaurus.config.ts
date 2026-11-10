import type { Config } from '@docusaurus/types'
import type * as Preset from '@docusaurus/preset-classic'
import { themes as prismThemes } from 'prism-react-renderer'

import { githubIcon } from './src/components/icons'

// Prism writes inline styles into the static HTML. Use the active palette
// there too so code is readable before React hydrates.
const prismPlain = {
  color: 'var(--ink-900)',
  backgroundColor: 'var(--ifm-code-background)',
}

const isZhHansBuild = process.env.DOCUSAURUS_CURRENT_LOCALE === 'zh-Hans'

const docsFooterItems = isZhHansBuild
  ? [
      { label: '概览', to: '/' },
      { label: '快速入门', to: '/start/quickstart' },
      { label: '基准评测', to: '/benchmarks/eval' },
      { label: '本地模型', to: '/local/overview' },
      { label: '连接提供商', to: '/usage/providers' },
    ]
  : [
      { label: 'Overview', to: '/' },
      { label: 'Quickstart', to: '/start/quickstart' },
      { label: 'Benchmarks', to: '/benchmarks/eval' },
      { label: 'Local Models', to: '/local/overview' },
      { label: 'Connect a provider', to: '/usage/providers' },
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

const changelogRedirect = {
  from: '/changelog',
  to: 'https://ante.run/changelog',
}

const config: Config = {
  title: 'Antigma Docs',
  tagline: 'Ante, the self-contained coding agent, and Antix, the platform behind it',
  favicon: 'assets/ante2.png',
  url: 'https://docs.antigma.ai',
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

  clientModules: [
    require.resolve('./src/clientModules/localeDetector.ts'),
  ],

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
        redirects: [
          ...(isZhHansBuild ? [] : legacyAnteRedirects),
          changelogRedirect,
        ],
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
          'antix/**',
        ],
        includeUnmatchedLast: true,
      },
    ],
  ],

  stylesheets: [
    { href: 'https://fonts.googleapis.com', rel: 'preconnect' },
    { href: 'https://fonts.gstatic.com', rel: 'preconnect', crossorigin: 'anonymous' },
    {
      href: 'https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@500;600;700&family=IBM+Plex+Mono:wght@400;500;600&display=swap',
      rel: 'stylesheet',
    },
  ],

  themeConfig: {
    navbar: {
      title: 'Antigma Docs',
      logo: {
        alt: 'Antigma',
        src: 'assets/antigma-logo.svg',
        href: '/',
      },
      items: [
        {
          type: 'custom-conditionalLocaleDropdown',
          position: 'left',
          className: 'navbar-locale-switcher',
        },
        {
          type: 'search',
          position: 'left',
          className: 'navbar-search',
        },
        {
          href: 'https://portal.antigma.ai',
          label: isZhHansBuild ? '打开 Antix Console' : 'Open Antix Console',
          position: 'right',
          className: 'navbar-cta',
        },
        {
          href: 'https://github.com/AntigmaLabs/ante',
          html: `<span class="navbar-icon-link" aria-label="GitHub" title="GitHub">${githubIcon}</span>`,
          position: 'right',
          className: 'navbar-github',
        },
      ],
    },
    prism: {
      theme: { ...prismThemes.github, plain: prismPlain },
      darkTheme: { ...prismThemes.dracula, plain: prismPlain },
      additionalLanguages: ['bash', 'json', 'rust', 'toml'],
    },
    colorMode: {
      defaultMode: 'light',
      respectPrefersColorScheme: true,
    },
    footer: {
      style: 'light',
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

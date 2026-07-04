import type { Config } from '@docusaurus/types'
import type * as Preset from '@docusaurus/preset-classic'

import { websiteIcon, discordIcon, githubIcon } from './src/components/icons'

const socialLinkHtml = (label: string, icon: string) =>
  `<span class="navbar-social-link"><span class="navbar-social-link__icon">${icon}</span><span>${label}</span></span>`

const config: Config = {
  title: 'Ante',
  tagline: 'a ghost in your shell: self-contained, self-organizing, benchmarked in public',
  favicon: 'assets/ante2.png',
  url: 'https://ante.run',
  baseUrl: '/',

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
        language: ['en'],
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
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  // Redirects are intentionally OFF. The @docusaurus/plugin-client-redirects
  // dependency is installed for future use only — do not remove it as "unused".
  // To soft-land the pre-restructure URLs, uncomment this block:
  //
  // plugins: [
  //   [
  //     '@docusaurus/plugin-client-redirects',
  //     {
  //       redirects: [
  //         { from: '/configuration/providers', to: '/usage/providers' },
  //         { from: '/experimental/offline', to: '/usage/offline' },
  //         { from: '/concepts/core-concepts', to: '/reference/core-concepts' },
  //         { from: '/concepts/architecture', to: '/reference/architecture' },
  //         { from: '/cookbook/login', to: '/usage/login' },
  //         { from: '/cookbook/providing-context', to: '/usage/providing-context' },
  //         { from: '/cookbook/models-and-thinking', to: '/usage/models-and-thinking' },
  //         { from: '/cookbook/steering', to: '/usage/steering' },
  //         { from: '/cookbook/approvals', to: '/usage/approvals' },
  //         { from: '/cookbook/web-browsing', to: '/usage/web-browsing' },
  //       ],
  //     },
  //   ],
  // ],

  themeConfig: {
    navbar: {
      title: 'Ante',
      logo: {
        alt: 'Ante',
        src: 'assets/ante.png',
      },
      items: [
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
          href: 'https://github.com/AntigmaLabs/ante-preview',
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
          items: [
            { label: 'Overview', to: '/' },
            { label: 'Quickstart', to: '/start/quickstart' },
            { label: 'Benchmarks', to: '/benchmarks/eval' },
            { label: 'Offline Mode', to: '/usage/offline' },
            { label: 'Providers', to: '/usage/providers' },
          ],
        },
        {
          title: 'Community',
          items: [
            { label: 'Discord', href: 'https://discord.gg/pqhj3DNGz2' },
            { label: 'GitHub', href: 'https://github.com/AntigmaLabs/ante-preview' },
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

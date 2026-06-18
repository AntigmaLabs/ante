import type { Config } from '@docusaurus/types'
import type * as Preset from '@docusaurus/preset-classic'

import { websiteIcon, discordIcon, githubIcon } from './src/components/icons'

const socialLinkHtml = (label: string, icon: string) =>
  `<span class="navbar-social-link"><span class="navbar-social-link__icon">${icon}</span><span>${label}</span></span>`

const config: Config = {
  title: 'Ante',
  tagline: 'ai-native, cloud-native, local-first agent runtime',
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
            { label: 'Configuration', to: '/configuration/providers' },
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
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Antigma Labs`,
    },
  } satisfies Preset.ThemeConfig,
}

export default config

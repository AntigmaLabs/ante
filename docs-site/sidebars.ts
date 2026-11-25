import type { SidebarsConfig } from '@docusaurus/plugin-content-docs'

const antixSidebar: SidebarsConfig[string] = [
  {
    type: 'category',
    label: 'Getting Started',
    collapsed: false,
    items: ['antix/introduction', 'antix/quickstart'],
  },
  {
    type: 'category',
    label: 'Core Concepts',
    collapsed: false,
    items: [
      'antix/concepts/routing',
      'antix/concepts/organizations',
      'antix/concepts/virtual-keys',
      'antix/concepts/endpoints',
      'antix/concepts/models',
      'antix/concepts/videos',
      'antix/concepts/analytics',
      'antix/concepts/error-handling',
      'antix/concepts/troubleshooting',
      'antix/concepts/security',
    ],
  },
  {
    type: 'category',
    label: 'Identity & Agents',
    collapsed: false,
    items: [
      'antix/concepts/ante-integration',
    ],
  },
]

const sidebars: SidebarsConfig = {
  docs: [
    {
      type: 'category',
      label: 'Getting Started',
      className: 'sidebar-section sidebar-section--start',
      collapsed: false,
      items: ['start/overview', 'start/quickstart', 'usage/providers', 'start/update'],
    },
    {
      type: 'category',
      label: 'Using the TUI',
      className: 'sidebar-section sidebar-section--use',
      link: { type: 'generated-index' },
      collapsed: false,
      items: [
        'usage/tui',
        'usage/slash-commands',
        'usage/providing-context',
        'usage/steering',
        'usage/approvals',
        'usage/goal-sessions',
        { type: 'doc', id: 'extend/memory', label: 'Auto Memory' },
      ],
    },
    {
      type: 'category',
      label: 'Models',
      className: 'sidebar-section sidebar-section--models',
      link: { type: 'generated-index' },
      collapsed: false,
      items: ['usage/models-and-thinking', 'usage/custom-sign-ins'],
    },
    {
      type: 'category',
      label: 'Local Models',
      className: 'sidebar-section sidebar-section--local',
      link: { type: 'generated-index' },
      collapsed: false,
      items: [
        'local/overview',
        'local/offline',
        'local/custom-engines',
        'local/existing-servers',
        'local/verified-models',
      ],
    },
    {
      type: 'category',
      label: 'Configure & Extend',
      className: 'sidebar-section sidebar-section--config',
      link: { type: 'generated-index' },
      collapsed: true,
      items: [
        'configuration/preference',
        'configuration/profiles',
        'configuration/permission',
        'extend/skills',
        'extend/subagents',
        'extend/mcp',
        'extend/agents-md',
      ],
    },
    {
      type: 'category',
      label: 'Build on Ante',
      className: 'sidebar-section sidebar-section--api',
      link: { type: 'generated-index' },
      collapsed: true,
      items: ['usage/headless', 'usage/serve', 'usage/sdk', 'usage/gateway'],
    },
    {
      type: 'category',
      label: 'Reference',
      className: 'sidebar-section sidebar-section--ref',
      link: { type: 'generated-index' },
      collapsed: true,
      items: [
        'reference/cli-reference',
        'reference/tui-reference',
        'reference/tools-reference',
        'reference/protocol-reference',
        'reference/catalog-reference',
        'reference/storage-reference',
        'reference/architecture',
      ],
    },
    {
      type: 'category',
      label: 'Why Ante',
      className: 'sidebar-section sidebar-section--why',
      link: { type: 'generated-index' },
      collapsed: true,
      items: ['benchmarks/eval', 'benchmarks/compare_table', 'start/philosophy'],
    },
    {
      type: 'category',
      label: 'Experimental',
      className: 'sidebar-section sidebar-section--exp',
      link: { type: 'generated-index' },
      collapsed: true,
      items: ['experimental/agent-org', 'experimental/agent-native-inference'],
    },
  ],
  antix: antixSidebar,
}

export default sidebars

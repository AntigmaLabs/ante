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
      'antix/concepts/error-handling',
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
      collapsible: false,
      items: [
        'start/overview',
        'start/quickstart',
        'start/update',
        'start/philosophy',
        {
          type: 'category',
          label: 'Benchmarks',
          collapsible: false,
          items: ['benchmarks/eval', 'benchmarks/compare_table'],
        },
      ],
    },
    {
      type: 'category',
      label: 'Using Ante',
      className: 'sidebar-section sidebar-section--use',
      collapsible: false,
      items: [
        'usage/providers',
        'usage/tui',
        'usage/slash-commands',
        {
          type: 'category',
          label: 'Input',
          collapsible: false,
          items: ['usage/providing-context', 'usage/steering'],
        },
        'usage/models-and-thinking',
        'usage/goal-sessions',
        'usage/approvals',
        { type: 'doc', id: 'extend/memory', label: 'Auto Memory' },
      ],
    },
    {
      type: 'category',
      label: 'Local Models',
      className: 'sidebar-section sidebar-section--local',
      collapsible: false,
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
      label: 'Configuration',
      className: 'sidebar-section sidebar-section--config',
      collapsed: true,
      items: [
        'configuration/preference',
        'configuration/permission',
        'configuration/coding-plan',
      ],
    },
    {
      type: 'category',
      label: 'Customization',
      className: 'sidebar-section sidebar-section--custom',
      collapsed: true,
      items: ['extend/skills', 'extend/subagents', 'extend/mcp', 'extend/agents-md'],
    },
    {
      type: 'category',
      label: 'Programmatic Usage',
      className: 'sidebar-section sidebar-section--api',
      collapsed: true,
      items: ['usage/headless', 'usage/serve', 'usage/acp-server', 'usage/gateway'],
    },
    {
      type: 'category',
      label: 'Reference',
      className: 'sidebar-section sidebar-section--ref',
      collapsed: true,
      items: [
        'reference/cli-reference',
        'reference/tools-reference',
        'reference/protocol-reference',
        'reference/catalog-reference',
        'reference/storage-reference',
        'reference/core-concepts',
        'reference/architecture',
      ],
    },
    {
      type: 'category',
      label: 'Experimental',
      className: 'sidebar-section sidebar-section--exp',
      collapsed: true,
      items: ['experimental/agent-org', 'experimental/agent-native-inference'],
    },
    'changelog',
  ],
  antix: antixSidebar,
}

export default process.env.DOCUSAURUS_CURRENT_LOCALE === 'zh-Hans'
  ? { antix: antixSidebar }
  : sidebars

import type { SidebarsConfig } from '@docusaurus/plugin-content-docs'

const sidebars: SidebarsConfig = {
  docs: [
    {
      type: 'category',
      label: 'Getting Started',
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
      collapsible: false,
      items: [
        'configuration/providers',
        'usage/tui',
        'cookbook/login',
        'cookbook/providing-context',
        'cookbook/models-and-thinking',
        'cookbook/steering',
        'cookbook/approvals',
        'cookbook/web-browsing',
        'experimental/offline',
      ],
    },
    {
      type: 'category',
      label: 'Configuration',
      collapsible: false,
      items: [
        'configuration/preference',
        'configuration/permission',
        'configuration/coding-plan',
      ],
    },
    {
      type: 'category',
      label: 'Customization',
      collapsible: false,
      items: ['extend/skills', 'extend/subagents', 'extend/mcp', 'extend/agents-md', 'extend/memory'],
    },
    {
      type: 'category',
      label: 'Programmatic Usage',
      collapsible: false,
      items: ['usage/headless', 'usage/serve', 'usage/gateway'],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsible: false,
      items: [
        'reference/cli-reference',
        'reference/tools-reference',
        'reference/protocol-reference',
        'reference/catalog-reference',
        'reference/storage-reference',
        'concepts/core-concepts',
        'concepts/architecture',
      ],
    },
    {
      type: 'category',
      label: 'Experimental',
      collapsible: false,
      items: ['experimental/agent-org'],
    },
    'changelog',
  ],
  antix: [
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
  ],
}

export default sidebars

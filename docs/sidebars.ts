import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  mainSidebar: [
    'introduction',
    {
      type: 'category',
      label: 'Getting Started',
      collapsed: false,
      items: [
        'start/introduction',
        'start/quickstart',
        'start/interactive-mode',
        'start/tui-cockpit',
        'start/daemon',
      ],
    },
    {
      type: 'category',
      label: 'Core Concepts',
      items: [
        'concepts/alive-loop',
        'concepts/cognitive-pipeline',
        'concepts/memory',
        'concepts/genome',
        'concepts/constitution',
        'concepts/the-equation',
      ],
    },
    {
      type: 'category',
      label: 'Extend Hydra',
      items: [
        'extend/skills',
        'extend/integrations',
        'extend/actions',
        'extend/vault',
      ],
    },
    {
      type: 'category',
      label: 'Capabilities',
      items: [
        'capabilities/thinking',
        'capabilities/judging',
        'capabilities/remembering',
        'capabilities/protecting',
        'capabilities/multiplying',
        'capabilities/growing',
        'capabilities/self-repair',
        'capabilities/web-omniscience',
        'capabilities/content-creation',
        'capabilities/video',
      ],
    },
    {
      type: 'category',
      label: 'Architecture',
      items: [
        'architecture/layers',
        'architecture/crates',
        'architecture/mathematics',
        'architecture/roadmap',
      ],
    },
    {
      type: 'category',
      label: 'Use Cases',
      items: [
        'use-cases/business',
        'use-cases/personal',
        'use-cases/developers',
        'use-cases/creators',
      ],
    },
  ],
};

export default sidebars;

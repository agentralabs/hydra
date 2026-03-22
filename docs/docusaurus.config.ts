import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Hydra',
  tagline: 'A living digital entity — 68 crates, 82,000 lines of Rust, one mind.',
  favicon: 'img/favicon.ico',
  url: 'https://hydra.agentralabs.com',
  baseUrl: '/',
  organizationName: 'agentralabs',
  projectName: 'hydra',
  onBrokenLinks: 'warn',
  onBrokenMarkdownLinks: 'warn',
  i18n: { defaultLocale: 'en', locales: ['en'] },
  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/agentralabs/hydra/tree/main/docs/',
          routeBasePath: '/', remarkPlugins: [], rehypePlugins: [],
        },
        blog: false,
        theme: { customCss: './src/css/custom.css' },
      } satisfies Preset.Options,
    ],
  ],
  themeConfig: {
    colorMode: { defaultMode: 'dark', disableSwitch: false, respectPrefersColorScheme: true },
    navbar: {
      title: 'Hydra',
      items: [
        { type: 'docSidebar', sidebarId: 'mainSidebar', position: 'left', label: 'Docs' },
        { href: 'https://github.com/agentralabs/hydra', label: 'GitHub', position: 'right' },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        { title: 'Docs', items: [
          { label: 'Quickstart', to: '/start/quickstart' },
          { label: 'Concepts', to: '/concepts/alive-loop' },
          { label: 'Extend', to: '/extend/skills' },
        ]},
        { title: 'Community', items: [
          { label: 'GitHub', href: 'https://github.com/agentralabs/hydra' },
          { label: 'Twitter', href: 'https://x.com/agentralabs' },
        ]},
      ],
      copyright: `© ${new Date().getFullYear()} Agentra Labs.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['rust', 'toml', 'bash'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;

import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Portaki SDK',
  description: 'Documentation officielle — modules catalogue, @portaki/sdk, @portaki/cli',
  lang: 'fr-FR',
  base: '/',
  head: [['link', { rel: 'icon', href: '/favicon.svg' }]],
  themeConfig: {
    logo: '/portaki-mark.svg',
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Modules', link: '/guide/module-authoring' },
      { text: 'API', link: '/api/index.html', target: '_blank' },
      { text: 'Hébergement', link: '/guide/hosting' },
      { text: 'GitHub', link: 'https://github.com/PortakiApp/portaki-sdk' },
    ],
    sidebar: [
      {
        text: 'Introduction',
        items: [
          { text: 'Démarrage', link: '/guide/getting-started' },
          { text: 'Documentation du code', link: '/guide/code-documentation' },
          { text: 'Hébergement (Vercel)', link: '/guide/hosting' },
          { text: 'Déploiement npm', link: '/guide/deployment' },
        ],
      },
      {
        text: 'Auteurs de modules',
        items: [
          { text: 'Créer un module', link: '/guide/module-authoring' },
          { text: 'E-mails transactionnels', link: '/guide/module-emails' },
          { text: 'Backend Wasm (AS)', link: '/guide/assemblyscript-backend' },
          { text: 'Exemple rules', link: '/guide/example-rules' },
        ],
      },
      {
        text: 'Référence',
        items: [
          { text: 'API TypeScript (TypeDoc)', link: '/api/index.html', target: '_blank' },
          { text: '@portaki/sdk-test-support', link: '/guide/test-support' },
        ],
      },
    ],
    socialLinks: [{ icon: 'github', link: 'https://github.com/PortakiApp/portaki-sdk' }],
    footer: {
      message: 'Portaki SDK — documentation officielle',
      copyright: 'Copyright © Portaki',
    },
    search: { provider: 'local' },
  },
})

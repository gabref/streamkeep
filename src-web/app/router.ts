import { createRouter, createWebHistory } from 'vue-router';

export const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('@/app/screens/HomeScreen.vue'),
    },
    {
      path: '/player',
      name: 'player',
      component: () => import('@/app/screens/PlayerScreen.vue'),
    },
    {
      path: '/detection',
      name: 'detection',
      component: () => import('@/app/screens/DetectionScreen.vue'),
    },
    {
      path: '/downloads',
      name: 'downloads',
      component: () => import('@/app/screens/DownloadsScreen.vue'),
    },
    {
      path: '/downloads/:jobId',
      name: 'download-detail',
      component: () => import('@/app/screens/DownloadDetailScreen.vue'),
      props: true,
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/app/screens/SettingsScreen.vue'),
    },
    {
      path: '/diagnostics',
      name: 'diagnostics',
      component: () => import('@/app/screens/DiagnosticsScreen.vue'),
    },
  ],
});


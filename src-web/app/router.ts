import { createRouter, createWebHistory } from 'vue-router';
import HomeScreen from '@/app/screens/HomeScreen.vue';

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: HomeScreen,
    },
  ],
});


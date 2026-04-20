import { createRouter, createWebHistory } from 'vue-router'
import ImportCenterView from '../views/ImportCenterView.vue'
import SystemSetupView from '../views/SystemSetupView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/setup',
    },
    {
      path: '/setup',
      name: 'setup',
      component: SystemSetupView,
    },
    {
      path: '/imports',
      name: 'imports',
      component: ImportCenterView,
    },
  ],
})

export default router

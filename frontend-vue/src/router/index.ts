import { createRouter, createWebHistory } from 'vue-router'
import ImportCenterView from '../views/ImportCenterView.vue'
import SystemSetupView from '../views/SystemSetupView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/setup',
      meta: {
        navLabel: '系统准备',
      },
    },
    {
      path: '/setup',
      name: 'setup',
      component: SystemSetupView,
      meta: {
        navLabel: '系统准备',
      },
    },
    {
      path: '/imports',
      name: 'imports',
      component: ImportCenterView,
      meta: {
        navLabel: '导入中心',
      },
    },
  ],
})

export default router

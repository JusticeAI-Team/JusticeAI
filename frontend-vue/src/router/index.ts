import { createRouter, createWebHistory } from 'vue-router'
import ImportCenterView from '../views/ImportCenterView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/imports',
    },
    {
      path: '/imports',
      name: 'imports',
      component: ImportCenterView,
    },
  ],
})

export default router

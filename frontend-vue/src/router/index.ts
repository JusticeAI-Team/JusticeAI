import { createRouter, createWebHistory } from 'vue-router'
import ImportCenterView from '../views/ImportCenterView.vue'
import NotFoundView from '../views/NotFoundView.vue'
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
    {
      path: '/:pathMatch(.*)*',
      name: 'not-found',
      component: NotFoundView,
      meta: {
        navLabel: '页面不存在',
      },
    },
  ],
})

router.afterEach((to) => {
  const pageLabel = typeof to.meta.navLabel === 'string' ? to.meta.navLabel : 'JusticeAI'
  document.title = `${pageLabel} - JusticeAI`

  requestAnimationFrame(() => {
    document.getElementById('main-content')?.focus()
  })
})

export default router

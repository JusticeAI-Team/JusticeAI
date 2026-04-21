import { createRouter, createWebHistory } from 'vue-router'
import DataIngestionView from '../views/DataIngestionView.vue'
import HomeDashboardView from '../views/HomeDashboardView.vue'
import ImportCenterView from '../views/ImportCenterView.vue'
import NotFoundView from '../views/NotFoundView.vue'
import ReportsView from '../views/ReportsView.vue'
import RiskAnalysisView from '../views/RiskAnalysisView.vue'
import SupervisionView from '../views/SupervisionView.vue'
import SystemSetupView from '../views/SystemSetupView.vue'
import WorkflowStageView from '../views/WorkflowStageView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: HomeDashboardView,
      meta: {
        navLabel: '流程总览',
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
      path: '/data-ingestion',
      name: 'data-ingestion',
      component: DataIngestionView,
      meta: {
        navLabel: '数据归集',
      },
    },
    {
      path: '/data-mapping',
      name: 'data-mapping',
      component: WorkflowStageView,
      props: {
        stepKey: 'data-mapping',
      },
      meta: {
        navLabel: '数据映射',
      },
    },
    {
      path: '/knowledge-extraction',
      name: 'knowledge-extraction',
      component: WorkflowStageView,
      props: {
        stepKey: 'knowledge-extraction',
      },
      meta: {
        navLabel: '知识抽取',
      },
    },
    {
      path: '/knowledge-graph',
      name: 'knowledge-graph',
      component: WorkflowStageView,
      props: {
        stepKey: 'knowledge-graph',
      },
      meta: {
        navLabel: '知识图谱',
      },
    },
    {
      path: '/risk-analysis',
      name: 'risk-analysis',
      component: RiskAnalysisView,
      meta: {
        navLabel: '风险研判',
      },
    },
    {
      path: '/alerts',
      name: 'alerts',
      component: WorkflowStageView,
      props: {
        stepKey: 'alerts',
      },
      meta: {
        navLabel: '预警中心',
      },
    },
    {
      path: '/case-dispatch',
      name: 'case-dispatch',
      component: WorkflowStageView,
      props: {
        stepKey: 'case-dispatch',
      },
      meta: {
        navLabel: '案件分派',
      },
    },
    {
      path: '/evaluation',
      name: 'evaluation',
      component: WorkflowStageView,
      props: {
        stepKey: 'evaluation',
      },
      meta: {
        navLabel: '成效评估',
      },
    },
    {
      path: '/supervision',
      name: 'supervision',
      component: SupervisionView,
      meta: {
        navLabel: '监督协调',
      },
    },
    {
      path: '/reports',
      name: 'reports',
      component: ReportsView,
      meta: {
        navLabel: '报告输出',
      },
    },
    {
      path: '/settings',
      name: 'settings',
      component: WorkflowStageView,
      props: {
        stepKey: 'settings',
      },
      meta: {
        navLabel: '平台设置',
      },
    },
    {
      path: '/imports',
      name: 'imports-compat',
      component: ImportCenterView,
      meta: {
        navLabel: '导入中心兼容入口',
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

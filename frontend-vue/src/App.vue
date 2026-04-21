<template>
  <div class="app-shell">
    <a class="skip-link" href="#main-content">跳到主要内容</a>

    <nav class="top-nav" aria-label="主导航">
      <div class="wrap nav-row">
        <div class="nav-brand-cluster">
          <RouterLink class="logo" to="/">
            <span class="mark"></span>
            JusticeAI
          </RouterLink>
          <div class="nav-edition">检察业务流程前端</div>
        </div>

        <div class="navlinks">
          <RouterLink v-for="item in navItems" :key="item.path" :to="item.path">{{ item.label }}</RouterLink>
        </div>

        <div class="navcta">
          <div class="nav-stage">
            <span class="kbd">{{ currentStageCode }}</span>
            <span class="nav-stage-label">{{ currentStageTitle }}</span>
          </div>
          <RouterLink class="btn primary" :to="actionLink.path">
            {{ actionLink.label }}
            <span class="arrow">→</span>
          </RouterLink>
        </div>
      </div>
    </nav>

    <main id="main-content" class="page-root" tabindex="-1">
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { workflowSteps } from './workflow/catalog'

const route = useRoute()

const navItems = [
  { label: '总览', path: '/' },
  { label: '系统准备', path: '/setup' },
  { label: '数据归集', path: '/data-ingestion' },
  { label: '风险研判', path: '/risk-analysis' },
  { label: '监督协调', path: '/supervision' },
  { label: '报告输出', path: '/reports' },
]

const currentStep = computed(() => workflowSteps.find((item) => item.path === route.path) ?? null)

const currentStageCode = computed(() => currentStep.value?.stageCode ?? 'OVR')
const currentStageTitle = computed(() => currentStep.value?.title ?? '流程总览')

const actionLink = computed(() => {
  if (route.path === '/') {
    return { label: '进入系统准备', path: '/setup' }
  }

  const currentIndex = workflowSteps.findIndex((item) => item.path === route.path)
  const nextStep = currentIndex >= 0 && currentIndex < workflowSteps.length - 1 ? workflowSteps[currentIndex + 1] : null

  if (nextStep) {
    return { label: `下一步 ${nextStep.title}`, path: nextStep.path }
  }

  return { label: '回到总览', path: '/' }
})
</script>

<style scoped>
.skip-link {
  position: absolute;
  left: 16px;
  top: -48px;
  z-index: 40;
  padding: 10px 14px;
  border-radius: 999px;
  background: var(--ink);
  color: var(--paper-2);
}

.skip-link:focus {
  top: 16px;
}

.nav-row {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 24px;
  align-items: center;
}

.nav-brand-cluster {
  display: flex;
  align-items: center;
  gap: 14px;
}

.nav-edition {
  color: var(--muted);
  font-family: var(--mono);
  font-size: 11px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
}

.nav-stage {
  display: flex;
  align-items: center;
  gap: 10px;
}

.nav-stage-label {
  color: var(--muted);
  font-size: 12px;
}

@media (max-width: 1080px) {
  .nav-row {
    grid-template-columns: 1fr;
    gap: 14px;
  }

  .nav-brand-cluster,
  .navcta {
    justify-content: space-between;
  }
}

@media (max-width: 640px) {
  .nav-edition,
  .nav-stage-label {
    display: none;
  }

  .nav-brand-cluster,
  .navcta {
    gap: 10px;
  }
}
</style>

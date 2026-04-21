<template>
  <aside class="sidebar">
    <RouterLink class="brand-card" to="/">
      <p class="eyebrow">JusticeAI / Workflow</p>
      <h2>完整流程骨架</h2>
      <p class="brand-summary">从系统准备、数据归集到监督报告，统一在一套中文亮色工作流界面内完成。</p>
    </RouterLink>

    <nav class="nav" aria-label="工作流导航">
      <RouterLink class="overview-link" :class="{ active: route.path === '/' }" to="/">
        <span class="overview-code">OVR</span>
        <span class="overview-text">流程总览</span>
      </RouterLink>

      <RouterLink
        v-for="step in workflowSteps"
        :key="step.key"
        class="step-link"
        :class="{ active: route.path === step.path }"
        :to="step.path"
      >
        <span class="step-code">{{ step.stageCode }}</span>
        <span class="step-meta">
          <strong>{{ step.title }}</strong>
          <span>{{ step.shortLabel }}</span>
        </span>
      </RouterLink>
    </nav>

    <div class="sidebar-note">
      旧版 <code>/imports</code> 已降级为兼容入口，并统一迁移到“数据归集”阶段。
    </div>
  </aside>
</template>

<script setup lang="ts">
import { RouterLink, useRoute } from 'vue-router'
import { workflowSteps } from '../../workflow/catalog'

const route = useRoute()
</script>

<style scoped>
.sidebar {
  position: sticky;
  top: 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.brand-card,
.sidebar-note,
.overview-link,
.step-link {
  border: 1px solid rgba(15, 23, 42, 0.1);
  background: rgba(255, 252, 247, 0.86);
  backdrop-filter: blur(18px);
  text-decoration: none;
  color: inherit;
}

.brand-card {
  padding: 20px;
  border-radius: 24px;
  box-shadow: 0 24px 60px rgba(15, 23, 42, 0.08);
}

.eyebrow {
  margin: 0 0 12px;
  font-size: 11px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #7c6f58;
}

.brand-card h2 {
  margin: 0;
  font-size: 28px;
  line-height: 1.05;
  font-weight: 500;
  font-family: 'Source Han Serif SC', 'Noto Serif SC', 'Songti SC', serif;
  color: #1f2937;
}

.brand-summary {
  margin: 14px 0 0;
  color: #5f6472;
  line-height: 1.7;
}

.nav {
  display: grid;
  gap: 10px;
}

.overview-link,
.step-link {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 12px;
  padding: 14px 16px;
  border-radius: 18px;
  transition:
    transform 0.2s ease,
    border-color 0.2s ease,
    box-shadow 0.2s ease;
}

.overview-link:hover,
.step-link:hover {
  transform: translateY(-1px);
  border-color: rgba(117, 137, 62, 0.36);
  box-shadow: 0 16px 32px rgba(15, 23, 42, 0.07);
}

.overview-link.active,
.step-link.active {
  border-color: rgba(117, 137, 62, 0.52);
  background: linear-gradient(135deg, rgba(255, 252, 247, 0.96), rgba(242, 247, 226, 0.94));
}

.overview-code,
.step-code {
  min-width: 42px;
  padding-top: 2px;
  font-size: 11px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: #7c6f58;
}

.overview-text,
.step-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.step-meta strong,
.overview-text {
  font-weight: 600;
  color: #111827;
}

.step-meta span {
  color: #6b7280;
  font-size: 13px;
  line-height: 1.55;
}

.sidebar-note {
  padding: 16px 18px;
  border-radius: 18px;
  color: #5f6472;
  line-height: 1.65;
}

code {
  padding: 2px 6px;
  border-radius: 999px;
  background: rgba(117, 137, 62, 0.09);
  color: #566a22;
}

@media (max-width: 960px) {
  .sidebar {
    position: static;
  }

  .nav {
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  }
}

@media (max-width: 640px) {
  .nav {
    grid-template-columns: 1fr;
  }
}
</style>

<template>
  <section class="hero">
    <div class="hero-copy">
      <p class="eyebrow">{{ eyebrow }}</p>
      <h1>{{ title }}</h1>
      <p class="summary">{{ summary }}</p>

      <div v-if="focus.length > 0" class="focus-list">
        <span v-for="item in focus" :key="item">{{ item }}</span>
      </div>
    </div>

    <aside class="hero-aside">
      <div class="meta-grid">
        <div class="meta-card">
          <span class="meta-label">当前步骤</span>
          <strong>{{ currentLabel }}</strong>
        </div>
        <div class="meta-card">
          <span class="meta-label">流程位置</span>
          <strong>{{ progressLabel }}</strong>
        </div>
        <div class="meta-card" :class="canProceed ? 'is-ready' : 'is-blocked'">
          <span class="meta-label">进入下一步</span>
          <strong>{{ gateLabel }}</strong>
          <p>{{ gateDescription }}</p>
        </div>
      </div>

      <div class="nav-actions">
        <RouterLink v-if="previousStep" class="ghost-link" :to="previousStep.path">
          上一步：{{ previousStep.label }}
        </RouterLink>
        <span v-else class="ghost-link disabled">上一步：无</span>

        <RouterLink v-if="nextStep" class="primary-link" :to="nextStep.path">
          下一步：{{ nextStep.label }}
        </RouterLink>
        <span v-else class="primary-link disabled">当前为流程尾部</span>
      </div>
    </aside>
  </section>
</template>

<script setup lang="ts">
import { RouterLink } from 'vue-router'
import type { WorkflowNavigationLink } from '../../workflow/catalog'

withDefaults(
  defineProps<{
    eyebrow: string
    title: string
    summary: string
    focus?: string[]
    currentLabel: string
    progressLabel: string
    gateLabel: string
    gateDescription: string
    canProceed: boolean
    previousStep?: WorkflowNavigationLink | null
    nextStep?: WorkflowNavigationLink | null
  }>(),
  {
    focus: () => [],
    previousStep: null,
    nextStep: null,
  },
)
</script>

<style scoped>
.hero {
  display: grid;
  grid-template-columns: minmax(0, 1.3fr) minmax(320px, 0.9fr);
  gap: 22px;
  padding: 28px;
  border: 1px solid rgba(15, 23, 42, 0.1);
  border-radius: 32px;
  background:
    linear-gradient(135deg, rgba(255, 252, 247, 0.98), rgba(243, 249, 232, 0.9)),
    linear-gradient(180deg, rgba(255, 255, 255, 0.98), rgba(252, 247, 239, 0.92));
  box-shadow: 0 30px 72px rgba(15, 23, 42, 0.1);
  overflow: hidden;
}

.eyebrow {
  margin: 0 0 14px;
  font-size: 11px;
  letter-spacing: 0.22em;
  text-transform: uppercase;
  color: #7c6f58;
}

.hero h1 {
  margin: 0;
  font-size: clamp(36px, 4vw, 58px);
  line-height: 0.98;
  letter-spacing: -0.03em;
  font-weight: 500;
  font-family: 'Source Han Serif SC', 'Noto Serif SC', 'Songti SC', serif;
  color: #111827;
}

.summary {
  margin: 18px 0 0;
  max-width: 720px;
  color: #525a68;
  font-size: 16px;
  line-height: 1.85;
}

.focus-list {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 22px;
}

.focus-list span {
  padding: 8px 12px;
  border: 1px solid rgba(117, 137, 62, 0.18);
  border-radius: 999px;
  background: rgba(117, 137, 62, 0.08);
  color: #4d5d1f;
  font-size: 13px;
}

.hero-aside {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.meta-grid {
  display: grid;
  gap: 12px;
}

.meta-card {
  padding: 18px;
  border-radius: 22px;
  border: 1px solid rgba(15, 23, 42, 0.08);
  background: rgba(255, 255, 255, 0.78);
}

.meta-card.is-ready {
  border-color: rgba(117, 137, 62, 0.28);
  background: rgba(243, 249, 232, 0.9);
}

.meta-card.is-blocked {
  border-color: rgba(194, 107, 73, 0.22);
  background: rgba(255, 245, 241, 0.92);
}

.meta-label {
  display: block;
  margin-bottom: 10px;
  font-size: 11px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: #7c6f58;
}

.meta-card strong {
  display: block;
  font-size: 18px;
  color: #111827;
}

.meta-card p {
  margin: 10px 0 0;
  color: #5f6472;
  line-height: 1.7;
}

.nav-actions {
  display: grid;
  gap: 10px;
}

.ghost-link,
.primary-link {
  display: inline-flex;
  justify-content: center;
  align-items: center;
  min-height: 46px;
  padding: 0 18px;
  border-radius: 999px;
  text-decoration: none;
  transition:
    transform 0.2s ease,
    box-shadow 0.2s ease,
    border-color 0.2s ease;
}

.ghost-link {
  border: 1px solid rgba(15, 23, 42, 0.12);
  color: #374151;
  background: rgba(255, 255, 255, 0.78);
}

.primary-link {
  border: 1px solid rgba(117, 137, 62, 0.32);
  color: #fffef9;
  background: linear-gradient(135deg, #75893e, #5d6f2c);
  box-shadow: 0 16px 28px rgba(93, 111, 44, 0.25);
}

.ghost-link:hover,
.primary-link:hover {
  transform: translateY(-1px);
}

.disabled {
  pointer-events: none;
  opacity: 0.56;
  box-shadow: none;
}

@media (max-width: 960px) {
  .hero {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 640px) {
  .hero {
    padding: 22px;
    border-radius: 28px;
  }
}
</style>

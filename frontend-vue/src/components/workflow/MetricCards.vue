<template>
  <div class="metric-grid">
    <article v-for="item in items" :key="`${item.label}-${item.value}`" class="metric-card" :class="toneClass(item.tone)">
      <p class="label">{{ item.label }}</p>
      <strong>{{ item.value }}</strong>
      <p v-if="item.hint" class="hint">{{ item.hint }}</p>
    </article>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  items: Array<{
    label: string
    value: string
    hint?: string
    tone?: string
  }>
}>()

function toneClass(tone?: string) {
  switch (tone) {
    case 'good':
      return 'tone-good'
    case 'warning':
      return 'tone-warning'
    case 'danger':
      return 'tone-danger'
    case 'accent':
      return 'tone-accent'
    default:
      return 'tone-neutral'
  }
}
</script>

<style scoped>
.metric-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 14px;
}

.metric-card {
  padding: 18px;
  border-radius: 22px;
  border: 1px solid rgba(15, 23, 42, 0.08);
  background: rgba(255, 255, 255, 0.7);
}

.label {
  margin: 0 0 14px;
  font-size: 12px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: #7c6f58;
}

.metric-card strong {
  display: block;
  font-size: 30px;
  line-height: 1.02;
  letter-spacing: -0.03em;
  color: #111827;
  font-weight: 600;
}

.hint {
  margin: 14px 0 0;
  color: #5f6472;
  line-height: 1.65;
}

.tone-good {
  border-color: rgba(117, 137, 62, 0.24);
  background: rgba(243, 249, 232, 0.9);
}

.tone-warning {
  border-color: rgba(194, 138, 54, 0.24);
  background: rgba(255, 248, 230, 0.92);
}

.tone-danger {
  border-color: rgba(194, 107, 73, 0.22);
  background: rgba(255, 244, 239, 0.92);
}

.tone-accent {
  border-color: rgba(117, 137, 62, 0.16);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.92), rgba(243, 249, 232, 0.82));
}
</style>

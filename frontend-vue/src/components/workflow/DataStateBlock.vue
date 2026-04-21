<template>
  <div class="state-block" :class="`kind-${kind}`">
    <div class="icon" aria-hidden="true">
      <span v-if="kind === 'loading'" class="spinner"></span>
      <span v-else>{{ icon }}</span>
    </div>

    <div class="copy">
      <p class="eyebrow">{{ eyebrow }}</p>
      <h3>{{ title }}</h3>
      <p>{{ description }}</p>
    </div>

    <RouterLink v-if="actionLabel && actionTo" class="action-link" :to="actionTo">
      {{ actionLabel }}
    </RouterLink>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'

const props = defineProps<{
  kind: 'loading' | 'disconnected' | 'empty'
  title: string
  description: string
  actionLabel?: string
  actionTo?: string
}>()

const eyebrow = computed(() => {
  if (props.kind === 'loading') {
    return '加载中'
  }

  if (props.kind === 'disconnected') {
    return '未接通'
  }

  return '空态占位'
})

const icon = computed(() => {
  if (props.kind === 'disconnected') {
    return '!'
  }

  return '·'
})
</script>

<style scoped>
.state-block {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 16px;
  align-items: center;
  padding: 22px;
  border-radius: 24px;
  border: 1px dashed rgba(15, 23, 42, 0.14);
  background: rgba(255, 255, 255, 0.56);
}

.icon {
  display: grid;
  place-items: center;
  width: 46px;
  height: 46px;
  border-radius: 50%;
  font-size: 22px;
  font-weight: 700;
}

.kind-loading .icon {
  background: rgba(117, 137, 62, 0.12);
  color: #566a22;
}

.kind-disconnected .icon {
  background: rgba(194, 107, 73, 0.12);
  color: #b25a36;
}

.kind-empty .icon {
  background: rgba(124, 111, 88, 0.12);
  color: #7c6f58;
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid rgba(117, 137, 62, 0.2);
  border-top-color: #566a22;
  border-radius: 50%;
  animation: spin 0.9s linear infinite;
}

.copy h3 {
  margin: 0;
  font-size: 20px;
  color: #111827;
}

.copy p {
  margin: 10px 0 0;
  color: #5f6472;
  line-height: 1.7;
}

.eyebrow {
  margin: 0 0 10px;
  font-size: 11px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: #7c6f58;
}

.action-link {
  grid-column: 2;
  justify-self: start;
  display: inline-flex;
  align-items: center;
  min-height: 40px;
  padding: 0 16px;
  border-radius: 999px;
  border: 1px solid rgba(117, 137, 62, 0.3);
  text-decoration: none;
  color: #4d5d1f;
  background: rgba(243, 249, 232, 0.88);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 640px) {
  .state-block {
    grid-template-columns: 1fr;
  }

  .action-link {
    grid-column: 1;
  }
}
</style>

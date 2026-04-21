<template>
  <div class="state-frame">
    <p class="eyebrow"><span class="dot"></span>{{ eyebrow }}</p>
    <h4>{{ title }}</h4>
    <p>{{ description }}</p>
    <div v-if="actionLabel && actionTo" class="inline-actions" style="margin-top: 18px">
      <RouterLink class="btn" :to="actionTo">{{ actionLabel }}</RouterLink>
    </div>
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

  return '空态'
})
</script>

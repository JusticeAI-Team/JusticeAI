<template>
  <div class="prov-grid">
    <RouterLink v-for="agent in agents" :key="agent.key" class="prov" :to="agent.path || '/supervision'">
      {{ agent.label }}
      <span class="tag">{{ agent.tagline }}</span>
    </RouterLink>
  </div>
</template>

<script setup lang="ts">
import { RouterLink } from 'vue-router'

defineProps<{
  agents: Array<{
    key: string
    label: string
    tagline: string
    path?: string
  }>
}>()
</script>

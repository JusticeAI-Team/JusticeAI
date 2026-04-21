<template>
  <div class="ticker">
    <div class="wrap">
      <div class="ticker-track">
        <span v-for="item in duplicatedItems" :key="item.key">
          {{ item.label }}
          <span class="bullet">•</span>
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  items: string[]
}>()

const duplicatedItems = computed(() => {
  const values = [...props.items, ...props.items]
  return values.map((label, index) => ({
    key: `${label}-${index}`,
    label,
  }))
})
</script>

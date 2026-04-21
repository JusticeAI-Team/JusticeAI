<template>
  <div class="diagram">
    <div class="diagram-row">
      <div class="node">
        <div class="label">{{ start.label }}</div>
        <div class="name">{{ start.name }}</div>
        <ul v-if="start.tags.length > 0">
          <li v-for="tag in start.tags" :key="`${start.name}-${tag}`">{{ tag }}</li>
        </ul>
      </div>

      <div class="conn"></div>

      <div class="node center">
        <div class="label">{{ current.label }}</div>
        <div class="name">{{ current.name }}</div>
        <ul v-if="current.tags.length > 0">
          <li v-for="tag in current.tags" :key="`${current.name}-${tag}`">{{ tag }}</li>
        </ul>
      </div>

      <div class="conn reverse"></div>

      <div class="node">
        <div class="label">{{ end.label }}</div>
        <div class="name">{{ end.name }}</div>
        <ul v-if="end.tags.length > 0">
          <li v-for="tag in end.tags" :key="`${end.name}-${tag}`">{{ tag }}</li>
        </ul>
      </div>
    </div>

    <div v-if="legend.length > 0" class="legend">
      <div v-for="item in legend" :key="item.key" class="item">
        <div class="k">{{ item.key }}</div>
        <div class="v">{{ item.value }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    start: { label: string; name: string; tags?: string[] }
    current: { label: string; name: string; tags?: string[] }
    end: { label: string; name: string; tags?: string[] }
    legend?: Array<{ key: string; value: string }>
  }>(),
  {
    legend: () => [],
  },
)
</script>

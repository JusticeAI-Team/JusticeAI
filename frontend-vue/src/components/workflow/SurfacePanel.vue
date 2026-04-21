<template>
  <section class="surface">
    <header v-if="hasHeader" class="surface-header">
      <div class="copy">
        <p v-if="eyebrow" class="eyebrow">{{ eyebrow }}</p>
        <h2 v-if="title" class="title">{{ title }}</h2>
        <p v-if="description" class="description">{{ description }}</p>
      </div>
      <div v-if="slots['header-extra']" class="header-extra">
        <slot name="header-extra" />
      </div>
    </header>

    <div class="surface-body">
      <slot />
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, useSlots } from 'vue'

const props = defineProps<{
  eyebrow?: string
  title?: string
  description?: string
}>()

const slots = useSlots()

const hasHeader = computed(() => Boolean(props.eyebrow || props.title || props.description || slots['header-extra']))
</script>

<style scoped>
.surface {
  border: 1px solid rgba(15, 23, 42, 0.1);
  border-radius: 28px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.98), rgba(255, 250, 243, 0.92)),
    rgba(255, 255, 255, 0.92);
  box-shadow: 0 26px 60px rgba(15, 23, 42, 0.08);
}

.surface-header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  padding: 24px 24px 0;
}

.copy {
  min-width: 0;
}

.eyebrow {
  margin: 0 0 10px;
  font-size: 11px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #7c6f58;
}

.title {
  margin: 0;
  font-size: 28px;
  line-height: 1.08;
  font-weight: 500;
  font-family: 'Source Han Serif SC', 'Noto Serif SC', 'Songti SC', serif;
  color: #111827;
}

.description {
  margin: 12px 0 0;
  color: #5f6472;
  line-height: 1.7;
}

.header-extra {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}

.surface-body {
  padding: 24px;
}

@media (max-width: 640px) {
  .surface-header,
  .surface-body {
    padding: 20px;
  }

  .surface-header {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>

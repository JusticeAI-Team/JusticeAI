<template>
  <div class="section-head">
    <div class="index">{{ index }}</div>
    <h3 class="title" v-html="title"></h3>
    <div class="lede">{{ lede }}</div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  index: string
  title: string
  lede: string
}>()
</script>

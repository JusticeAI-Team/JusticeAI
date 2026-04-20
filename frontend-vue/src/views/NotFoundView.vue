<template>
  <section class="page">
    <div class="panel">
      <p class="eyebrow">404</p>
      <h1>这个页面不存在</h1>
      <p class="hint">当前地址没有对应的界面入口，你可以先回到系统准备页，再进入导入中心。</p>
      <div class="actions">
        <RouterLink class="primary-link" to="/setup">前往系统准备页</RouterLink>
        <RouterLink class="secondary-link" to="/imports">前往导入中心</RouterLink>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { RouterLink } from 'vue-router'
</script>

<style scoped>
.page {
  color: #1f2937;
}

.panel {
  padding: 24px;
  border: 1px solid #dcdfe6;
  border-radius: 8px;
  background: #fff;
}

.eyebrow {
  margin: 0 0 8px;
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #c62828;
}

h1 {
  margin: 0 0 12px;
}

.hint {
  color: #666;
}

.actions {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 16px;
}

.primary-link,
.secondary-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 36px;
  padding: 0 14px;
  border-radius: 6px;
  text-decoration: none;
}

.primary-link {
  color: #fff;
  background: #2e7d32;
}

.secondary-link {
  border: 1px solid #d0d7de;
  color: #1f2937;
  background: #fff;
}
</style>

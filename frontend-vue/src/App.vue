<template>
  <div class="app-shell">
    <a class="skip-link" href="#main-content">跳到主要内容</a>
    <header class="topbar">
      <div>
        <p class="eyebrow">JusticeAI</p>
        <h1>本地联调控制台</h1>
      </div>
      <nav class="nav" aria-label="主导航">
        <RouterLink class="nav-link" to="/setup">系统准备</RouterLink>
        <RouterLink class="nav-link" to="/imports">导入中心</RouterLink>
      </nav>
    </header>

    <main id="main-content" class="content" tabindex="-1">
      <router-view />
    </main>
  </div>
</template>

<script setup lang="ts">
import { RouterLink } from 'vue-router'
</script>

<style scoped>
.app-shell {
  min-height: 100vh;
  background: #f5f7fa;
  color: #1f2937;
}

.skip-link {
  position: absolute;
  left: 16px;
  top: -48px;
  z-index: 10;
  padding: 8px 12px;
  border-radius: 8px;
  color: #fff;
  background: #2e7d32;
  text-decoration: none;
}

.skip-link:focus {
  top: 16px;
}

.topbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  padding: 20px 24px;
  border-bottom: 1px solid #dcdfe6;
  background: #ffffff;
  flex-wrap: wrap;
}

.topbar h1 {
  margin: 0;
  font-size: 24px;
}

.eyebrow {
  margin: 0 0 6px;
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #2e7d32;
}

.nav {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.nav-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 36px;
  padding: 0 14px;
  border: 1px solid #d0d7de;
  border-radius: 999px;
  color: #1f2937;
  text-decoration: none;
  background: #fff;
}

.nav-link.router-link-active {
  border-color: #2e7d32;
  color: #2e7d32;
  background: #e8f5e9;
}

.content {
  max-width: 1280px;
  margin: 0 auto;
  padding: 24px;
}
</style>

<template>
  <section>
    <header class="hero">
      <div class="wrap hero-grid">
        <div class="hero-meta" v-reveal="40">
          <div class="eyebrow"><span class="dot"></span>JusticeAI · 404</div>
          <div class="vol">当前地址不在主流程中</div>
        </div>

        <h1 class="headline" v-reveal="100">
          这个地址.<br />
          <em>没有对应页面.</em><br />
          请回到主流程.
        </h1>

        <p class="hero-sub" v-reveal="160">从总览、系统准备或数据归集重新进入。当前前端主线已经切到完整流程页，不再回退到旧结构。</p>

        <div class="hero-cta" v-reveal="220">
          <RouterLink class="btn primary" to="/">
            回到流程总览
            <span class="arrow">→</span>
          </RouterLink>
          <RouterLink class="btn" to="/setup">进入系统准备</RouterLink>
        </div>
      </div>
    </header>
  </section>
</template>

<script setup lang="ts">
import { RouterLink } from 'vue-router'
</script>

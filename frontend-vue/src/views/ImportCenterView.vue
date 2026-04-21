<template>
  <section>
    <header class="hero">
      <div class="wrap hero-grid">
        <div class="hero-meta" v-reveal="40">
          <div class="eyebrow"><span class="dot"></span>JusticeAI · 兼容入口</div>
          <div class="vol">旧 /imports 入口已迁移</div>
        </div>

        <h1 class="headline" v-reveal="100">
          旧导入中心.<br />
          <em>已并入数据归集.</em><br />
          正在跳转.
        </h1>

        <p class="hero-sub" v-reveal="160">上传、批次、兼容记录和导入详情已经统一放到数据归集页，这里只保留兼容跳转。</p>

        <div class="hero-cta" v-reveal="220">
          <RouterLink class="btn primary" to="/data-ingestion">
            立即进入数据归集
            <span class="arrow">→</span>
          </RouterLink>
        </div>
      </div>
    </header>
  </section>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { RouterLink, useRouter } from 'vue-router'

const router = useRouter()

onMounted(() => {
  window.setTimeout(() => {
    void router.replace('/data-ingestion')
  }, 320)
})
</script>

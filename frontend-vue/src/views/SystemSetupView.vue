<template>
  <main class="page">
    <header class="hero">
      <div>
        <p class="eyebrow">JusticeAI Setup</p>
        <h1>先确认后端与配置，再进入图形界面</h1>
        <p class="lead">
          前端不会自动拉起后端。请先按这里的说明启动后端、修改根目录 <code>.env</code>，再继续进入导入中心。
        </p>
      </div>
      <div class="hero-actions">
        <button type="button" :disabled="checking" @click="checkSystemStatus">
          {{ checking ? '检查中...' : '重新检查后端状态' }}
        </button>
        <RouterLink class="secondary-link" :class="{ disabled: !canOpenImportCenter }" :to="canOpenImportCenter ? '/imports' : '/setup'">
          进入导入中心
        </RouterLink>
      </div>
    </header>

    <section class="panel">
      <div class="section-header">
        <h2>后端连接状态</h2>
        <span class="badge" :class="backendStatusClass">{{ backendStatusLabel }}</span>
      </div>
      <p class="hint">{{ backendStatusDescription }}</p>
      <p v-if="systemError" class="error">{{ systemError }}</p>
      <p v-if="healthError" class="error">{{ healthError }}</p>
    </section>

    <section class="panel">
      <h2>手动启动方式</h2>
      <ol class="steps">
        <li>复制仓库根目录 <code>.env.example</code> 为 <code>.env</code>，按你的环境修改配置。</li>
        <li>先启动后端：<code>cargo run --manifest-path backend-rust/Cargo.toml</code></li>
        <li>再启动前端：<code>npm --prefix frontend-vue run dev</code></li>
        <li>确认本页状态为“已连接”或“部分可用”后，再进入导入中心。</li>
      </ol>
    </section>

    <section class="panel">
      <h2>配置文件位置</h2>
      <ul class="config-list">
        <li><code>.env</code>：推荐的统一本地配置入口</li>
        <li><code>.env.example</code>：根目录配置示例，包含前后端关键变量</li>
        <li><code>backend-rust/.env.example</code>：后端环境变量命名参考</li>
        <li><code>backend-rust/config/default.toml</code>：默认配置</li>
        <li><code>backend-rust/config/development.toml</code>：开发环境默认配置</li>
      </ul>
    </section>

    <section class="panel" v-if="systemInfo">
      <h2>当前运行配置</h2>
      <dl class="detail-grid">
        <div>
          <dt>应用名</dt>
          <dd>{{ systemInfo.app.name }}</dd>
        </div>
        <div>
          <dt>运行环境</dt>
          <dd>{{ systemInfo.app.env }}</dd>
        </div>
        <div>
          <dt>监听地址</dt>
          <dd>{{ systemInfo.app.host }}:{{ systemInfo.app.port }}</dd>
        </div>
        <div>
          <dt>运行版本</dt>
          <dd>{{ systemInfo.runtime.version }}</dd>
        </div>
        <div>
          <dt>数据库连接池</dt>
          <dd>{{ systemInfo.database.max_connections }} / 超时 {{ systemInfo.database.acquire_timeout_secs }}s</dd>
        </div>
        <div>
          <dt>vLLM 模型</dt>
          <dd>{{ systemInfo.vllm.model_name }}</dd>
        </div>
        <div>
          <dt>上传目录</dt>
          <dd>{{ systemInfo.storage.upload_dir }}</dd>
        </div>
        <div>
          <dt>报告目录</dt>
          <dd>{{ systemInfo.storage.report_dir }}</dd>
        </div>
        <div>
          <dt>训练目录</dt>
          <dd>{{ systemInfo.storage.training_dir }}</dd>
        </div>
      </dl>
    </section>

    <section class="panel" v-if="healthInfo">
      <h2>依赖健康状态</h2>
      <table class="table">
        <thead>
          <tr>
            <th>依赖</th>
            <th>状态</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>PostgreSQL</td>
            <td>{{ healthInfo.dependencies.postgres }}</td>
          </tr>
          <tr>
            <td>HugeGraph</td>
            <td>{{ healthInfo.dependencies.hugegraph }}</td>
          </tr>
          <tr>
            <td>vLLM</td>
            <td>{{ healthInfo.dependencies.vllm }}</td>
          </tr>
          <tr>
            <td>Milvus</td>
            <td>{{ healthInfo.dependencies.milvus }}</td>
          </tr>
        </tbody>
      </table>
      <ul v-if="healthInfo.notes.length > 0" class="notes">
        <li v-for="note in healthInfo.notes" :key="note">{{ note }}</li>
      </ul>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { fetchHealth, fetchSystemInfo, type HealthResponse, type SystemInfoResponse } from '../api/system'

const checking = ref(false)
const systemInfo = ref<SystemInfoResponse | null>(null)
const healthInfo = ref<HealthResponse | null>(null)
const systemError = ref('')
const healthError = ref('')

const canOpenImportCenter = computed(() => systemInfo.value !== null)
const backendStatusLabel = computed(() => {
  if (checking.value) {
    return '检查中'
  }

  if (!systemInfo.value) {
    return '未连接'
  }

  if (healthInfo.value?.status === 'degraded') {
    return '部分可用'
  }

  return '已连接'
})

const backendStatusClass = computed(() => {
  if (checking.value) {
    return 'status-waiting'
  }

  if (!systemInfo.value) {
    return 'status-down'
  }

  if (healthInfo.value?.status === 'degraded') {
    return 'status-degraded'
  }

  return 'status-up'
})

const backendStatusDescription = computed(() => {
  if (!systemInfo.value) {
    return '当前还没有连上后端。请先按下方命令手动启动后端，再点击“重新检查后端状态”。'
  }

  if (healthInfo.value?.status === 'degraded') {
    return '后端已经可访问，但部分依赖处于降级状态。你仍然可以进入导入中心继续联调。'
  }

  return '后端与核心依赖已经连通，可以继续进入导入中心。'
})

async function checkSystemStatus() {
  checking.value = true
  systemError.value = ''
  healthError.value = ''

  try {
    systemInfo.value = await fetchSystemInfo()
  } catch (error) {
    systemInfo.value = null
    healthInfo.value = null
    systemError.value = error instanceof Error ? error.message : '读取系统信息失败'
    checking.value = false
    return
  }

  try {
    healthInfo.value = await fetchHealth()
  } catch (error) {
    healthInfo.value = null
    healthError.value = error instanceof Error ? error.message : '读取健康状态失败'
  } finally {
    checking.value = false
  }
}

onMounted(() => {
  void checkSystemStatus()
})
</script>

<style scoped>
.page {
  padding: 24px;
  font-family: Arial, sans-serif;
  color: #1f2937;
}

.hero,
.panel {
  margin-bottom: 16px;
  padding: 16px;
  border: 1px solid #dcdfe6;
  border-radius: 8px;
  background: #fff;
}

.hero {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  flex-wrap: wrap;
}

.eyebrow {
  margin: 0 0 8px;
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #2e7d32;
}

.lead {
  max-width: 720px;
  color: #475569;
}

.hero-actions {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 220px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
  margin-bottom: 12px;
}

.badge {
  display: inline-flex;
  align-items: center;
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
}

.status-up {
  background: #e8f5e9;
  color: #2e7d32;
}

.status-degraded {
  background: #fff8e1;
  color: #b26a00;
}

.status-down {
  background: #ffebee;
  color: #c62828;
}

.status-waiting {
  background: #eef2ff;
  color: #3949ab;
}

.steps,
.config-list,
.notes {
  padding-left: 20px;
  color: #475569;
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
  margin: 0;
}

.detail-grid div {
  padding: 12px;
  border: 1px solid #ebeef5;
  border-radius: 8px;
  background: #f8fafc;
}

.detail-grid dt {
  color: #666;
  font-weight: 600;
}

.detail-grid dd {
  margin: 8px 0 0;
  word-break: break-all;
}

.table {
  width: 100%;
  border-collapse: collapse;
  font-size: 14px;
}

.table th,
.table td {
  padding: 10px 8px;
  border-bottom: 1px solid #ebeef5;
  text-align: left;
  vertical-align: top;
}

.hint {
  color: #666;
}

.error {
  color: #c62828;
}

.secondary-link {
  display: inline-flex;
  justify-content: center;
  align-items: center;
  min-height: 36px;
  padding: 0 12px;
  border: 1px solid #d0d7de;
  border-radius: 6px;
  color: #1f2937;
  text-decoration: none;
  background: #fff;
}

.secondary-link.disabled {
  pointer-events: none;
  opacity: 0.6;
}

button,
.secondary-link,
code {
  font: inherit;
}

code {
  padding: 2px 6px;
  border-radius: 4px;
  background: #f5f7fa;
}

button {
  min-height: 36px;
  padding: 0 12px;
  cursor: pointer;
}
</style>

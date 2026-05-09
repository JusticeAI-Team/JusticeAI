<template>
  <div class="ops-page">
    <header class="ops-top">
      <div>
        <div class="kicker">TECH OPS CONSOLE</div>
        <h2>技术运维后台</h2>
      </div>
      <div class="ops-actions">
        <button class="ghost-btn" @click="refreshAll" :disabled="loading">{{ loading ? '刷新中...' : '刷新状态' }}</button>
        <button class="primary-btn" @click="runProbe" :disabled="testing">{{ testing ? '探测中...' : '测试外部依赖' }}</button>
      </div>
    </header>

    <section class="overview-strip">
      <div class="overview-card">
        <span>后端状态</span>
        <strong :class="statusClassName(health?.status)">{{ statusText(health?.status) }}</strong>
      </div>
      <div class="overview-card">
        <span>运行环境</span>
        <strong>{{ health?.app_env || '--' }}</strong>
      </div>
      <div class="overview-card">
        <span>导入批次</span>
        <strong>{{ health?.data_overview?.import_batches ?? '--' }}</strong>
      </div>
      <div class="overview-card">
        <span>风险案件</span>
        <strong>{{ health?.data_overview?.risk_cases ?? '--' }}</strong>
      </div>
      <div class="overview-card">
        <span>最新检查</span>
        <strong>{{ shortTime(health?.timestamp) }}</strong>
      </div>
    </section>

    <div v-if="error" class="error-box">{{ error }}</div>

    <main class="ops-grid">
      <section class="panel deps-panel">
        <div class="panel-header">
          <span class="bar"></span>
          <div>
            <div class="kicker">DEPENDENCIES</div>
            <h3>外部服务联通状态</h3>
          </div>
        </div>
        <div class="dependency-grid">
          <div v-for="item in dependencies" :key="item.key" class="dependency-card">
            <div class="dep-head">
              <strong>{{ item.label }}</strong>
              <span :class="['badge', item.className]">{{ item.text }}</span>
            </div>
            <p class="endpoint">{{ item.endpoint }}</p>
            <p class="message">{{ item.message }}</p>
          </div>
        </div>
      </section>

      <section class="panel">
        <div class="panel-header">
          <span class="bar red"></span>
          <div>
            <div class="kicker">PIPELINE</div>
            <h3>真实闭环链路</h3>
          </div>
        </div>
        <div class="pipeline-list">
          <div v-for="item in pipeline" :key="item.name" class="pipeline-item">
            <span :class="['node-dot', item.state]"></span>
            <div>
              <strong>{{ item.name }}</strong>
              <p>{{ item.desc }}</p>
            </div>
          </div>
        </div>
      </section>

      <section class="panel">
        <div class="panel-header">
          <span class="bar orange"></span>
          <div>
            <div class="kicker">STORAGE</div>
            <h3>存储目录</h3>
          </div>
        </div>
        <div class="storage-list">
          <div v-for="item in health?.storage || []" :key="item.key" class="storage-item">
            <div>
              <strong>{{ item.key }}</strong>
              <p>{{ item.path }}</p>
            </div>
            <span :class="['badge', item.exists && item.writable ? 'ok' : 'bad']">
              {{ item.exists && item.writable ? '可写' : '需检查' }}
            </span>
          </div>
        </div>
      </section>

      <section class="panel command-panel">
        <div class="panel-header">
          <span class="bar green"></span>
          <div>
            <div class="kicker">LOCAL CONTAINERS</div>
            <h3>本机容器调试提示</h3>
          </div>
        </div>
        <div class="command-grid">
          <code v-for="command in localCommands" :key="command">{{ command }}</code>
        </div>
        <p class="hint">前端只做状态可视化和接口探测；容器启动/重建仍由本机运维命令执行，避免浏览器页面直接获得主机 Docker 控制权。</p>
      </section>

      <section class="panel command-panel">
        <div class="panel-header">
          <span class="bar"></span>
          <div>
            <div class="kicker">BACKGROUND JOBS</div>
            <h3>后台任务队列</h3>
          </div>
        </div>
        <div class="job-table">
          <div class="job-row head">
            <span>任务类型</span>
            <span>目标对象</span>
            <span>状态</span>
            <span>进度</span>
            <span>消息</span>
            <span>操作</span>
          </div>
          <div v-for="job in jobs" :key="job.id" class="job-row">
            <span class="mono">{{ job.job_type }}</span>
            <span class="mono">{{ job.target_id ? job.target_id.slice(0, 8) : '--' }}</span>
            <span :class="['badge', statusClass(job.status)]">{{ statusText(job.status) }}</span>
            <span class="mono">{{ job.progress_percent }}%</span>
            <span class="job-message">{{ job.error_message || job.message }}</span>
            <button class="mini-btn" :disabled="!canRetry(job) || retryingId === job.id" @click="retryJob(job)">
              {{ retryingId === job.id ? '重试中' : '重试' }}
            </button>
          </div>
          <div v-if="!jobs.length" class="empty-jobs">暂无后台任务记录</div>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { apiGet, apiPost, statusClass, statusText } from '../api/platform'

const loading = ref(false)
const testing = ref(false)
const error = ref('')
const health = ref(null)
const integrations = ref(null)
const probe = ref(null)
const jobs = ref([])
const retryingId = ref('')

const localCommands = [
  'docker ps --format "table {{.Names}}\\t{{.Status}}\\t{{.Ports}}"',
  'curl http://127.0.0.1:8000/v1/models',
  'curl http://127.0.0.1:7997/v1/embeddings',
  'curl http://127.0.0.1:8080/'
]

const statusClassName = (status) => statusClass(status)
const shortTime = (value) => (value ? new Date(value).toLocaleTimeString('zh-CN', { hour12: false }) : '--')

const refreshAll = async () => {
  loading.value = true
  try {
    const [healthResult, integrationResult, jobResult] = await Promise.all([
      apiGet('/health'),
      apiGet('/settings/integrations'),
      apiGet('/jobs?page_size=8')
    ])
    health.value = healthResult
    integrations.value = integrationResult
    jobs.value = jobResult?.items || []
    error.value = ''
  } catch (err) {
    error.value = err.message
  } finally {
    loading.value = false
  }
}

const runProbe = async () => {
  testing.value = true
  try {
    probe.value = await apiPost('/settings/integrations/test', {})
    error.value = ''
  } catch (err) {
    error.value = err.message
  } finally {
    testing.value = false
  }
}

const canRetry = (job) => ['failed', 'cancelled', 'completed_with_warnings'].includes(String(job.status || '').toLowerCase())

const retryJob = async (job) => {
  retryingId.value = job.id
  try {
    await apiPost(`/jobs/${job.id}/retry`, {})
    await refreshAll()
    error.value = ''
  } catch (err) {
    error.value = `任务重试失败：${err.message}`
  } finally {
    retryingId.value = ''
  }
}

const dependencies = computed(() => {
  const healthDetails = health.value?.dependency_details || []
  const current = probe.value || integrations.value || {}
  const mapped = [
    current.database,
    current.hugegraph,
    current.milvus,
    current.model_service,
    current.embedding_service
  ].filter(Boolean)

  if (mapped.length > 0) {
    return mapped.map((item) => ({
      key: item.key,
      label: labelFor(item.key),
      endpoint: item.endpoint || '--',
      message: item.message || '接口已纳入平台集成配置。',
      text: statusText(item.status),
      className: statusClass(item.status)
    }))
  }

  return healthDetails.map((item) => ({
    key: item.key,
    label: item.label,
    endpoint: item.endpoint,
    message: item.message,
    text: statusText(item.status),
    className: statusClass(item.status)
  }))
})

const labelFor = (key) => ({
  postgres: 'PostgreSQL',
  database: 'PostgreSQL',
  hugegraph: 'HugeGraph',
  milvus: 'Milvus',
  model_service: 'vLLM / ChatCompletion',
  embedding_service: 'Embedding Service'
}[key] || key)

const pipeline = computed(() => [
  { name: '多源表格导入', state: 'ok', desc: 'xlsx/xls/csv 上传后端落盘，当前已支持多 sheet 和截图 sheet 待 OCR 合同。' },
  { name: '标准案件生成', state: 'ok', desc: '中文字段别名映射为案件标题、地区、来源、状态、风险标签和时间。' },
  { name: 'AI 风险原因', state: health.value?.dependencies?.vllm === 'up' ? 'ok' : 'warn', desc: '通过 OpenAI-compatible ChatCompletion 调用 vLLM，失败时后端保留结构化降级。' },
  { name: 'HugeGraph 同步', state: health.value?.dependencies?.hugegraph === 'up' ? 'ok' : 'warn', desc: '抽取实体/关系后同步图数据库，并将同步状态写回案件。' },
  { name: 'Milvus 向量化', state: integrations.value?.embedding_service?.status === 'up' ? 'ok' : 'warn', desc: 'Embedding 输出写入 Milvus，支持相似案件召回辅助研判。' }
])

onMounted(refreshAll)
</script>

<style scoped>
.ops-page { height: 94vh; background: #F5EFEA; padding: 26px 30px; box-sizing: border-box; overflow: auto; color: #333; font-family: 'PingFang SC', 'Microsoft YaHei', sans-serif; }
.ops-top { display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px; }
.kicker { font-family: 'JetBrains Mono', Consolas, monospace; font-size: 11px; color: #8C98B0; font-weight: 900; letter-spacing: 1px; }
h2, h3 { margin: 4px 0 0; color: #122E8A; letter-spacing: 1px; }
.ops-actions { display: flex; gap: 10px; }
button { height: 36px; border-radius: 6px; padding: 0 14px; font-weight: 900; cursor: pointer; }
button:disabled { opacity: 0.65; cursor: not-allowed; }
.primary-btn { border: 1px solid #122E8A; background: #122E8A; color: #FFFFFF; }
.ghost-btn { border: 1px solid rgba(18, 46, 138, 0.22); background: #FFFFFF; color: #122E8A; }
.overview-strip { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px; margin-bottom: 14px; }
.overview-card { background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.12); border-radius: 8px; padding: 14px; box-shadow: 0 4px 12px rgba(18, 46, 138, 0.04); }
.overview-card span { color: #666; font-size: 12px; font-weight: bold; }
.overview-card strong { display: block; margin-top: 7px; color: #122E8A; font-size: 22px; font-family: 'JetBrains Mono', Consolas, monospace; }
.overview-card strong.ok { color: #0F7E3B; }
.overview-card strong.warn { color: #B56B00; }
.overview-card strong.bad { color: #D9363E; }
.error-box { margin-bottom: 14px; padding: 10px 14px; background: rgba(217, 54, 62, 0.08); border: 1px solid rgba(217, 54, 62, 0.2); color: #D9363E; border-radius: 6px; font-weight: bold; font-size: 12px; }
.ops-grid { display: grid; grid-template-columns: 1.2fr 0.8fr; gap: 16px; }
.panel { background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.14); border-radius: 8px; padding: 18px; box-shadow: 0 6px 18px rgba(18, 46, 138, 0.05); }
.deps-panel, .command-panel { grid-column: 1 / -1; }
.panel-header { display: flex; gap: 10px; align-items: center; margin-bottom: 16px; }
.bar { width: 4px; height: 30px; background: #122E8A; border-radius: 2px; }
.bar.red { background: #D9363E; }
.bar.orange { background: #F5A623; }
.bar.green { background: #52C41A; }
.dependency-grid { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px; }
.dependency-card { min-height: 132px; background: #FAFAFA; border: 1px solid rgba(18, 46, 138, 0.08); border-radius: 6px; padding: 12px; }
.dep-head { display: flex; justify-content: space-between; gap: 8px; align-items: center; }
.dep-head strong { color: #122E8A; }
.badge { border-radius: 999px; padding: 4px 9px; font-size: 11px; font-weight: 900; white-space: nowrap; }
.badge.ok { background: rgba(82, 196, 26, 0.12); color: #0F7E3B; }
.badge.warn { background: rgba(245, 166, 35, 0.14); color: #B56B00; }
.badge.bad { background: rgba(217, 54, 62, 0.12); color: #D9363E; }
.endpoint { color: #666; font-size: 11px; word-break: break-all; font-family: 'JetBrains Mono', Consolas, monospace; }
.message { color: #666; line-height: 1.6; font-size: 12px; }
.pipeline-list, .storage-list { display: grid; gap: 10px; }
.pipeline-item, .storage-item { display: flex; gap: 12px; align-items: flex-start; background: #FAFAFA; border: 1px solid rgba(18, 46, 138, 0.08); border-radius: 6px; padding: 12px; }
.storage-item { justify-content: space-between; align-items: center; }
.pipeline-item strong, .storage-item strong { color: #122E8A; }
.pipeline-item p, .storage-item p { margin: 5px 0 0; color: #666; font-size: 12px; line-height: 1.6; word-break: break-all; }
.node-dot { width: 10px; height: 10px; margin-top: 6px; border-radius: 50%; background: #F5A623; flex-shrink: 0; }
.node-dot.ok { background: #52C41A; }
.node-dot.warn { background: #F5A623; }
.command-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
code { display: block; background: #F5EFEA; border: 1px solid rgba(18, 46, 138, 0.12); border-radius: 6px; padding: 12px; color: #122E8A; font-family: 'JetBrains Mono', Consolas, monospace; font-size: 12px; white-space: pre-wrap; }
.hint { margin: 12px 0 0; color: #666; line-height: 1.7; font-size: 12px; }
.job-table { display: grid; gap: 8px; }
.job-row { display: grid; grid-template-columns: 1.1fr 0.8fr 80px 70px 1.8fr 76px; gap: 10px; align-items: center; padding: 10px 12px; background: #FAFAFA; border: 1px solid rgba(18, 46, 138, 0.08); border-radius: 6px; font-size: 12px; color: #333; }
.job-row.head { background: rgba(18, 46, 138, 0.05); color: #122E8A; font-weight: 900; }
.mono { font-family: 'JetBrains Mono', Consolas, monospace; color: #122E8A; font-weight: 900; }
.job-message { color: #666; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mini-btn { height: 28px; padding: 0 10px; border: 1px solid rgba(18, 46, 138, 0.22); background: #FFFFFF; color: #122E8A; border-radius: 6px; font-size: 12px; font-weight: 900; }
.empty-jobs { padding: 14px; background: #FAFAFA; border: 1px dashed rgba(18, 46, 138, 0.16); border-radius: 6px; color: #666; font-size: 12px; font-weight: bold; }
@media (max-width: 1180px) { .overview-strip, .dependency-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } .ops-grid, .command-grid { grid-template-columns: 1fr; } }
@media (max-width: 1180px) { .job-row { grid-template-columns: 1fr; } }
</style>

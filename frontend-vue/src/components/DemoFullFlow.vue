<template>
  <div class="demo-page">
    <header class="demo-top">
      <div>
        <div class="kicker">JUSTICEAI FULL FLOW</div>
        <h2>全流程闭环</h2>
        <p>一键生成闭环数据，覆盖导入、映射、处理、抽取、图谱/向量、风险、预警、分派与报告。</p>
      </div>
      <div class="demo-actions">
        <button class="ghost-btn" @click="refreshPreview" :disabled="loading || running">
          {{ loading ? '刷新中...' : '刷新看板数据' }}
        </button>
        <button class="primary-btn" @click="runDemo" :disabled="running">
          {{ running ? '正在生成闭环...' : '一键生成闭环数据' }}
        </button>
      </div>
    </header>

    <div v-if="error" class="error-box">{{ error }}</div>

    <section class="hero-panel">
      <div class="hero-left">
        <div class="hero-label">PRESENTATION READY</div>
        <h3>{{ demo ? '闭环数据已生成' : '准备生成通州治理风险数据' }}</h3>
        <p>
          后端会真实写入 PostgreSQL，并生成导入批次、映射模板、标准案件、知识实体、图谱关系、预警、处置任务和报告记录。
          Milvus 未启动时使用 <b>indexed_demo</b> 状态明确兜底，不伪装真实同步成功。
        </p>
        <div class="id-grid" v-if="demo">
          <div>
            <span>导入批次</span>
            <code>{{ demo.import_id }}</code>
          </div>
          <div>
            <span>抽取运行</span>
            <code>{{ demo.extraction_run_id }}</code>
          </div>
          <div>
            <span>研判报告</span>
            <code>{{ demo.report_id }}</code>
          </div>
        </div>
      </div>
      <div class="hero-right">
        <div class="big-number">{{ demo?.case_ids?.length || latestCases.length || '--' }}</div>
        <span>风险案件</span>
      </div>
    </section>

    <section class="metric-strip">
      <div class="metric-card" v-for="item in metricCards" :key="item.key">
        <span>{{ item.label }}</span>
        <strong>{{ item.value }}</strong>
        <em :class="statusClassName(item.status)">{{ statusText(item.status) }}</em>
      </div>
    </section>

    <main class="demo-grid">
      <section class="panel flow-panel">
        <div class="panel-header">
          <span class="bar"></span>
          <div>
            <div class="kicker">STAGES</div>
            <h3>业务链路</h3>
          </div>
        </div>
        <div class="flow-list">
          <div v-for="(stage, index) in stages" :key="stage.key" class="flow-item">
            <div class="stage-index">{{ String(index + 1).padStart(2, '0') }}</div>
            <div class="stage-body">
              <div class="stage-title">
                <strong>{{ stage.label }}</strong>
                <span :class="['badge', statusClassName(stage.status)]">{{ statusText(stage.status) }}</span>
              </div>
              <p>{{ stage.detail }}</p>
              <small>对象数：{{ stage.count }}</small>
            </div>
          </div>
        </div>
      </section>

      <section class="panel case-panel">
        <div class="panel-header">
          <span class="bar red"></span>
          <div>
            <div class="kicker">RISK CASES</div>
            <h3>最新风险案件</h3>
          </div>
        </div>
        <div class="case-list">
          <div v-for="item in latestCases" :key="item.id" class="case-item">
            <div class="case-main">
              <strong>{{ item.title }}</strong>
              <span>{{ item.case_code }} · {{ item.area_name }} · {{ sourceText(item.source_type) }}</span>
            </div>
            <div class="case-side">
              <b :class="riskClass(item.risk_level)">{{ riskText(item.risk_level) }}</b>
              <em>{{ Number(item.risk_score || 0).toFixed(1) }}</em>
            </div>
          </div>
          <div v-if="latestCases.length === 0" class="empty-state">点击“一键生成闭环数据”后展示风险案件。</div>
        </div>
      </section>

      <section class="panel notes-panel">
        <div class="panel-header">
          <span class="bar orange"></span>
          <div>
            <div class="kicker">TALK TRACK</div>
            <h3>讲解提示</h3>
          </div>
        </div>
        <div class="talk-list">
          <div v-for="note in notes" :key="note" class="talk-item">{{ note }}</div>
        </div>
      </section>

      <section class="panel route-panel">
        <div class="panel-header">
          <span class="bar green"></span>
          <div>
            <div class="kicker">API CONTRACT</div>
            <h3>可联动页面与接口</h3>
          </div>
        </div>
        <div class="route-grid">
          <code>POST /api/demo/full-flow</code>
          <code>GET /api/dashboard/overview</code>
          <code>GET /api/risk/cases?page_size=10</code>
          <code>GET /api/alerts</code>
          <code>GET /api/dispatch/tasks</code>
          <code>GET /api/reports?status=ready</code>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { apiGet, apiPost, statusClass, statusText } from '../api/platform'

const running = ref(false)
const loading = ref(false)
const error = ref('')
const demo = ref(null)
const dashboard = ref(null)
const latestCases = ref([])

const fallbackStages = [
  { key: 'readiness', label: '系统准备', status: 'not_checked', detail: '检查 PostgreSQL、vLLM、HugeGraph、Embedding、Milvus 等运行状态。', count: 0 },
  { key: 'ingestion', label: '数据导入', status: 'not_checked', detail: '生成通州 12345、110、信访、检察信访和 395 平台批次。', count: 0 },
  { key: 'mapping', label: '字段映射', status: 'not_checked', detail: '统一多分页 Excel 字段到标准案件、风险标签和处置字段。', count: 0 },
  { key: 'processing', label: '数据处理入库', status: 'not_checked', detail: '把原始记录转换为标准风险案件。', count: 0 },
  { key: 'extraction', label: '知识抽取', status: 'not_checked', detail: '生成实体、关系和抽取运行记录。', count: 0 },
  { key: 'graph_vector', label: '图谱/向量同步', status: 'not_checked', detail: '写入图谱同步状态和向量索引状态。', count: 0 },
  { key: 'risk', label: '风险研判', status: 'not_checked', detail: '生成风险等级、风险原因和处置建议。', count: 0 },
  { key: 'alerts', label: '预警管理', status: 'not_checked', detail: '生成可处理的预警记录。', count: 0 },
  { key: 'dispatch', label: '任务分派', status: 'not_checked', detail: '生成责任人、进度和反馈结果。', count: 0 },
  { key: 'report', label: '报告生成', status: 'not_checked', detail: '生成 Markdown 报告。', count: 0 }
]

const defaultNotes = [
  '建议讲解顺序：全流程闭环页 → 异构数据接入 → 线索审核与预警 → 技术运维后台。',
  '这套闭环数据会重复清理并重建 TZ-* 案件，适合会前反复彩排。',
  'Milvus 当前如果未启动，页面展示 indexed_fallback，技术运维后台仍会真实显示依赖状态。'
]

const stages = computed(() => demo.value?.stages?.length ? demo.value.stages : fallbackStages)
const notes = computed(() => demo.value?.notes?.length ? demo.value.notes : defaultNotes)

const metricCards = computed(() => {
  if (demo.value?.metrics?.length) {
    return demo.value.metrics.map((item) => ({
      key: item.key,
      label: labelForMetric(item.key, item.label),
      value: item.value,
      status: item.status
    }))
  }
  const metrics = dashboard.value?.metrics || []
  return [
    metricFromDashboard(metrics, 'import_batches', '导入批次'),
    metricFromDashboard(metrics, 'risk_cases', '风险案件'),
    metricFromDashboard(metrics, 'high_risk_cases', '高风险案件'),
    metricFromDashboard(metrics, 'pending_alerts', '待处理预警')
  ]
})

const metricFromDashboard = (metrics, key, label) => {
  const found = metrics.find((item) => item.key === key)
  return {
    key,
    label,
    value: found?.value ?? '--',
    status: found?.status || 'not_checked'
  }
}

const labelForMetric = (key, fallback) => ({
  demo_cases: '闭环案件',
  demo_entities: '抽取实体',
  demo_relations: '图谱关系',
  demo_report: '生成报告'
}[key] || fallback || key)

const statusClassName = (status) => statusClass(status)

const sourceText = (sourceType) => ({
  hotline_12345: '12345热线',
  police_110: '110接警',
  petitions: '综治信访',
  procuratorate_petition: '检察信访',
  platform_395: '395平台',
  demo_full_flow: '闭环数据包'
}[sourceType] || sourceType || '--')

const riskText = (level) => ({
  high: '高风险',
  medium: '中风险',
  low: '低风险'
}[level] || level || '--')

const riskClass = (level) => `risk-${level || 'unknown'}`

const refreshPreview = async () => {
  loading.value = true
  try {
    const [dashboardResult, riskResult] = await Promise.all([
      apiGet('/dashboard/overview'),
      apiGet('/risk/cases?page_size=8')
    ])
    dashboard.value = dashboardResult
    latestCases.value = riskResult?.items || []
    error.value = ''
  } catch (err) {
    error.value = err.message
  } finally {
    loading.value = false
  }
}

const runDemo = async () => {
  running.value = true
  try {
    demo.value = await apiPost('/demo/full-flow', {})
    await refreshPreview()
    ElMessage.success('闭环数据已生成，可开始讲解。')
  } catch (err) {
    error.value = err.message
    ElMessage.error(err.message)
  } finally {
    running.value = false
  }
}

onMounted(refreshPreview)
</script>

<style scoped>
.demo-page { height: 94vh; background: #F5EFEA; padding: 26px 30px; box-sizing: border-box; overflow: auto; color: #333; font-family: 'PingFang SC', 'Microsoft YaHei', sans-serif; }
.demo-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 18px; margin-bottom: 16px; }
.kicker { font-family: 'JetBrains Mono', Consolas, monospace; font-size: 11px; color: #8C98B0; font-weight: 900; letter-spacing: 1px; }
h2, h3 { margin: 4px 0 0; color: #122E8A; letter-spacing: 1px; }
.demo-top p { margin: 8px 0 0; color: #666; font-size: 13px; line-height: 1.7; }
.demo-actions { display: flex; gap: 10px; flex-shrink: 0; }
button { height: 38px; border-radius: 6px; padding: 0 15px; font-weight: 900; cursor: pointer; }
button:disabled { opacity: 0.65; cursor: not-allowed; }
.primary-btn { border: 1px solid #122E8A; background: #122E8A; color: #FFFFFF; box-shadow: 0 8px 18px rgba(18, 46, 138, 0.18); }
.ghost-btn { border: 1px solid rgba(18, 46, 138, 0.22); background: #FFFFFF; color: #122E8A; }
.error-box { margin-bottom: 14px; padding: 10px 14px; background: rgba(217, 54, 62, 0.08); border: 1px solid rgba(217, 54, 62, 0.2); color: #D9363E; border-radius: 6px; font-weight: bold; font-size: 12px; }
.hero-panel { display: grid; grid-template-columns: 1fr 260px; gap: 16px; background: linear-gradient(135deg, #FFFFFF 0%, #F8FBFF 55%, #F5EFEA 100%); border: 1px solid rgba(18, 46, 138, 0.14); border-radius: 8px; padding: 22px; box-shadow: 0 8px 22px rgba(18, 46, 138, 0.07); margin-bottom: 14px; position: relative; overflow: hidden; }
.hero-panel::after { content: ''; position: absolute; right: -80px; top: -90px; width: 240px; height: 240px; border-radius: 50%; background: rgba(18, 46, 138, 0.06); }
.hero-label { display: inline-flex; font-family: 'JetBrains Mono', Consolas, monospace; font-weight: 900; font-size: 11px; color: #D9363E; border: 1px solid rgba(217, 54, 62, 0.22); border-radius: 999px; padding: 4px 10px; background: rgba(217, 54, 62, 0.06); }
.hero-left p { color: #555; line-height: 1.8; font-size: 13px; max-width: 880px; }
.id-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; margin-top: 14px; }
.id-grid div { background: rgba(255, 255, 255, 0.72); border: 1px solid rgba(18, 46, 138, 0.1); border-radius: 6px; padding: 10px; }
.id-grid span { display: block; color: #666; font-size: 12px; font-weight: bold; margin-bottom: 6px; }
code { display: block; color: #122E8A; font-family: 'JetBrains Mono', Consolas, monospace; font-size: 11px; word-break: break-all; }
.hero-right { position: relative; z-index: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; background: #122E8A; border-radius: 8px; color: #FFFFFF; min-height: 150px; }
.big-number { font-family: 'JetBrains Mono', Consolas, monospace; font-size: 58px; font-weight: 900; line-height: 1; }
.hero-right span { margin-top: 8px; font-weight: 900; letter-spacing: 1px; }
.metric-strip { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin-bottom: 14px; }
.metric-card { background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.12); border-radius: 8px; padding: 14px; box-shadow: 0 4px 12px rgba(18, 46, 138, 0.04); }
.metric-card span { color: #666; font-size: 12px; font-weight: bold; }
.metric-card strong { display: block; margin-top: 7px; color: #122E8A; font-size: 26px; font-family: 'JetBrains Mono', Consolas, monospace; }
.metric-card em { display: inline-block; margin-top: 8px; font-style: normal; font-size: 11px; font-weight: 900; }
.metric-card em.ok { color: #0F7E3B; }
.metric-card em.warn { color: #B56B00; }
.metric-card em.bad { color: #D9363E; }
.demo-grid { display: grid; grid-template-columns: 1.2fr 0.8fr; gap: 16px; }
.panel { background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.14); border-radius: 8px; padding: 18px; box-shadow: 0 6px 18px rgba(18, 46, 138, 0.05); }
.panel-header { display: flex; gap: 10px; align-items: center; margin-bottom: 16px; }
.bar { width: 4px; height: 30px; background: #122E8A; border-radius: 2px; }
.bar.red { background: #D9363E; }
.bar.orange { background: #F5A623; }
.bar.green { background: #52C41A; }
.flow-panel { grid-row: span 2; }
.flow-list { display: grid; gap: 10px; }
.flow-item { display: grid; grid-template-columns: 46px 1fr; gap: 12px; align-items: stretch; }
.stage-index { display: flex; align-items: center; justify-content: center; border-radius: 6px; color: #122E8A; background: rgba(18, 46, 138, 0.07); font-family: 'JetBrains Mono', Consolas, monospace; font-weight: 900; }
.stage-body { border: 1px solid rgba(18, 46, 138, 0.08); border-radius: 6px; background: #FAFAFA; padding: 12px; }
.stage-title { display: flex; justify-content: space-between; gap: 10px; align-items: center; }
.stage-title strong { color: #122E8A; }
.stage-body p { margin: 7px 0; color: #666; font-size: 12px; line-height: 1.7; }
.stage-body small { color: #8C98B0; font-family: 'JetBrains Mono', Consolas, monospace; }
.badge { border-radius: 999px; padding: 4px 9px; font-size: 11px; font-weight: 900; white-space: nowrap; }
.badge.ok { background: rgba(82, 196, 26, 0.12); color: #0F7E3B; }
.badge.warn { background: rgba(245, 166, 35, 0.14); color: #B56B00; }
.badge.bad { background: rgba(217, 54, 62, 0.12); color: #D9363E; }
.case-list, .talk-list { display: grid; gap: 10px; }
.case-item { display: flex; align-items: center; justify-content: space-between; gap: 12px; border: 1px solid rgba(18, 46, 138, 0.08); background: #FAFAFA; border-radius: 6px; padding: 12px; }
.case-main strong { display: block; color: #122E8A; font-size: 13px; line-height: 1.5; }
.case-main span { display: block; margin-top: 5px; color: #666; font-size: 11px; }
.case-side { text-align: right; flex-shrink: 0; }
.case-side b { display: block; font-size: 12px; }
.case-side em { display: block; margin-top: 4px; color: #122E8A; font-family: 'JetBrains Mono', Consolas, monospace; font-style: normal; font-weight: 900; }
.risk-high { color: #D9363E; }
.risk-medium { color: #B56B00; }
.risk-low { color: #0F7E3B; }
.empty-state { color: #8C98B0; border: 1px dashed rgba(18, 46, 138, 0.16); border-radius: 6px; padding: 18px; text-align: center; font-size: 12px; }
.talk-item { color: #555; background: #FAFAFA; border: 1px solid rgba(18, 46, 138, 0.08); border-left: 4px solid #F5A623; border-radius: 6px; padding: 12px; line-height: 1.7; font-size: 12px; }
.route-panel { grid-column: 1 / -1; }
.route-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; }
.route-grid code { background: #F5EFEA; border: 1px solid rgba(18, 46, 138, 0.12); border-radius: 6px; padding: 12px; }
@media (max-width: 1180px) { .hero-panel, .demo-grid { grid-template-columns: 1fr; } .metric-strip, .id-grid, .route-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } .flow-panel { grid-row: auto; } }
@media (max-width: 760px) { .demo-top, .demo-actions { flex-direction: column; align-items: stretch; } .metric-strip, .id-grid, .route-grid { grid-template-columns: 1fr; } }
</style>

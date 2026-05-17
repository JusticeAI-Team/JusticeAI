<template>
  <div class="appeal-workbench">
    <section class="metric-strip">
      <article v-for="metric in metrics" :key="metric.label" class="metric-card">
        <div class="metric-icon" :class="metric.tone">{{ metric.icon }}</div>
        <div>
          <span>{{ metric.label }}</span>
          <strong>{{ metric.value }}</strong>
          <small>{{ metric.delta }}</small>
        </div>
      </article>
    </section>

    <section class="flow-strip">
      <div class="flow-title">农民工欠薪诉求闭环</div>
      <div v-for="step in workflowSteps" :key="step.name" class="flow-step active">
        <span>{{ step.icon }}</span>
        <strong>{{ step.name }}</strong>
        <small>{{ step.desc }}</small>
      </div>
    </section>

    <section class="main-grid">
      <aside class="queue-column">
        <div class="panel-card queue-panel">
          <header class="panel-header">
            <div>
              <strong>移动端线索队列</strong>
              <span>{{ total }} 条</span>
            </div>
            <button @click="loadAppeals">刷新</button>
          </header>

          <div class="search-row">
            <input v-model="keyword" placeholder="欠薪 / XX区 / 项目 / 手机号" @keydown.enter="loadAppeals" />
            <button @click="loadAppeals">筛选</button>
          </div>

          <div class="status-tabs">
            <button :class="{ active: !statusFilter }" @click="setStatus('')">全部</button>
            <button :class="{ active: statusFilter === 'submitted_incomplete' }" @click="setStatus('submitted_incomplete')">材料不全</button>
            <button :class="{ active: statusFilter === 'material_requested' }" @click="setStatus('material_requested')">待补证</button>
            <button :class="{ active: statusFilter === 'accepted' }" @click="setStatus('accepted')">已接收</button>
          </div>

          <button
            v-for="item in appeals"
            :key="item.id"
            class="case-item"
            :class="{ active: item.id === activeId }"
            @click="openDetail(item.id)"
          >
            <div class="risk-badge" :class="scoreTone(item.material_score)">{{ item.material_score }}</div>
            <div class="case-main">
              <strong>{{ item.project_name || '北京市XX区XX地点欠薪线索' }}</strong>
              <span>{{ item.worker_name }} · {{ item.worker_phone }} · {{ item.area_name || '待确认区域' }}</span>
              <small>{{ item.appeal_code }} · {{ statusText(item.status) }}</small>
            </div>
            <em>{{ item.risk_case_status }}</em>
          </button>
        </div>

        <div class="panel-card map-panel">
          <header class="panel-header compact">
            <div>
              <strong>通州区线索分布图</strong>
              <span>保留原风险地图能力</span>
            </div>
          </header>
          <div ref="mapRef" class="tongzhou-map"></div>
          <div class="map-legend">
            <span><i class="red"></i>欠薪高关注</span>
            <span><i class="blue"></i>普通诉求</span>
            <span><i class="orange"></i>材料待补</span>
          </div>
        </div>
      </aside>

      <main class="review-column">
        <div class="panel-card review-panel" v-if="detail">
          <header class="review-header">
            <div>
              <span class="kicker">LABOR APPEAL CASE FILE</span>
              <h2>{{ detail.appeal.project_name || '北京市XX区XX地点欠薪线索' }}</h2>
              <p>{{ detail.appeal.appeal_code }} · {{ statusText(detail.appeal.status) }}</p>
            </div>
            <div class="header-actions">
              <button @click="requestMaterials">要求补证</button>
              <button class="primary" @click="acceptAppeal">接收线索</button>
            </div>
          </header>

          <nav class="doc-tabs">
            <button :class="{ active: activeTab === 'summary' }" @click="activeTab = 'summary'">线索摘要</button>
            <button :class="{ active: activeTab === 'standard' }" @click="activeTab = 'standard'">智能标准化</button>
            <button :class="{ active: activeTab === 'raw' }" @click="activeTab = 'raw'">移动端原始信息</button>
            <button :class="{ active: activeTab === 'timeline' }" @click="activeTab = 'timeline'">时间线</button>
            <button :class="{ active: activeTab === 'fusion' }" @click="activeTab = 'fusion'">融合研判</button>
          </nav>

          <section v-if="activeTab === 'summary'" class="doc-body">
            <article class="standard-section">
              <header>
                <strong>一、诉求摘要</strong>
                <span>来自后端 labor_appeals</span>
              </header>
              <p>{{ detail.appeal.oral_description || '暂无口语化描述' }}</p>
            </article>

            <article class="standard-section">
              <header>
                <strong>二、关键字段</strong>
                <span>材料完整度 {{ detail.appeal.material_score }}%</span>
              </header>
              <div class="field-grid">
                <div><span>申请人</span><strong>{{ detail.appeal.worker_name }}</strong></div>
                <div><span>联系电话</span><strong>{{ detail.appeal.worker_phone }}</strong></div>
                <div><span>欠薪金额</span><strong>{{ detail.appeal.wage_amount_text || '待补充' }}</strong></div>
                <div><span>项目地点</span><strong>{{ detail.appeal.project_name || locationText }}</strong></div>
                <div><span>用工主体</span><strong>{{ detail.appeal.employer_name || '待核实' }}</strong></div>
                <div><span>工作时间</span><strong>{{ detail.appeal.work_period_text || '待补充' }}</strong></div>
              </div>
            </article>

            <article class="standard-section">
              <header>
                <strong>三、证据链完整度</strong>
                <span>{{ detail.appeal.material_score }}%</span>
              </header>
              <div class="evidence-track">
                <div class="evidence-fill" :style="{ width: `${detail.appeal.material_score}%` }"></div>
              </div>
              <div class="evidence-checks">
                <span v-for="item in missingMaterials" :key="item">{{ item }}</span>
              </div>
            </article>
          </section>

          <section v-else-if="activeTab === 'standard'" class="doc-body">
            <article class="standard-section">
              <header>
                <strong>智能整理结果</strong>
                <button @click="runStandardization">{{ standardization ? '重新整理' : '开始整理' }}</button>
              </header>
              <template v-if="standardization">
                <h3>{{ standardization.standardized_title }}</h3>
                <p>{{ standardization.standardized_text || standardization.standard_summary }}</p>
                <div class="field-grid">
                  <div><span>模型</span><strong>{{ standardization.model_name || 'fallback' }}</strong></div>
                  <div><span>Prompt</span><strong>{{ standardization.prompt_version }}</strong></div>
                  <div><span>置信度</span><strong>{{ Math.round((standardization.confidence || 0) * 100) }}%</strong></div>
                </div>
              </template>
              <p v-else>尚未生成标准化材料。点击开始整理后，系统会基于口语描述、材料元数据和定位信息生成可复核材料。</p>
            </article>

            <article class="standard-section" v-if="standardization">
              <header>
                <strong>缺失材料与冲突信息</strong>
                <span>需人工复核</span>
              </header>
              <div class="evidence-checks">
                <span v-for="item in standardMissingMaterials" :key="item.label || item">{{ item.label || item }}</span>
                <span v-if="standardMissingMaterials.length === 0">未识别到缺失材料</span>
              </div>
              <div class="conflict-list">
                <p v-for="item in standardConflicts" :key="item.description || item">{{ item.description || item }}</p>
                <p v-if="standardConflicts.length === 0">未识别到明显冲突。</p>
              </div>
            </article>

            <article class="standard-section" v-if="standardization">
              <header>
                <strong>证据强度与风险案件建议</strong>
                <span>{{ standardEvidence.level || '待评估' }}</span>
              </header>
              <div class="field-grid">
                <div><span>证据分</span><strong>{{ standardEvidence.score ?? detail.appeal.material_score }}</strong></div>
                <div><span>建议风险等级</span><strong>{{ standardRiskMapping.risk_level || 'medium' }}</strong></div>
                <div><span>建议区域</span><strong>{{ standardRiskMapping.area_name || locationText }}</strong></div>
              </div>
              <p>{{ standardRiskMapping.risk_reason_summary || '标准化后可用于转入 risk_cases 前的人工复核。' }}</p>
            </article>
          </section>

          <section v-else-if="activeTab === 'raw'" class="doc-body raw-body">
            <article class="trace-card">
              <header><strong>定位与行政区</strong><button @click="confirmLocation">人工确认</button></header>
              <p>定位点：{{ locationText }}</p>
              <p>经纬度：{{ detail.location?.latitude || '-' }}, {{ detail.location?.longitude || '-' }}</p>
              <p>行政区：{{ detail.location?.area_name || '待确认' }} / {{ detail.location?.area_code || '-' }}</p>
              <p>校验置信度：{{ Math.round((detail.location?.confidence || 0) * 100) }}%</p>
              <p>冲突标记：{{ detail.location?.conflict_flags || '无' }}</p>
              <p>人工确认：{{ detail.location?.confirmed_by_staff ? '已确认' : '待确认' }}</p>
            </article>

            <article class="trace-card">
              <header><strong>材料清单</strong><button @click="requestMaterials">催补材料</button></header>
              <div v-for="item in detail.materials" :key="item.id" class="material-row">
                <span class="ok"></span>
                <strong>{{ materialLabel(item.category) }}</strong>
                <em>{{ item.original_filename }}</em>
                <button @click="downloadMaterial(item)">下载</button>
              </div>
              <p v-if="detail.materials.length === 0">申请人尚未上传材料。</p>
            </article>
          </section>

          <section v-else-if="activeTab === 'timeline'" class="doc-body">
            <div v-for="event in detail.timeline" :key="event.id" class="mini-progress done">
              <span></span>
              <div>
                <strong>{{ event.title }}</strong>
                <small>{{ event.content || event.event_type }} · {{ formatTime(event.created_at) }}</small>
              </div>
            </div>
          </section>

          <section v-else class="doc-body">
            <article class="standard-section">
              <header>
                <strong>risk_cases 接入状态</strong>
                <button @click="loadFusionData">刷新融合数据</button>
              </header>
              <div class="field-grid">
                <div><span>关联案件</span><strong>{{ graphData?.case_code || similarData?.case_code || '待转入 risk_cases' }}</strong></div>
                <div><span>图谱同步</span><strong>{{ graphData?.graph_sync_status || 'pending' }}</strong></div>
                <div><span>向量索引</span><strong>{{ similarData?.vector_sync_status || 'pending' }}</strong></div>
              </div>
              <p>{{ graphData?.graph_sync_message || similarData?.vector_sync_message || '转入 risk_cases 后，系统会排队图谱、向量和类案发现任务。' }}</p>
            </article>

            <article class="standard-section">
              <header>
                <strong>图谱实体与关系</strong>
                <span>{{ graphData?.nodes?.length || 0 }} / {{ graphData?.edges?.length || 0 }}</span>
              </header>
              <div class="fusion-list">
                <div v-for="node in (graphData?.nodes || [])" :key="node.id" class="fusion-item">
                  <strong>{{ node.label }}</strong>
                  <span>{{ node.node_type }} · {{ Math.round((node.confidence || 0) * 100) }}%</span>
                </div>
                <p v-if="!graphData?.nodes?.length">暂无图谱实体，可先执行智能标准化或等待下游任务完成。</p>
              </div>
            </article>

            <article class="standard-section">
              <header>
                <strong>类案发现</strong>
                <span>{{ similarData?.items?.length || 0 }} 条</span>
              </header>
              <div class="fusion-list">
                <div v-for="item in (similarData?.items || [])" :key="item.id" class="fusion-item">
                  <strong>{{ item.case_code }} · {{ item.title }}</strong>
                  <span>{{ item.area_name }} · {{ item.risk_level }} · 匹配 {{ Math.round((item.match_score || 0) * 100) }}%</span>
                </div>
                <p v-if="!similarData?.items?.length">暂未发现可展示的类案案件。</p>
              </div>
            </article>
          </section>

          <footer class="work-actions">
            <button @click="runStandardization">智能整理</button>
            <button @click="startProcessing">开始办理</button>
            <button @click="convertRiskCase">转风险案件</button>
            <button class="primary" @click="resolveAppeal">办结并通知</button>
          </footer>
        </div>
        <div class="panel-card empty-panel" v-else>
          <strong>暂无线索</strong>
          <p>移动端提交后会自动出现在这里。</p>
        </div>
      </main>

      <aside class="trace-column" v-if="detail">
        <div class="panel-card trace-grid">
          <header class="panel-header compact">
            <div>
              <strong>提交信息全量视图</strong>
              <span>来自移动端真实 API</span>
            </div>
          </header>

          <article class="trace-card">
            <header><strong>申请人信息</strong><button>实名信息</button></header>
            <p>姓名：{{ detail.appeal.worker_name }}</p>
            <p>电话：{{ detail.appeal.worker_phone }}</p>
            <p>工友人数：{{ detail.appeal.coworker_count || '待补充' }}</p>
          </article>

          <article class="trace-card">
            <header><strong>办理动作</strong><button @click="loadDetail">刷新</button></header>
            <p>可用动作：{{ detail.available_actions.join('、') || '暂无' }}</p>
            <p>风险案件状态：{{ detail.appeal.risk_case_status }}</p>
            <p>关联案件数：{{ detail.risk_case_links.length }}</p>
            <p>定位确认：{{ detail.location?.confirmed_by_staff ? '工作人员已确认' : '等待人工确认' }}</p>
          </article>

          <article class="trace-card">
            <header><strong>移动端消息</strong><button @click="requestMaterials">发送补证</button></header>
            <p>补证、接收、办结会写入通知表，移动端消息列表实时读取。</p>
          </article>
        </div>
      </aside>
    </section>
  </div>
</template>

<script setup>
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import * as echarts from 'echarts'
import tongzhouGeoJson from '../assets/maps/tongzhou.json'
import { apiDownloadUrl, apiGet, apiPost } from '../api/platform'

const STAFF_HEADERS = { 'X-Staff-Id': 'dev-staff', 'X-Staff-Role': 'prosecutor' }

const mapRef = ref(null)
const activeTab = ref('summary')
const keyword = ref('')
const statusFilter = ref('')
const appeals = ref([])
const detail = ref(null)
const standardization = ref(null)
const graphData = ref(null)
const similarData = ref(null)
const total = ref(0)
const activeId = ref('')
let mapIns = null

const workflowSteps = [
  { icon: '手', name: '移动端提交', desc: '口述与定位' },
  { icon: '证', name: '证据链评分', desc: '缺口提示' },
  { icon: '补', name: '要求补证', desc: '通知回传' },
  { icon: '检', name: '接收办理', desc: '人工复核' },
  { icon: '案', name: '转风险案件', desc: '复用研判链路' },
  { icon: '结', name: '办结反馈', desc: '移动端同步' }
]

const metrics = computed(() => {
  const incomplete = appeals.value.filter((item) => item.status === 'submitted_incomplete').length
  const requested = appeals.value.filter((item) => item.status === 'material_requested').length
  const accepted = appeals.value.filter((item) => item.status === 'accepted').length
  const converted = appeals.value.filter((item) => item.risk_case_status !== 'not_converted').length
  return [
    { icon: '线', tone: 'blue', label: '移动端线索', value: total.value, delta: '真实后端' },
    { icon: '补', tone: 'orange', label: '材料不全', value: incomplete, delta: '可先提交' },
    { icon: '证', tone: 'red', label: '待补证', value: requested, delta: '已通知' },
    { icon: '收', tone: 'cyan', label: '已接收', value: accepted, delta: '进入办理' },
    { icon: '案', tone: 'purple', label: '已转案件', value: converted, delta: 'risk_cases' },
    { icon: '%', tone: 'blue', label: '当前完整度', value: detail.value?.appeal.material_score || 0, delta: '规则评分' }
  ]
})

const locationText = computed(() => detail.value?.location?.address_text || '北京市XX区XX地点')
const missingMaterials = computed(() => (detail.value?.appeal.missing_materials || '').split('\n').filter(Boolean))
const standardMissingMaterials = computed(() => Array.isArray(standardization.value?.missing_materials) ? standardization.value.missing_materials : [])
const standardConflicts = computed(() => Array.isArray(standardization.value?.conflict_items) ? standardization.value.conflict_items : [])
const standardEvidence = computed(() => standardization.value?.evidence_assessment || {})
const standardRiskMapping = computed(() => standardization.value?.risk_case_mapping || {})

const mapPointData = () => appeals.value
  .filter((item) => Number.isFinite(Number(item.longitude)) && Number.isFinite(Number(item.latitude)))
  .map((item) => {
    const isSupplementNeeded = ['submitted_incomplete', 'material_requested'].includes(item.status)
    const color = isSupplementNeeded ? '#f59e0b' : item.material_score >= 75 ? '#ef4444' : '#2563eb'
    return {
      name: item.project_name || item.address_text || item.appeal_code,
      value: [Number(item.longitude), Number(item.latitude)],
      symbolSize: item.material_score >= 75 ? 15 : isSupplementNeeded ? 12 : 9,
      itemStyle: { color }
    }
  })

const updateMapPoints = () => {
  if (!mapIns) return
  mapIns.setOption({
    series: [{ data: mapPointData() }]
  })
}

const loadAppeals = async () => {
  const params = new URLSearchParams({ page: '1', page_size: '50' })
  if (keyword.value) params.set('keyword', keyword.value)
  if (statusFilter.value) params.set('status', statusFilter.value)
  const data = await apiGet(`/prosecutor/appeals?${params.toString()}`, { headers: STAFF_HEADERS })
  appeals.value = data.items || []
  total.value = data.total || appeals.value.length
  updateMapPoints()
  if (!detail.value && appeals.value[0]) await openDetail(appeals.value[0].id)
}

const openDetail = async (id) => {
  activeId.value = id
  detail.value = await apiGet(`/prosecutor/appeals/${id}`, { headers: STAFF_HEADERS })
  standardization.value = await apiGet(`/prosecutor/appeals/${id}/standardizations/latest`, { headers: STAFF_HEADERS })
  await loadFusionData()
}

const loadDetail = async () => {
  if (activeId.value) await openDetail(activeId.value)
}

const loadFusionData = async () => {
  graphData.value = null
  similarData.value = null
  if (!activeId.value || !detail.value?.risk_case_links?.length) return
  try {
    const [graph, similar] = await Promise.all([
      apiGet(`/prosecutor/appeals/${activeId.value}/graph`, { headers: STAFF_HEADERS }),
      apiGet(`/prosecutor/appeals/${activeId.value}/similar?limit=5`, { headers: STAFF_HEADERS })
    ])
    graphData.value = graph
    similarData.value = similar
  } catch (error) {
    graphData.value = null
    similarData.value = null
  }
}

const setStatus = async (status) => {
  statusFilter.value = status
  await loadAppeals()
}

const requestMaterials = async () => {
  if (!activeId.value) return
  await apiPost(`/prosecutor/appeals/${activeId.value}/request-materials`, {
    request_materials: ['劳动合同或用工证明', '工资记录', '考勤记录'],
    message: '请补充能证明工作关系和欠薪金额的材料；如果暂时没有，可以先上传聊天记录、转账记录或工友证明。',
    deadline: '2026-05-25',
    internal_note: '当前材料链不完整，需补强工作关系和欠薪金额。'
  }, { headers: STAFF_HEADERS })
  await afterAction()
}

const confirmLocation = async () => {
  if (!activeId.value || !detail.value?.location) return
  await apiPost(`/prosecutor/appeals/${activeId.value}/location/confirm`, {
    area_code: detail.value.location.area_code,
    area_name: detail.value.location.area_name,
    address_text: detail.value.location.address_text
  }, { headers: STAFF_HEADERS })
  await loadDetail()
}

const runStandardization = async () => {
  if (!activeId.value) return
  standardization.value = await apiPost(`/prosecutor/appeals/${activeId.value}/standardize`, {}, { headers: STAFF_HEADERS })
  activeTab.value = 'standard'
  await loadDetail()
}

const acceptAppeal = async () => {
  if (!activeId.value) return
  await apiPost(`/prosecutor/appeals/${activeId.value}/accept`, {}, { headers: STAFF_HEADERS })
  await afterAction()
}

const startProcessing = async () => {
  if (!activeId.value) return
  await apiPost(`/prosecutor/appeals/${activeId.value}/status`, {
    action: 'start_processing',
    reason: '已联系属地部门核实欠薪事实。',
    notify_applicant: true
  }, { headers: STAFF_HEADERS })
  await afterAction()
}

const convertRiskCase = async () => {
  if (!activeId.value) return
  await apiPost(`/prosecutor/appeals/${activeId.value}/convert-risk-case`, {
    risk_level: 'medium',
    risk_tags: ['欠薪', '农民工', '工程建设'],
    create_alert: true,
    create_dispatch_task: false
  }, { headers: STAFF_HEADERS })
  activeTab.value = 'fusion'
  await afterAction()
}

const resolveAppeal = async () => {
  if (!activeId.value) return
  await apiPost(`/prosecutor/appeals/${activeId.value}/resolve`, {
    result_summary: '经协调，已督促相关单位支付部分工资，剩余部分进入后续监督。',
    notify_applicant: true
  }, { headers: STAFF_HEADERS })
  await afterAction()
}

const afterAction = async () => {
  await loadDetail()
  await loadAppeals()
}

const statusText = (status) => ({
  submitted: '已提交',
  submitted_incomplete: '材料不完整已提交',
  under_review: '复核中',
  material_requested: '待补证',
  accepted: '已接收',
  processing: '办理中',
  resolved: '已办结',
  closed: '已关闭',
  rejected: '已退回'
}[status] || status || '未知')

const scoreTone = (score) => score >= 75 ? 'high' : score >= 60 ? 'medium' : 'low'

const materialLabel = (category) => ({
  identity: '身份信息',
  labor_contract: '劳动合同/用工证明',
  wage_record: '工资记录/欠条',
  attendance: '考勤记录',
  chat_record: '聊天记录',
  bank_statement: '银行流水',
  work_badge: '工牌',
  project_photo: '项目照片',
  location_screenshot: '定位截图',
  coworker_statement: '工友证明',
  other: '其他材料'
}[category] || category)

const downloadMaterial = (item) => {
  if (!activeId.value || !item?.id) return
  window.open(apiDownloadUrl(`/prosecutor/appeals/${activeId.value}/materials/${item.id}/download`), '_blank', 'noopener')
}

const formatTime = (value) => value ? new Date(value).toLocaleString('zh-CN') : ''

const initMap = async () => {
  await nextTick()
  if (!mapRef.value) return
  mapIns = echarts.init(mapRef.value)
  echarts.registerMap('tongzhou-appeal', tongzhouGeoJson)
  mapIns.setOption({
    backgroundColor: 'transparent',
    tooltip: { trigger: 'item', formatter: '{b}' },
    geo: {
      map: 'tongzhou-appeal',
      roam: false,
      zoom: 1.08,
      itemStyle: { areaColor: '#f8fbff', borderColor: '#b9cced', borderWidth: 1 },
      emphasis: { itemStyle: { areaColor: '#e8f1ff' }, label: { color: '#173a7a' } }
    },
    series: [{
      type: 'effectScatter',
      coordinateSystem: 'geo',
      rippleEffect: { scale: 3.2, brushType: 'stroke' },
      data: mapPointData()
    }]
  })
}

onMounted(async () => {
  await Promise.all([loadAppeals(), initMap()])
  window.addEventListener('resize', () => mapIns?.resize())
})

onUnmounted(() => {
  mapIns?.dispose()
})
</script>

<style scoped>
.appeal-workbench {
  height: 100%;
  padding: 18px;
  background: #f3f6fb;
  color: #17233f;
  overflow: auto;
  font-family: "PingFang SC", "Microsoft YaHei", sans-serif;
}

button {
  border: 0;
  cursor: pointer;
  font: inherit;
}

.metric-strip {
  display: grid;
  grid-template-columns: repeat(6, minmax(130px, 1fr));
  gap: 10px;
  margin-bottom: 12px;
}

.metric-card,
.panel-card,
.flow-strip {
  background: #ffffff;
  border: 1px solid #e0e8f5;
  border-radius: 8px;
  box-shadow: 0 4px 14px rgba(30, 58, 112, 0.06);
}

.metric-card {
  min-height: 82px;
  display: grid;
  grid-template-columns: 44px 1fr;
  gap: 10px;
  align-items: center;
  padding: 12px;
}

.metric-icon {
  width: 38px;
  height: 38px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  font-weight: 900;
}

.blue { background: #eaf2ff; color: #1f63d1; }
.orange { background: #fff2e5; color: #e7791a; }
.red { background: #fff0f0; color: #d9363e; }
.purple { background: #f0ecff; color: #5b45d6; }
.cyan { background: #e8f8ff; color: #1282a8; }

.metric-card span,
.metric-card small,
.panel-header span,
.standard-section span,
small,
em {
  color: #687793;
  font-size: 12px;
  font-style: normal;
}

.metric-card strong {
  display: block;
  font-size: 20px;
  color: #173a7a;
}

.flow-strip {
  min-height: 92px;
  display: grid;
  grid-template-columns: 128px repeat(6, minmax(110px, 1fr));
  gap: 8px;
  align-items: center;
  padding: 12px;
  margin-bottom: 12px;
}

.flow-title {
  color: #174ea6;
  font-weight: 900;
  border-right: 1px solid #dbe5f3;
  height: 60px;
  display: grid;
  place-items: center;
  text-align: center;
}

.flow-step {
  min-height: 68px;
  border-radius: 8px;
  display: grid;
  justify-items: center;
  align-content: center;
  gap: 4px;
  background: #f7faff;
}

.flow-step.active span {
  color: #174ea6;
  font-weight: 900;
}

.main-grid {
  display: grid;
  grid-template-columns: minmax(320px, 0.78fr) minmax(520px, 1.28fr) minmax(280px, 0.72fr);
  gap: 12px;
}

.panel-card {
  padding: 14px;
}

.panel-header,
.review-header,
.doc-tabs,
.header-actions,
.work-actions,
.search-row,
.status-tabs,
.case-item,
.material-row,
.mini-progress {
  display: flex;
  align-items: center;
  gap: 10px;
}

.panel-header,
.review-header {
  justify-content: space-between;
}

.panel-header button,
.header-actions button,
.work-actions button,
.status-tabs button,
.search-row button {
  padding: 8px 12px;
  border-radius: 8px;
  background: #edf3fb;
  color: #173a7a;
}

.primary,
.work-actions .primary {
  background: #1f63d1 !important;
  color: #fff !important;
}

.search-row {
  margin: 12px 0;
}

.search-row input {
  min-width: 0;
  flex: 1;
  border: 1px solid #d8e3f2;
  border-radius: 8px;
  padding: 9px 10px;
}

.status-tabs {
  flex-wrap: wrap;
  margin-bottom: 10px;
}

.status-tabs .active {
  background: #1f63d1;
  color: #fff;
}

.case-item {
  width: 100%;
  text-align: left;
  border-radius: 8px;
  padding: 12px;
  background: #f7faff;
  margin-top: 8px;
}

.case-item.active {
  outline: 2px solid #9fc3ff;
  background: #eef5ff;
}

.risk-badge {
  width: 42px;
  height: 42px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  font-weight: 900;
}

.risk-badge.high { background: #fee2e2; color: #b91c1c; }
.risk-badge.medium { background: #ffedd5; color: #c2410c; }
.risk-badge.low { background: #dcfce7; color: #15803d; }

.case-main {
  flex: 1;
  min-width: 0;
}

.case-main strong,
.case-main span,
.case-main small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.map-panel {
  margin-top: 12px;
}

.tongzhou-map {
  height: 260px;
}

.map-legend {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.map-legend i {
  width: 8px;
  height: 8px;
  display: inline-block;
  border-radius: 999px;
  margin-right: 5px;
}

.map-legend .red { background: #ef4444; }
.map-legend .blue { background: #2563eb; }
.map-legend .orange { background: #f59e0b; }

.review-header h2 {
  margin: 4px 0;
  font-size: 22px;
}

.kicker {
  color: #1f63d1;
  font-size: 12px;
  font-weight: 900;
}

.doc-tabs {
  margin: 14px 0;
  border-bottom: 1px solid #dbe5f3;
}

.doc-tabs button {
  background: transparent;
  padding: 10px 0;
  color: #687793;
}

.doc-tabs .active {
  color: #174ea6;
  font-weight: 900;
  border-bottom: 3px solid #174ea6;
}

.doc-body {
  display: grid;
  gap: 12px;
}

.standard-section,
.trace-card,
.empty-panel {
  background: #f8fbff;
  border: 1px solid #dfe8f5;
  border-radius: 8px;
  padding: 14px;
}

.standard-section header,
.trace-card header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 10px;
}

.standard-section h3 {
  margin: 4px 0 8px;
  color: #173a7a;
}

.field-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}

.field-grid div {
  background: #fff;
  border-radius: 8px;
  padding: 10px;
}

.field-grid span,
.field-grid strong {
  display: block;
}

.evidence-track {
  height: 10px;
  background: #e8eef7;
  border-radius: 999px;
  overflow: hidden;
}

.evidence-fill {
  height: 100%;
  background: #1f63d1;
}

.evidence-checks {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 10px;
}

.evidence-checks span {
  background: #fff7ed;
  color: #9a3412;
  border-radius: 999px;
  padding: 6px 9px;
}

.conflict-list {
  margin-top: 10px;
}

.conflict-list p {
  margin: 6px 0;
  color: #46556c;
}

.fusion-list {
  display: grid;
  gap: 8px;
}

.fusion-item {
  display: grid;
  gap: 4px;
  background: #fff;
  border: 1px solid #e4ecf7;
  border-radius: 8px;
  padding: 10px;
}

.fusion-item span {
  color: #687793;
  font-size: 12px;
}

.trace-grid {
  display: grid;
  gap: 12px;
}

.trace-card button {
  background: #edf3fb;
  color: #174ea6;
  border-radius: 8px;
  padding: 6px 9px;
}

.material-row,
.mini-progress {
  justify-content: space-between;
  padding: 8px 0;
  border-top: 1px solid #e4ecf7;
}

.material-row .ok,
.mini-progress > span {
  width: 10px;
  height: 10px;
  border-radius: 999px;
  background: #22c55e;
}

.work-actions {
  margin-top: 14px;
  justify-content: flex-end;
}

.work-actions button {
  padding: 10px 14px;
  border-radius: 8px;
  background: #edf3fb;
  color: #173a7a;
}

@media (max-width: 1180px) {
  .metric-strip {
    grid-template-columns: repeat(3, 1fr);
  }

  .flow-strip,
  .main-grid {
    grid-template-columns: 1fr;
  }
}
</style>

<template>
  <main class="mobile-shell">
    <section class="phone-stage">
      <header class="status-bar">
        <span>9:41</span>
        <span>5G · 96%</span>
      </header>

      <nav class="top-nav">
        <button class="icon-button" @click="prevStep">‹</button>
        <h1>{{ activeStep.title }}</h1>
        <button class="icon-button" @click="refreshAll">↻</button>
      </nav>

      <div class="flow-progress">
        <div class="progress-copy">
          <span>{{ activeStep.phase }}</span>
          <strong>{{ progressPercent }}%</strong>
        </div>
        <div class="progress-track">
          <div class="progress-fill" :style="{ width: `${progressPercent}%` }"></div>
        </div>
      </div>

      <section ref="contentRef" class="content">
        <div class="notice-line">
          <span class="shield">盾</span>
          <span>信息仅用于欠薪线索反映和办理进度同步</span>
        </div>

        <TransitionGroup name="ppt" tag="div" class="slide-stack">
          <article
            v-for="step in visibleSteps"
            :key="step.id"
            class="flow-block white-card"
            :class="{ highlight: step.id === activeStep.id }"
          >
            <template v-if="step.id === 'profile'">
              <section class="profile-hero">
                <div class="profile-avatar">工</div>
                <div>
                  <strong>{{ profile?.display_name || '张师傅' }}</strong>
                  <p>{{ profile?.address_text || '北京市XX区XX地点' }}</p>
                </div>
                <span class="verified-badge">{{ profile?.auth_status === 'dev_verified' ? '已核验' : '待核验' }}</span>
              </section>
              <section class="profile-grid">
                <div><strong>{{ appeals.length }}</strong><span>我的线索</span></div>
                <div><strong>{{ currentAppeal?.material_score || 0 }}</strong><span>材料完整度</span></div>
                <div><strong>{{ unreadCount }}</strong><span>未读消息</span></div>
              </section>
              <div class="profile-row"><span>联系电话</span><strong>{{ profile?.phone || '13800001234' }}</strong></div>
              <div class="profile-row"><span>务工类型</span><strong>{{ profile?.worker_type || '建筑施工 / 临时用工' }}</strong></div>
            </template>

            <template v-else-if="step.id === 'messages'">
              <header class="section-title">
                <strong>最新消息</strong>
                <em>{{ unreadCount }} 条未读</em>
              </header>
              <button
                v-for="message in notifications"
                :key="message.id"
                class="message-row"
                :class="{ unread: !message.read_at }"
                @click="markRead(message.id)"
              >
                <span class="message-dot"></span>
                <div>
                  <strong>{{ message.title }}</strong>
                  <small>{{ message.content }}</small>
                </div>
                <em>{{ message.read_at ? '已读' : '未读' }}</em>
              </button>
              <p v-if="notifications.length === 0" class="empty-copy">暂无新消息，提交线索后办理进度会在这里更新。</p>
            </template>

            <template v-else-if="step.id === 'oral'">
              <div class="chat-row bot">
                <div class="bot-avatar">AI</div>
                <div class="bubble">请像平时说话一样描述欠薪情况，不需要一开始就写成正式材料。</div>
              </div>
              <textarea v-model="draft.oral_description" rows="5"></textarea>
            </template>

            <template v-else-if="step.id === 'fields'">
              <header class="section-title">
                <strong>逐步补充关键信息</strong>
                <em>可先填知道的部分</em>
              </header>
              <label v-for="field in fieldDefs" :key="field.key" class="form-row">
                <span>{{ field.label }}</span>
                <input v-model="draft[field.key]" :placeholder="field.placeholder" />
              </label>
              <label class="form-row">
                <span>工友人数</span>
                <input v-model.number="draft.coworker_count" type="number" />
              </label>
            </template>

            <template v-else-if="step.id === 'map'">
              <header class="section-title">
                <strong>地图辅助定位</strong>
                <button @click="saveLocation">确认定位</button>
              </header>
              <section class="beijing-map opened">
                <div class="district north">北部山区</div>
                <div class="district west">西部片区</div>
                <div class="district center">中心城区</div>
                <div class="district east active">XX区</div>
                <div class="district south">南部片区</div>
                <button class="map-pin" @click="saveLocation"><span></span>北京市XX区XX地点</button>
              </section>
              <div class="district-picker">
                <button
                  v-for="district in districts"
                  :key="district.area_code"
                  :class="{ active: location.area_code === district.area_code }"
                  @click="chooseDistrict(district)"
                >
                  {{ district.area_name }}
                </button>
              </div>
              <aside class="geo-check">
                <strong>行政区划确认</strong>
                <p>当前以定位点、反向地址、行政边界三项信息共同确认：{{ location.area_name }}。</p>
              </aside>
            </template>

            <template v-else-if="step.id === 'materials'">
              <header class="section-title">
                <strong>补充材料</strong>
                <em>{{ materials.length }} 份已上传</em>
              </header>
              <button
                v-for="item in materialDefs"
                :key="item.category"
                class="evidence-row"
                :class="{ uploaded: hasMaterial(item.category) }"
                @click="uploadSampleMaterial(item)"
              >
                <span class="doc-icon">{{ hasMaterial(item.category) ? '✓' : '需' }}</span>
                <span><strong>{{ item.label }}</strong><small>{{ item.tip }}</small></span>
                <em>{{ hasMaterial(item.category) ? '已补' : '上传' }}</em>
              </button>
            </template>

            <template v-else-if="step.id === 'submit'">
              <header class="section-title">
                <strong>提交确认</strong>
                <em>{{ currentAppeal?.material_score || 0 }}%</em>
              </header>
              <p>材料不完整也可以先提交，后续如果工作人员要求补证，可以回到本线索继续补交材料。</p>
              <div class="risk-grid">
                <div><strong>当前状态</strong><span>{{ statusText(currentAppeal?.status) }}</span></div>
                <div><strong>缺少材料</strong><span>{{ missingMaterialsText }}</span></div>
              </div>
            </template>

            <template v-else-if="step.id === 'cases'">
              <header class="section-title">
                <strong>我的线索</strong>
                <em>{{ appeals.length }} 条</em>
              </header>
              <button v-for="appeal in appeals" :key="appeal.id" class="case-row" @click="openAppeal(appeal.id)">
                <div>
                  <strong>{{ appeal.project_name || '北京市XX区XX地点欠薪线索' }}</strong>
                  <small>{{ appeal.appeal_code }} · {{ statusText(appeal.status) }}</small>
                </div>
                <em>{{ appeal.material_score }}%</em>
              </button>
            </template>

            <template v-else-if="step.id === 'progress'">
              <header class="section-title">
                <strong>办理进度</strong>
                <em>来自后端时间线</em>
              </header>
              <div v-for="event in timeline" :key="event.id" class="progress-node done">
                <span></span>
                <div>
                  <strong>{{ event.title }}</strong>
                  <small>{{ event.content || event.event_type }}</small>
                </div>
              </div>
            </template>
          </article>
        </TransitionGroup>
      </section>

      <footer class="demo-controls">
        <button class="ghost-action" @click="saveDraft" :disabled="busy">存草稿</button>
        <button class="primary-action" @click="nextStep" :disabled="busy">{{ nextLabel }}</button>
        <button v-if="currentAppeal?.status === 'material_requested'" class="ghost-action" @click="submitSupplement" :disabled="busy">提交补证</button>
      </footer>
    </section>
  </main>
</template>

<script setup>
import { computed, nextTick, onMounted, ref } from 'vue'
import { apiGet, apiPost, apiPut, apiUpload } from './api/platform'

const MOBILE_HEADERS = { 'X-Mobile-Applicant-Id': '11111111-1111-1111-1111-111111111111' }

const contentRef = ref(null)
const busy = ref(false)
const stepIndex = ref(0)
const profile = ref(null)
const notifications = ref([])
const appeals = ref([])
const currentAppeal = ref(null)
const materials = ref([])
const timeline = ref([])
const districts = ref([])

const draft = ref({
  oral_description: '我在北京市XX区XX地点附近的工地干活，老板拖欠三个月工资，大概三万二，希望帮忙要回工资。',
  wage_amount_text: '大概三万二',
  employer_name: '不确定，只知道可能是XX劳务',
  contractor_name: '李某',
  project_name: '北京市XX区XX地点附近项目',
  work_period_text: '2025年6月到2026年1月',
  coworker_count: 6,
  demand_text: '希望帮忙要回工资',
  worker_name: '张三',
  worker_phone: '13800001234'
})

const location = ref({
  latitude: 39.9023,
  longitude: 116.6561,
  address_text: '北京市XX区XX地点附近',
  area_code: '110112',
  area_name: '北京市XX区',
  confirmed_by_applicant: true
})

const steps = [
  { id: 'profile', title: '个人中心', phase: '个人信息' },
  { id: 'messages', title: '最新消息', phase: '消息通知' },
  { id: 'oral', title: '欠薪诉求反映', phase: '问询 1/6' },
  { id: 'fields', title: '补充信息', phase: '问询 2/6' },
  { id: 'map', title: '位置确认', phase: '定位 3/6' },
  { id: 'materials', title: '材料补充', phase: '材料 4/6' },
  { id: 'submit', title: '提交确认', phase: '提交 5/6' },
  { id: 'cases', title: '我的线索', phase: '进度查看' },
  { id: 'progress', title: '办理进度', phase: '进度查看' }
]

const fieldDefs = [
  { key: 'wage_amount_text', label: '欠薪金额', placeholder: '大概两万多' },
  { key: 'project_name', label: '项目/地点', placeholder: '说不清项目名也可以写附近地点' },
  { key: 'employer_name', label: '单位/老板', placeholder: '不知道全名也可以写线索' },
  { key: 'contractor_name', label: '联系人', placeholder: '包工头或现场负责人' },
  { key: 'work_period_text', label: '工作时间', placeholder: '去年十月到今年一月' },
  { key: 'worker_name', label: '姓名', placeholder: '张三' },
  { key: 'worker_phone', label: '联系电话', placeholder: '13800001234' },
  { key: 'demand_text', label: '诉求', placeholder: '希望帮忙要回工资' }
]

const materialDefs = [
  { category: 'chat_record', label: '聊天催讨记录', tip: '微信截图、短信、通话记录等' },
  { category: 'wage_record', label: '工资记录或欠条', tip: '结算单、欠条、工资表' },
  { category: 'attendance', label: '考勤记录', tip: '打卡、进出场或班组记录' },
  { category: 'bank_statement', label: '银行流水', tip: '曾发工资或转账记录' },
  { category: 'coworker_statement', label: '工友证明', tip: '工友联系方式或书面说明' },
  { category: 'identity', label: '身份信息', tip: '便于工作人员联系核实' }
]

const activeStep = computed(() => steps[stepIndex.value])
const visibleSteps = computed(() => steps.slice(0, stepIndex.value + 1))
const progressPercent = computed(() => Math.round(((stepIndex.value + 1) / steps.length) * 100))
const unreadCount = computed(() => notifications.value.filter((item) => !item.read_at).length)
const missingMaterialsText = computed(() => currentAppeal.value?.missing_materials || '后端将根据材料完整度计算')

const nextLabel = computed(() => {
  if (activeStep.value.id === 'messages') return '开始申诉'
  if (activeStep.value.id === 'map') return '保存定位'
  if (activeStep.value.id === 'materials') return '材料不完整先提交'
  if (activeStep.value.id === 'submit') return '提交线索'
  if (activeStep.value.id === 'cases') return '查看进度'
  if (activeStep.value.id === 'progress') return '刷新进度'
  return '下一步'
})

const refreshAll = async () => {
  busy.value = true
  try {
    profile.value = (await apiGet('/mobile/profile', { headers: MOBILE_HEADERS })).profile
  } catch {
    profile.value = null
  }
  try {
    districts.value = await apiGet('/geo/beijing-districts')
  } catch {
    districts.value = [{ area_code: '110112', area_name: '北京市XX区' }]
  }
  await Promise.all([loadAppeals(), loadNotifications()])
  busy.value = false
}

const ensureDraft = async () => {
  if (currentAppeal.value?.id) return currentAppeal.value
  const appeal = await apiPost('/mobile/appeals/drafts', {
    client_request_id: `mobile-${Date.now()}`,
    oral_description: draft.value.oral_description,
    worker_name: draft.value.worker_name,
    worker_phone: draft.value.worker_phone
  }, { headers: MOBILE_HEADERS })
  currentAppeal.value = appeal
  await loadAppeals()
  return appeal
}

const saveDraft = async () => {
  busy.value = true
  try {
    const appeal = await ensureDraft()
    currentAppeal.value = await apiPut(`/mobile/appeals/${appeal.id}/draft`, draft.value, { headers: MOBILE_HEADERS })
    await openAppeal(currentAppeal.value.id)
  } finally {
    busy.value = false
  }
}

const saveLocation = async () => {
  busy.value = true
  try {
    const appeal = await ensureDraft()
    await saveDraft()
    await apiPut(`/mobile/appeals/${appeal.id}/location`, location.value, { headers: MOBILE_HEADERS })
    await openAppeal(appeal.id)
  } finally {
    busy.value = false
  }
}

const uploadSampleMaterial = async (item) => {
  if (hasMaterial(item.category)) return
  busy.value = true
  try {
    const appeal = await ensureDraft()
    const formData = new FormData()
    const file = new File([`${item.label}示例材料`], `${item.category}.txt`, { type: 'text/plain' })
    formData.append('file', file)
    formData.append('category', item.category)
    formData.append('description', item.tip)
    await apiUpload(`/mobile/appeals/${appeal.id}/materials`, formData, { headers: MOBILE_HEADERS })
    await openAppeal(appeal.id)
  } finally {
    busy.value = false
  }
}

const submitAppeal = async () => {
  busy.value = true
  try {
    const appeal = await ensureDraft()
    await saveDraft()
    currentAppeal.value = await apiPost(`/mobile/appeals/${appeal.id}/submit`, { allow_incomplete: true }, { headers: MOBILE_HEADERS })
    await openAppeal(appeal.id)
    await loadNotifications()
  } finally {
    busy.value = false
  }
}

const submitSupplement = async () => {
  if (!currentAppeal.value?.id) return
  busy.value = true
  try {
    await apiPost(`/mobile/appeals/${currentAppeal.value.id}/supplement`, {}, { headers: MOBILE_HEADERS })
    await openAppeal(currentAppeal.value.id)
    await loadNotifications()
  } finally {
    busy.value = false
  }
}

const loadAppeals = async () => {
  appeals.value = await apiGet('/mobile/appeals', { headers: MOBILE_HEADERS })
  if (!currentAppeal.value && appeals.value[0]) {
    await openAppeal(appeals.value[0].id)
  }
}

const openAppeal = async (id) => {
  const detail = await apiGet(`/mobile/appeals/${id}`, { headers: MOBILE_HEADERS })
  currentAppeal.value = detail.appeal
  materials.value = detail.materials || []
  timeline.value = detail.timeline || []
}

const loadNotifications = async () => {
  notifications.value = await apiGet('/mobile/notifications', { headers: MOBILE_HEADERS })
}

const markRead = async (id) => {
  await apiPost(`/mobile/notifications/${id}/read`, {}, { headers: MOBILE_HEADERS })
  await loadNotifications()
}

const nextStep = async () => {
  if (activeStep.value.id === 'oral' || activeStep.value.id === 'fields') await saveDraft()
  if (activeStep.value.id === 'map') await saveLocation()
  if (activeStep.value.id === 'submit') await submitAppeal()
  if (activeStep.value.id === 'progress') await refreshAll()
  if (stepIndex.value < steps.length - 1) stepIndex.value += 1
  await scrollBottom()
}

const prevStep = () => {
  if (stepIndex.value > 0) stepIndex.value -= 1
}

const chooseDistrict = (district) => {
  location.value.area_code = district.area_code
  location.value.area_name = district.area_name.replace('通州区', 'XX区')
}

const hasMaterial = (category) => materials.value.some((item) => item.category === category)

const statusText = (status) => {
  const map = {
    draft: '草稿',
    submitted: '已提交',
    submitted_incomplete: '材料不完整已提交',
    under_review: '复核中',
    material_requested: '待补证',
    accepted: '已接收',
    processing: '办理中',
    resolved: '已办结',
    closed: '已关闭',
    rejected: '已退回'
  }
  return map[status] || status || '未创建'
}

const scrollBottom = async () => {
  await nextTick()
  contentRef.value?.scrollTo({ top: contentRef.value.scrollHeight, behavior: 'smooth' })
}

onMounted(refreshAll)
</script>

<style scoped>
.mobile-shell {
  min-height: 100vh;
  display: grid;
  place-items: center;
  background: #eef3f8;
  color: #152033;
  font-family: "PingFang SC", "Microsoft YaHei", sans-serif;
}

.phone-stage {
  width: min(430px, 100vw);
  height: min(920px, 100vh);
  background: #f7f9fc;
  display: grid;
  grid-template-rows: auto auto auto 1fr auto;
  overflow: hidden;
}

.status-bar,
.top-nav,
.demo-controls,
.flow-progress {
  padding: 10px 18px;
}

.status-bar,
.top-nav,
.progress-copy,
.section-title,
.profile-hero,
.message-row,
.case-row,
.evidence-row,
.demo-controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.top-nav h1 {
  font-size: 18px;
  margin: 0;
}

button {
  border: 0;
  cursor: pointer;
  font: inherit;
}

.icon-button {
  width: 34px;
  height: 34px;
  border-radius: 999px;
  background: #e8eef7;
}

.progress-track {
  height: 7px;
  background: #dfe7f2;
  border-radius: 999px;
  overflow: hidden;
  margin-top: 8px;
}

.progress-fill {
  height: 100%;
  background: #2563eb;
}

.content {
  overflow: auto;
  padding: 0 14px 14px;
}

.notice-line {
  margin: 8px 0;
  padding: 9px 12px;
  background: #e9f7ef;
  color: #166534;
  border-radius: 8px;
  font-size: 13px;
}

.slide-stack {
  display: grid;
  gap: 12px;
}

.white-card {
  background: #fff;
  border: 1px solid #dfe8f5;
  border-radius: 8px;
  padding: 14px;
  box-shadow: 0 6px 18px rgba(21, 32, 51, 0.06);
}

.highlight {
  outline: 2px solid #bfd7ff;
}

.profile-avatar,
.bot-avatar {
  width: 42px;
  height: 42px;
  border-radius: 999px;
  display: grid;
  place-items: center;
  background: #1f63d1;
  color: #fff;
  font-weight: 900;
}

.verified-badge {
  color: #0f766e;
  background: #e6fffb;
  padding: 5px 8px;
  border-radius: 999px;
  font-size: 12px;
}

.profile-grid,
.risk-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  margin: 12px 0;
}

.profile-grid div,
.risk-grid div {
  background: #f5f8fc;
  border-radius: 8px;
  padding: 10px;
}

.profile-grid strong,
.risk-grid strong {
  display: block;
  color: #174ea6;
}

.profile-grid span,
.risk-grid span,
small,
em {
  color: #66758d;
  font-size: 12px;
  font-style: normal;
}

.profile-row,
.form-row {
  display: grid;
  gap: 6px;
  margin-top: 10px;
}

input,
textarea {
  width: 100%;
  box-sizing: border-box;
  border: 1px solid #cfd9e8;
  border-radius: 8px;
  padding: 10px;
  background: #fbfdff;
}

.message-row,
.case-row,
.evidence-row {
  width: 100%;
  text-align: left;
  background: #f7faff;
  border-radius: 8px;
  padding: 11px;
  margin-top: 8px;
}

.message-row.unread .message-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: #ef4444;
}

.chat-row {
  display: flex;
  gap: 10px;
  margin-bottom: 12px;
}

.bubble {
  background: #eef4ff;
  border-radius: 8px;
  padding: 10px;
}

.beijing-map {
  height: 210px;
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  grid-template-rows: repeat(3, 1fr);
  gap: 6px;
  margin: 12px 0;
  position: relative;
}

.district {
  display: grid;
  place-items: center;
  border-radius: 8px;
  background: #edf3fb;
  color: #4d5e78;
  font-size: 12px;
}

.district.active {
  background: #dbeafe;
  color: #1d4ed8;
  font-weight: 900;
}

.north { grid-column: 2; }
.west { grid-column: 1; grid-row: 2; }
.center { grid-column: 2; grid-row: 2; }
.east { grid-column: 3; grid-row: 2; }
.south { grid-column: 2; grid-row: 3; }

.map-pin {
  position: absolute;
  right: 28px;
  top: 82px;
  background: #fff;
  border: 1px solid #ef4444;
  border-radius: 999px;
  padding: 7px 10px;
  color: #b91c1c;
}

.district-picker {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.district-picker button,
.section-title button {
  padding: 7px 10px;
  border-radius: 999px;
  background: #edf3fb;
}

.district-picker .active,
.section-title button,
.primary-action {
  background: #2563eb;
  color: #fff;
}

.geo-check {
  margin-top: 10px;
  background: #f8fafc;
  padding: 10px;
  border-radius: 8px;
}

.doc-icon {
  min-width: 28px;
  height: 28px;
  border-radius: 999px;
  display: grid;
  place-items: center;
  background: #dbeafe;
  color: #174ea6;
}

.evidence-row.uploaded .doc-icon {
  background: #dcfce7;
  color: #15803d;
}

.progress-node {
  display: grid;
  grid-template-columns: 12px 1fr;
  gap: 10px;
  margin-top: 12px;
}

.progress-node > span {
  width: 10px;
  height: 10px;
  border-radius: 999px;
  background: #22c55e;
  margin-top: 4px;
}

.ghost-action,
.primary-action {
  flex: 1;
  min-height: 42px;
  border-radius: 8px;
}

.ghost-action {
  background: #e8eef7;
  color: #173a7a;
}

.ppt-enter-active,
.ppt-leave-active {
  transition: all .24s ease;
}

.ppt-enter-from,
.ppt-leave-to {
  opacity: 0;
  transform: translateY(12px);
}
</style>

<template>
  <main class="mobile-shell">
    <section class="phone-stage">
      <header class="status-bar">
        <span>9:41</span>
        <span class="status-icons">●●● WiFi ▰</span>
      </header>

      <nav class="top-nav">
        <button class="icon-button" aria-label="返回">‹</button>
        <h1>{{ currentTitle }}</h1>
        <div class="mini-actions" aria-label="更多操作">
          <span></span>
          <span></span>
          <span></span>
        </div>
      </nav>

      <div class="flow-progress">
        <div class="progress-copy">
          <span>{{ currentPhase }}</span>
          <strong>{{ progressText }}</strong>
        </div>
        <div class="progress-track">
          <div class="progress-fill" :style="{ width: `${progressPercent}%` }"></div>
        </div>
      </div>

      <section ref="contentRef" class="content">
        <div class="notice-line">
          <span class="shield">◇</span>
          <span>信息仅用于欠薪线索反映，严格保密</span>
        </div>

        <TransitionGroup name="ppt" tag="div" class="slide-stack">
          <div
            v-for="item in visibleSteps"
            :key="item.id"
            class="flow-block"
            :class="[`type-${item.type}`, { highlight: item.id === activeStep.id }]"
          >
            <template v-if="item.type === 'bot' || item.type === 'user'">
              <div class="chat-row" :class="item.type">
                <div v-if="item.type === 'bot'" class="bot-avatar">AI</div>
                <div class="bubble">{{ item.text }}</div>
                <div v-if="item.type === 'user'" class="user-avatar"></div>
              </div>
            </template>

            <article v-else-if="item.type === 'profile'" class="profile-page">
              <section class="profile-hero">
                <div class="profile-avatar">申</div>
                <div>
                  <strong>张某某</strong>
                  <p>北京市XX区XX地点务工人员</p>
                </div>
                <span class="verified-badge">已认证</span>
              </section>

              <section class="profile-grid">
                <div>
                  <strong>1</strong>
                  <span>进行中线索</span>
                </div>
                <div>
                  <strong>{{ uploadedCount }}/6</strong>
                  <span>材料链完整度</span>
                </div>
                <div>
                  <strong>{{ draftSaved ? '1' : '0' }}</strong>
                  <span>已保存草稿</span>
                </div>
              </section>

              <section class="white-card profile-card">
                <header>
                  <strong>个人信息</strong>
                  <button @click="profileEdited = true">{{ profileEdited ? '已更新' : '完善' }}</button>
                </header>
                <div v-for="row in profileRows" :key="row.label" class="profile-row">
                  <span>{{ row.label }}</span>
                  <strong>{{ row.value }}</strong>
                </div>
              </section>

              <section class="entry-actions">
                <button @click="nextStep">开始欠薪申诉</button>
                <button @click="jumpToMessages">查看最新消息</button>
              </section>
            </article>

            <article v-else-if="item.type === 'messages'" class="white-card messages-card">
              <header>
                <strong>最新消息</strong>
                <span>{{ unreadCount }} 条未读</span>
              </header>
              <button
                v-for="message in messages"
                :key="message.title"
                class="message-row"
                :class="{ unread: !readMessages.includes(message.title) }"
                @click="markRead(message.title)"
              >
                <span class="message-dot"></span>
                <div>
                  <strong>{{ message.title }}</strong>
                  <small>{{ message.desc }}</small>
                </div>
                <em>{{ readMessages.includes(message.title) ? '已读' : message.time }}</em>
              </button>
            </article>

            <article v-else-if="item.type === 'chips'" class="white-card chips-card">
              <strong>可以这样补充</strong>
              <p>不会写也没关系，按您知道的情况选择或口述即可。</p>
              <div class="chips">
                <button v-for="chip in helpChips" :key="chip" @click="selectChip(chip)" :class="{ active: selectedChips.includes(chip) }">
                  {{ chip }}
                </button>
              </div>
            </article>

            <article v-else-if="item.type === 'map-location'" class="white-card map-card">
              <header>
                <div>
                  <strong>地图辅助定位</strong>
                  <p>项目名称说不清时，可先在北京市地图上点一个大概位置。</p>
                </div>
                <button @click="mapOpened = true">{{ mapOpened ? '地图已打开' : '打开地图' }}</button>
              </header>

              <section class="beijing-map" :class="{ opened: mapOpened }">
                <div class="district north">北部山区</div>
                <div class="district west">西部城区</div>
                <div class="district center">中心城区</div>
                <div class="district east active">XX区</div>
                <div class="district south">南部片区</div>
                <button class="map-pin" @click="mapOpened = true">
                  <span></span>
                  北京市XX区XX地点
                </button>
              </section>

              <div class="district-picker">
                <button
                  v-for="district in districtCandidates"
                  :key="district"
                  :class="{ active: selectedDistrict === district }"
                  @click="selectDistrict(district)"
                >
                  {{ district }}
                </button>
              </div>

              <div class="nearby-list">
                <button
                  v-for="place in nearbyPlaces"
                  :key="place.name"
                  :class="{ active: selectedNearby === place.name }"
                  @click="selectNearby(place.name)"
                >
                  <strong>{{ place.name }}</strong>
                  <small>{{ place.desc }}</small>
                </button>
              </div>

              <aside class="geo-check">
                <strong>行政区划确认</strong>
                <p>已按“定位点 + 逆地址 + 行政区边界”三步校验，当前归属：{{ selectedDistrict }} / XX街道。</p>
              </aside>
            </article>

            <article v-else-if="item.type === 'standard'" class="white-card standard-card">
              <header>
                <div>
                  <strong>已整理为标准申诉内容</strong>
                  <p>系统根据您的口述自动生成，可继续补充或修改。</p>
                </div>
                <span class="spark">✦</span>
              </header>
              <div v-for="field in standardFields" :key="field.label" class="field-row">
                <span class="field-icon">{{ field.icon }}</span>
                <span class="field-label">{{ field.label }}</span>
                <strong>{{ field.value }}</strong>
                <button @click="markEdited(field.label)">{{ editedFields.includes(field.label) ? '已改' : '编辑' }}</button>
              </div>
            </article>

            <article v-else-if="item.type === 'summary'" class="white-card summary-card">
              <header>
                <strong>申诉摘要</strong>
                <span>可保存</span>
              </header>
              <p>申请人反映，其于 2025 年 6 月至 2026 年 1 月在北京市XX区XX地点项目从事施工工作，由李某安排用工，尚有约 32000 元工资未结清。申请人目前能提供聊天催讨记录，工资结算和考勤材料仍需补充。</p>
              <div class="summary-actions">
                <button @click="draftSaved = true">{{ draftSaved ? '草稿已保存' : '保存草稿' }}</button>
                <button @click="nextStep">继续补材料</button>
              </div>
            </article>

            <aside v-else-if="item.type === 'tip'" class="warm-tip">
              <strong>{{ item.title }}</strong>
              <p>{{ item.text }}</p>
            </aside>

            <article v-else-if="item.type === 'evidence'" class="white-card list-card">
              <header class="section-title">
                <span></span>
                <strong>建议补充材料</strong>
                <em>{{ uploadedCount }}/6 已补</em>
              </header>
              <button
                v-for="row in evidenceItems"
                :key="row.title"
                class="evidence-row"
                :class="{ uploaded: uploadedEvidence.includes(row.title), optional: row.optional }"
                @click="toggleEvidence(row.title)"
              >
                <span class="doc-icon">{{ uploadedEvidence.includes(row.title) ? '✓' : row.optional ? '可' : '需' }}</span>
                <span>
                  <strong>{{ row.title }}</strong>
                  <small>{{ row.meta }}</small>
                </span>
                <em>{{ uploadedEvidence.includes(row.title) ? '已补' : '补充 ›' }}</em>
              </button>
            </article>

            <article v-else-if="item.type === 'incomplete'" class="white-card incomplete-card">
              <header>
                <strong>材料暂不完整，也可以先提交</strong>
                <span>{{ evidencePercent }}%</span>
              </header>
              <p>您可以先把已掌握的信息提交给接待人员，后续再从“我的线索”继续补充材料。</p>
              <div class="risk-grid">
                <div>
                  <strong>已具备</strong>
                  <span>口述事实、欠薪金额、用工线索</span>
                </div>
                <div>
                  <strong>待补充</strong>
                  <span>考勤、结算单、身份证明</span>
                </div>
              </div>
            </article>

            <article v-else-if="item.type === 'submit'" class="success-card">
              <div class="success-mark">✓</div>
              <strong>线索已提交</strong>
              <p>系统已同步生成申诉摘要、材料缺口和后续补证提醒。工作人员受理后将通过本页面更新进度。</p>
              <dl>
                <div>
                  <dt>线索编号</dt>
                  <dd>BJ-XX-QX-20260516-001</dd>
                </div>
                <div>
                  <dt>当前状态</dt>
                  <dd>待接收核验</dd>
                </div>
              </dl>
            </article>

            <article v-else-if="item.type === 'case-list'" class="white-card case-card">
              <header>
                <strong>我的线索</strong>
                <span>1 条进行中</span>
              </header>
              <button class="case-row" @click="caseOpened = true">
                <div>
                  <strong>北京市XX区XX地点欠薪线索</strong>
                  <small>BJ-XX-QX-20260516-001 · 待接收核验</small>
                </div>
                <em>{{ caseOpened ? '已打开' : '查看 ›' }}</em>
              </button>
            </article>

            <article v-else-if="item.type === 'progress'" class="white-card progress-card">
              <header>
                <strong>办理进度</strong>
                <span>材料链 {{ evidencePercent }}%</span>
              </header>
              <div v-for="node in progressNodes" :key="node.title" class="progress-node" :class="node.state">
                <span></span>
                <div>
                  <strong>{{ node.title }}</strong>
                  <small>{{ node.desc }}</small>
                </div>
              </div>
            </article>

            <article v-else-if="item.type === 'notice'" class="notice-card">
              <div class="notice-icon">铃</div>
              <div>
                <strong>新通知</strong>
                <p>检察服务窗口已接收线索，建议优先补充考勤或结算单据。您可继续返回本线索上传材料。</p>
              </div>
            </article>
          </div>
        </TransitionGroup>
      </section>

      <footer class="demo-controls">
        <button class="ghost-action" @click="prevStep" :disabled="stepIndex === 0">上一步</button>
        <button class="primary-action" @click="nextStep">{{ isLastStep ? '重新开始' : nextLabel }}</button>
        <button class="ghost-action" @click="saveDraft">{{ draftSaved ? '已保存' : '存草稿' }}</button>
      </footer>
    </section>
  </main>
</template>

<script setup>
import { computed, nextTick, ref } from 'vue'

const contentRef = ref(null)
const stepIndex = ref(0)
const editedFields = ref([])
const uploadedEvidence = ref([])
const selectedChips = ref([])
const draftSaved = ref(false)
const caseOpened = ref(false)
const profileEdited = ref(false)
const readMessages = ref([])
const mapOpened = ref(false)
const selectedDistrict = ref('北京市XX区')
const selectedNearby = ref('XX地点施工围挡')

const flowSteps = [
  { id: 'profile', type: 'profile', title: '个人中心', phase: '个人信息' },
  { id: 'messages', type: 'messages', title: '最新消息', phase: '消息通知' },
  { id: 'hello', type: 'bot', title: '欠薪线索反映', phase: '问询 1/6', text: '您好，请先不用担心格式。您只要像平时说话一样，说清楚谁欠了您多少钱。' },
  { id: 'user-basic', type: 'user', title: '欠薪线索反映', phase: '问询 1/6', text: '我在工地上班，老板欠我三个月工资没发。' },
  { id: 'ask-project', type: 'bot', title: '欠薪线索反映', phase: '追问 2/6', text: '这个工地大概叫什么名字？在哪个地方？如果记不清，附近商场、小区、路名也可以。' },
  { id: 'user-project', type: 'user', title: '欠薪线索反映', phase: '追问 2/6', text: '在北京市XX区XX地点那边，具体项目名字我记不太清。' },
  { id: 'map-location', type: 'map-location', title: '位置确认', phase: '定位 2/6' },
  { id: 'ask-person', type: 'bot', title: '欠薪线索反映', phase: '追问 2/6', text: '是谁联系您干活的？是包工头、劳务公司，还是项目上的负责人？' },
  { id: 'user-person', type: 'user', title: '欠薪线索反映', phase: '追问 2/6', text: '老李叫我去的，我也不知道公司全名，大家说是 XX 劳务。' },
  { id: 'ask-time-money', type: 'bot', title: '欠薪线索反映', phase: '追问 2/6', text: '您大概从什么时候干到什么时候？还差多少钱？可以说大概数。' },
  { id: 'user-time-money', type: 'user', title: '欠薪线索反映', phase: '追问 2/6', text: '去年6月干到快过年，还差我三万二。' },
  { id: 'chips', type: 'chips', title: '辅助补充', phase: '补充 3/6' },
  { id: 'standard', type: 'standard', title: '智能整理', phase: '整理 4/6' },
  { id: 'summary', type: 'summary', title: '申诉摘要', phase: '整理 4/6' },
  { id: 'tip', type: 'tip', title: '材料不足不用中断', phase: '材料 5/6', text: '没有完整证据也可以先保存或提交。系统会把缺失材料列出来，方便您之后继续补。' },
  { id: 'evidence', type: 'evidence', title: '材料补充', phase: '材料 5/6' },
  { id: 'incomplete', type: 'incomplete', title: '提交确认', phase: '提交 6/6' },
  { id: 'submit', type: 'submit', title: '提交确认', phase: '提交 6/6' },
  { id: 'case-list', type: 'case-list', title: '我的线索', phase: '进度查看' },
  { id: 'progress', type: 'progress', title: '办理进度', phase: '进度查看' },
  { id: 'notice', type: 'notice', title: '通知接收', phase: '进度查看' }
]

const helpChips = ['只记得包工头姓李', '没有签合同', '有微信聊天记录', '有部分转账记录', '工友可以作证', '考勤在班组长手机里']

const profileRows = [
  { label: '联系电话', value: '138****5521' },
  { label: '常住地址', value: '北京市XX区XX地点' },
  { label: '务工类型', value: '建筑施工 / 临时用工' },
  { label: '身份核验', value: '实名信息已核验' }
]

const messages = [
  { title: '草稿提醒', desc: '您有一条欠薪线索尚未补充材料，可继续填写。', time: '刚刚' },
  { title: '材料提示', desc: '聊天记录、转账记录可先作为初步材料上传。', time: '09:32' },
  { title: '服务通知', desc: '提交后可在“我的线索”查看接收、核验和反馈进度。', time: '昨日' }
]

const districtCandidates = ['北京市XX区', '北京市YY区', '北京市ZZ区']

const nearbyPlaces = [
  { name: 'XX地点施工围挡', desc: '距定位点约 80 米' },
  { name: 'XX路公交站', desc: '距定位点约 210 米' },
  { name: 'XX建材市场', desc: '距定位点约 360 米' }
]

const standardFields = [
  { icon: '▥', label: '项目地点', value: '北京市XX区XX地点' },
  { icon: '⌖', label: '行政区划', value: '北京市XX区 / XX街道' },
  { icon: '♙', label: '用工主体', value: '李某 / XX劳务公司（待核实）' },
  { icon: '▣', label: '工作期间', value: '2025.06 - 2026.01' },
  { icon: '¥', label: '欠薪金额', value: '约 32000 元' },
  { icon: '▤', label: '已有材料', value: '聊天记录、部分转账记录' }
]

const evidenceItems = [
  { short: '聊天记录', title: '催讨记录（聊天记录/通话录音等）', meta: '证明您曾催讨欠薪，可先上传截图', optional: false },
  { short: '转账记录', title: '工资支付凭证或转账记录', meta: '证明曾经发过工资或仍有差额', optional: false },
  { short: '工友证明', title: '同工地工友联系方式或证言', meta: '证明您实际参与该项目施工', optional: true },
  { short: '考勤记录', title: '考勤记录或进出场记录', meta: '证明工作时间，暂时没有可后补', optional: true },
  { short: '结算单', title: '工资结算单、欠条或班组记账', meta: '证明欠薪金额，能找到多少传多少', optional: true },
  { short: '身份信息', title: '本人身份信息和联系方式', meta: '便于工作人员联系核验', optional: false }
]

const progressNodes = computed(() => [
  { title: '线索已提交', desc: '申诉摘要和已有材料已保存', state: 'done' },
  { title: '检察服务窗口接收', desc: stepIndex.value >= 17 ? '已收到线索，等待人工核验' : '等待接收确认', state: stepIndex.value >= 17 ? 'done' : 'active' },
  { title: '材料链完善', desc: `${uploadedCount.value}/6 项材料已补充，可继续上传`, state: 'active' },
  { title: '处置反馈', desc: '后续通知将在本页同步', state: 'pending' }
])

const activeStep = computed(() => flowSteps[stepIndex.value])
const visibleSteps = computed(() => flowSteps.slice(0, stepIndex.value + 1))
const currentTitle = computed(() => activeStep.value.title)
const currentPhase = computed(() => activeStep.value.phase)
const progressPercent = computed(() => Math.round(((stepIndex.value + 1) / flowSteps.length) * 100))
const progressText = computed(() => `${progressPercent.value}%`)
const isLastStep = computed(() => stepIndex.value === flowSteps.length - 1)
const uploadedCount = computed(() => uploadedEvidence.value.length)
const evidencePercent = computed(() => Math.round((uploadedCount.value / evidenceItems.length) * 100))
const unreadCount = computed(() => messages.length - readMessages.value.length)

const nextLabel = computed(() => {
  if (activeStep.value.type === 'profile') return '查看消息'
  if (activeStep.value.type === 'messages') return '开始申诉'
  if (activeStep.value.type === 'map-location') return '确认位置'
  if (activeStep.value.type === 'summary') return draftSaved.value ? '继续补材料' : '保存后继续'
  if (activeStep.value.type === 'evidence') return '材料不足先提交'
  if (activeStep.value.type === 'submit') return '查看我的线索'
  if (activeStep.value.type === 'case-list') return '查看办理进度'
  if (activeStep.value.type === 'progress') return '接收通知'
  return '下一步'
})

const scrollBottom = async () => {
  await nextTick()
  if (contentRef.value) {
    contentRef.value.scrollTo({ top: contentRef.value.scrollHeight, behavior: 'smooth' })
  }
}

const nextStep = () => {
  if (isLastStep.value) {
    resetFlow()
    return
  }
  if (activeStep.value.type === 'summary') {
    draftSaved.value = true
  }
  stepIndex.value += 1
  if (activeStep.value.type === 'evidence' && uploadedEvidence.value.length === 0) {
    uploadedEvidence.value = [evidenceItems[0].title, evidenceItems[1].title]
  }
  scrollBottom()
}

const prevStep = () => {
  if (stepIndex.value > 0) {
    stepIndex.value -= 1
    scrollBottom()
  }
}

const saveDraft = () => {
  draftSaved.value = true
}

const resetFlow = () => {
  stepIndex.value = 0
  editedFields.value = []
  uploadedEvidence.value = []
  selectedChips.value = []
  draftSaved.value = false
  caseOpened.value = false
  profileEdited.value = false
  readMessages.value = []
  mapOpened.value = false
  selectedDistrict.value = '北京市XX区'
  selectedNearby.value = 'XX地点施工围挡'
  scrollBottom()
}

const jumpToMessages = () => {
  stepIndex.value = 1
  scrollBottom()
}

const markRead = (title) => {
  if (!readMessages.value.includes(title)) {
    readMessages.value = [...readMessages.value, title]
  }
}

const selectDistrict = (district) => {
  selectedDistrict.value = district
  mapOpened.value = true
}

const selectNearby = (place) => {
  selectedNearby.value = place
  mapOpened.value = true
}

const selectChip = (chip) => {
  if (!selectedChips.value.includes(chip)) {
    selectedChips.value = [...selectedChips.value, chip]
  }
}

const markEdited = (label) => {
  if (!editedFields.value.includes(label)) {
    editedFields.value = [...editedFields.value, label]
  }
}

const toggleEvidence = (title) => {
  if (uploadedEvidence.value.includes(title)) {
    uploadedEvidence.value = uploadedEvidence.value.filter((item) => item !== title)
    return
  }
  uploadedEvidence.value = [...uploadedEvidence.value, title]
}
</script>

<style scoped>
.mobile-shell {
  min-height: 100vh;
  background: #e7f0ff;
  display: flex;
  justify-content: center;
  align-items: stretch;
  font-family: "PingFang SC", "Microsoft YaHei", sans-serif;
  color: #1d2433;
}

.phone-stage {
  width: min(430px, 100vw);
  min-height: 100vh;
  background: linear-gradient(180deg, #ffffff 0%, #f6f9fe 100%);
  position: relative;
  display: flex;
  flex-direction: column;
  box-shadow: 0 0 40px rgba(23, 95, 210, 0.16);
}

.status-bar,
.top-nav {
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 22px;
  background: rgba(255, 255, 255, 0.96);
}

.status-bar {
  font-size: 15px;
  font-weight: 800;
}

.status-icons {
  font-size: 12px;
}

.top-nav {
  height: 56px;
  border-bottom: 1px solid #eef2f8;
}

.top-nav h1 {
  font-size: 20px;
  margin: 0;
  font-weight: 900;
  letter-spacing: 0;
}

button {
  border: 0;
  font-family: inherit;
  cursor: pointer;
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.icon-button {
  width: 40px;
  background: transparent;
  font-size: 34px;
  line-height: 1;
  color: #111827;
}

.mini-actions {
  width: 62px;
  height: 32px;
  border-radius: 18px;
  background: #ffffff;
  border: 1px solid #eef2f8;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}

.mini-actions span {
  width: 5px;
  height: 5px;
  background: #111827;
  border-radius: 50%;
}

.flow-progress {
  padding: 14px 22px 12px;
  background: rgba(255, 255, 255, 0.96);
}

.progress-copy {
  display: flex;
  justify-content: space-between;
  font-size: 15px;
  font-weight: 800;
  margin-bottom: 10px;
}

.progress-track {
  height: 9px;
  background: #e5e9ef;
  border-radius: 999px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #1266f1, #3d8bff);
  border-radius: inherit;
  transition: width 360ms ease;
}

.content {
  flex: 1;
  overflow-y: auto;
  padding: 16px 18px 98px;
  scroll-behavior: smooth;
}

.notice-line {
  color: #778399;
  font-size: 13px;
  text-align: center;
  display: flex;
  justify-content: center;
  gap: 6px;
  margin-bottom: 20px;
}

.shield,
.spark {
  color: #1266f1;
  font-weight: 900;
}

.slide-stack {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.flow-block.highlight {
  animation: focus-pulse 760ms ease;
}

.chat-row {
  display: flex;
  gap: 10px;
  align-items: flex-start;
}

.chat-row.user {
  justify-content: flex-end;
}

.bot-avatar,
.user-avatar {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  flex: 0 0 auto;
}

.bot-avatar {
  background: linear-gradient(180deg, #1474ff, #0b58df);
  color: #ffffff;
  display: grid;
  place-items: center;
  font-size: 13px;
  font-weight: 900;
  box-shadow: 0 8px 18px rgba(18, 102, 241, 0.22);
}

.user-avatar {
  background: #1266f1;
  position: relative;
}

.user-avatar::before,
.user-avatar::after {
  content: "";
  position: absolute;
  background: #ffffff;
  left: 50%;
  transform: translateX(-50%);
}

.user-avatar::before {
  width: 10px;
  height: 10px;
  top: 8px;
  border-radius: 50%;
}

.user-avatar::after {
  width: 19px;
  height: 9px;
  top: 21px;
  border-radius: 10px 10px 4px 4px;
}

.bubble {
  max-width: 285px;
  background: #ffffff;
  border-radius: 8px;
  padding: 14px 16px;
  font-size: 16px;
  line-height: 1.65;
  box-shadow: 0 8px 24px rgba(40, 61, 96, 0.08);
}

.user .bubble {
  background: #1266f1;
  color: #ffffff;
  border-radius: 8px 8px 2px 8px;
}

.profile-hero,
.white-card,
.warm-tip,
.success-card,
.notice-card {
  background: #ffffff;
  border-radius: 10px;
  box-shadow: 0 8px 24px rgba(40, 61, 96, 0.08);
}

.profile-page {
  display: grid;
  gap: 14px;
}

.profile-hero {
  min-height: 96px;
  display: grid;
  grid-template-columns: 54px 1fr auto;
  align-items: center;
  gap: 12px;
  padding: 18px;
  background: linear-gradient(135deg, #1266f1, #4d95ff);
  color: #ffffff;
}

.profile-avatar {
  width: 54px;
  height: 54px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.18);
  display: grid;
  place-items: center;
  font-size: 22px;
  font-weight: 900;
}

.profile-hero strong {
  display: block;
  font-size: 19px;
}

.profile-hero p {
  margin: 5px 0 0;
  font-size: 13px;
  opacity: 0.9;
}

.verified-badge {
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.18);
  padding: 6px 10px;
  font-size: 12px;
  font-weight: 900;
}

.profile-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}

.profile-grid div {
  min-height: 72px;
  background: #ffffff;
  border-radius: 10px;
  display: grid;
  align-content: center;
  justify-items: center;
  gap: 4px;
  box-shadow: 0 8px 24px rgba(40, 61, 96, 0.08);
}

.profile-grid strong {
  color: #1266f1;
  font-size: 18px;
}

.profile-grid span {
  color: #66748a;
  font-size: 12px;
}

.chips-card,
.standard-card,
.summary-card,
.list-card,
.incomplete-card,
.case-card,
.progress-card,
.profile-card,
.messages-card,
.map-card {
  padding: 16px;
}

.chips-card strong,
.standard-card strong,
.summary-card strong,
.section-title strong,
.incomplete-card strong,
.case-card strong,
.progress-card strong,
.profile-card strong,
.messages-card strong,
.map-card strong {
  font-size: 17px;
}

.chips-card p,
.standard-card p,
.summary-card p,
.incomplete-card p,
.map-card p {
  color: #66748a;
  line-height: 1.7;
}

.chips {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}

.chips button,
.summary-actions button {
  min-height: 40px;
  border-radius: 8px;
  background: #f1f6ff;
  color: #4b5f7a;
  font-weight: 800;
}

.chips button.active,
.summary-actions button:last-child {
  background: #1266f1;
  color: #ffffff;
}

.standard-card header,
.summary-card header,
.incomplete-card header,
.case-card header,
.progress-card header,
.profile-card header,
.messages-card header,
.map-card header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
}

.map-card header button {
  flex: 0 0 auto;
  background: #1266f1;
  color: #ffffff;
  border-radius: 999px;
  padding: 8px 12px;
  font-weight: 900;
}

.beijing-map {
  height: 190px;
  position: relative;
  overflow: hidden;
  border-radius: 12px;
  background:
    linear-gradient(135deg, rgba(18, 102, 241, 0.1), rgba(255, 255, 255, 0.4)),
    #eef5ff;
  border: 1px solid #dce8fb;
  margin: 12px 0;
}

.beijing-map::before {
  content: "";
  position: absolute;
  inset: 18px 26px;
  border-radius: 48% 52% 45% 55%;
  border: 2px solid rgba(18, 102, 241, 0.24);
  background: rgba(255, 255, 255, 0.46);
}

.beijing-map.opened {
  box-shadow: inset 0 0 0 2px rgba(18, 102, 241, 0.28);
}

.district {
  position: absolute;
  min-width: 70px;
  height: 34px;
  border-radius: 18px;
  display: grid;
  place-items: center;
  background: rgba(255, 255, 255, 0.82);
  color: #66748a;
  font-size: 12px;
  font-weight: 900;
  border: 1px solid rgba(18, 102, 241, 0.12);
}

.district.north {
  top: 24px;
  left: 126px;
}

.district.west {
  top: 82px;
  left: 44px;
}

.district.center {
  top: 76px;
  left: 145px;
}

.district.east {
  top: 84px;
  right: 38px;
}

.district.south {
  bottom: 22px;
  left: 128px;
}

.district.active {
  background: #1266f1;
  color: #ffffff;
  box-shadow: 0 8px 20px rgba(18, 102, 241, 0.22);
}

.map-pin {
  position: absolute;
  right: 40px;
  top: 124px;
  display: flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 10px;
  border-radius: 16px;
  background: #ffffff;
  color: #1d2433;
  font-size: 12px;
  font-weight: 900;
  box-shadow: 0 10px 24px rgba(40, 61, 96, 0.16);
}

.map-pin span {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #ff4857;
  box-shadow: 0 0 0 5px rgba(255, 72, 87, 0.18);
}

.district-picker,
.nearby-list {
  display: grid;
  gap: 8px;
}

.district-picker {
  grid-template-columns: repeat(3, 1fr);
  margin-bottom: 10px;
}

.district-picker button {
  min-height: 36px;
  border-radius: 8px;
  background: #f1f6ff;
  color: #526071;
  font-weight: 900;
}

.district-picker button.active {
  background: #1266f1;
  color: #ffffff;
}

.nearby-list {
  margin-bottom: 12px;
}

.nearby-list button {
  min-height: 54px;
  display: grid;
  grid-template-columns: 1fr;
  gap: 4px;
  text-align: left;
  border-radius: 8px;
  background: #f6f9fe;
  padding: 10px 12px;
}

.nearby-list button.active {
  background: #edf5ff;
  box-shadow: inset 0 0 0 1px rgba(18, 102, 241, 0.22);
}

.nearby-list small {
  color: #7b8798;
}

.geo-check {
  border-radius: 10px;
  background: #eef7f2;
  padding: 12px;
  color: #49675a;
}

.geo-check strong {
  color: #1a8f58;
}

.geo-check p {
  margin: 6px 0 0;
  font-size: 13px;
}

.profile-card header button {
  background: #edf5ff;
  color: #1266f1;
  border-radius: 999px;
  padding: 6px 12px;
  font-weight: 900;
}

.profile-row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 11px 0;
  border-top: 1px solid #edf1f6;
}

.profile-row span {
  color: #7b8798;
}

.profile-row strong {
  font-size: 14px;
  text-align: right;
}

.entry-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.entry-actions button {
  min-height: 48px;
  border-radius: 10px;
  background: #1266f1;
  color: #ffffff;
  font-weight: 900;
  box-shadow: 0 8px 22px rgba(18, 102, 241, 0.18);
}

.entry-actions button:last-child {
  background: #ffffff;
  color: #1266f1;
}

.messages-card header span {
  color: #1266f1;
  font-size: 13px;
  font-weight: 900;
}

.message-row {
  width: 100%;
  min-height: 72px;
  background: #ffffff;
  display: grid;
  grid-template-columns: 10px 1fr auto;
  align-items: center;
  gap: 12px;
  padding: 12px 0;
  text-align: left;
  border-top: 1px solid #edf1f6;
}

.message-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #cbd5e1;
}

.message-row.unread .message-dot {
  background: #1266f1;
}

.message-row strong {
  display: block;
  font-size: 15px;
}

.message-row small {
  display: block;
  color: #7b8798;
  margin-top: 5px;
  line-height: 1.5;
}

.message-row em {
  color: #8a95a6;
  font-style: normal;
  font-size: 12px;
  font-weight: 900;
}

.standard-card header p {
  margin: 4px 0 0;
  color: #8a95a6;
  font-size: 13px;
}

.field-row {
  min-height: 48px;
  display: grid;
  grid-template-columns: 28px 78px 1fr 44px;
  align-items: center;
  gap: 8px;
  font-size: 15px;
}

.field-icon {
  color: #1266f1;
  font-weight: 900;
}

.field-label {
  color: #7b8798;
}

.field-row strong {
  color: #263244;
}

.field-row button {
  background: transparent;
  color: #1266f1;
  font-size: 14px;
  font-weight: 900;
}

.summary-card header span,
.incomplete-card header span,
.case-card header span,
.progress-card header span {
  color: #1266f1;
  font-size: 13px;
  font-weight: 900;
}

.summary-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  margin-top: 14px;
}

.warm-tip {
  background: #eaf3ff;
  padding: 14px 16px;
  color: #5c6d84;
  font-size: 13px;
}

.warm-tip strong {
  color: #276bd8;
}

.warm-tip p {
  margin: 6px 0 0;
  line-height: 1.7;
}

.section-title {
  display: grid;
  grid-template-columns: 5px 1fr auto;
  gap: 10px;
  align-items: center;
  margin: 0 0 12px;
}

.section-title span {
  width: 5px;
  height: 24px;
  background: #1266f1;
  border-radius: 999px;
}

.section-title em {
  font-style: normal;
  color: #8a95a6;
  font-size: 13px;
}

.evidence-row {
  width: 100%;
  min-height: 74px;
  background: #ffffff;
  display: grid;
  grid-template-columns: 38px 1fr auto;
  align-items: center;
  gap: 12px;
  padding: 12px 0;
  text-align: left;
  border-top: 1px solid #edf1f6;
}

.doc-icon {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-weight: 900;
  background: #edf5ff;
  color: #1266f1;
  font-size: 12px;
}

.evidence-row.uploaded .doc-icon {
  background: #1266f1;
  color: #ffffff;
}

.evidence-row.optional .doc-icon {
  background: #eef7f2;
  color: #1a8f58;
}

.evidence-row strong {
  display: block;
  font-size: 15px;
  color: #263244;
}

.evidence-row small {
  display: block;
  margin-top: 4px;
  font-size: 12px;
  color: #8a95a6;
}

.evidence-row em {
  color: #1266f1;
  font-style: normal;
  font-size: 14px;
  font-weight: 900;
}

.risk-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.risk-grid div {
  background: #f5f8fd;
  border-radius: 8px;
  padding: 12px;
}

.risk-grid strong,
.risk-grid span {
  display: block;
}

.risk-grid span {
  color: #66748a;
  font-size: 12px;
  line-height: 1.6;
  margin-top: 5px;
}

.success-card {
  text-align: center;
  padding: 22px 18px;
}

.success-mark {
  width: 54px;
  height: 54px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  margin: 0 auto 12px;
  font-size: 28px;
  font-weight: 900;
  background: #1266f1;
  color: #ffffff;
}

.success-card p {
  color: #66748a;
  line-height: 1.7;
}

.success-card dl {
  margin: 14px 0 0;
  display: grid;
  gap: 8px;
}

.success-card dl div {
  display: flex;
  justify-content: space-between;
  padding: 10px 12px;
  background: #f5f8fd;
  border-radius: 8px;
}

.success-card dt,
.success-card dd {
  margin: 0;
  font-size: 13px;
}

.success-card dt {
  color: #7b8798;
}

.success-card dd {
  color: #263244;
  font-weight: 900;
}

.case-row {
  width: 100%;
  background: #f6f9fe;
  border-radius: 10px;
  padding: 14px;
  display: flex;
  justify-content: space-between;
  text-align: left;
  align-items: center;
}

.case-row small {
  display: block;
  color: #7b8798;
  margin-top: 5px;
}

.case-row em {
  color: #1266f1;
  font-style: normal;
  font-weight: 900;
}

.progress-node {
  display: grid;
  grid-template-columns: 28px 1fr;
  gap: 10px;
  padding: 10px 0;
  border-top: 1px solid #edf1f6;
}

.progress-node > span {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #d7dde7;
  margin-top: 2px;
}

.progress-node.done > span,
.progress-node.active > span {
  background: #1266f1;
}

.progress-node small {
  color: #7b8798;
  display: block;
  margin-top: 4px;
}

.notice-card {
  display: grid;
  grid-template-columns: 48px 1fr;
  gap: 12px;
  padding: 18px;
  background: #eef6ff;
}

.notice-icon {
  width: 42px;
  height: 42px;
  background: #1266f1;
  color: #ffffff;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-weight: 900;
}

.notice-card p {
  margin: 5px 0 0;
  color: #5c6d84;
  line-height: 1.7;
}

.demo-controls {
  position: fixed;
  left: 50%;
  bottom: 14px;
  transform: translateX(-50%);
  width: min(390px, calc(100vw - 28px));
  height: 58px;
  border-radius: 30px;
  background: rgba(255, 255, 255, 0.94);
  box-shadow: 0 12px 30px rgba(31, 51, 86, 0.18);
  display: grid;
  grid-template-columns: 72px 1fr 72px;
  gap: 8px;
  padding: 7px;
  z-index: 10;
}

.primary-action,
.ghost-action {
  border-radius: 23px;
  font-weight: 900;
}

.primary-action {
  background: #1266f1;
  color: #ffffff;
  font-size: 15px;
}

.ghost-action {
  background: #f0f4fa;
  color: #607086;
  font-size: 13px;
}

.ppt-enter-active,
.ppt-leave-active {
  transition: all 360ms cubic-bezier(0.2, 0.8, 0.2, 1);
}

.ppt-enter-from {
  opacity: 0;
  transform: translateY(22px) scale(0.98);
}

.ppt-leave-to {
  opacity: 0;
  transform: translateY(-10px) scale(0.98);
}

@keyframes focus-pulse {
  0% {
    filter: drop-shadow(0 0 0 rgba(18, 102, 241, 0));
  }
  45% {
    filter: drop-shadow(0 10px 18px rgba(18, 102, 241, 0.16));
  }
  100% {
    filter: drop-shadow(0 0 0 rgba(18, 102, 241, 0));
  }
}

@media (min-width: 720px) {
  .mobile-shell {
    padding: 28px 0;
    align-items: center;
  }

  .phone-stage {
    min-height: 844px;
    max-height: 900px;
    border-radius: 38px;
    overflow: hidden;
  }
}
</style>

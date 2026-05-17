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
      <div v-for="(step, index) in workflowSteps" :key="step.name" class="flow-step" :class="{ active: index <= 5 }">
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
              <span>{{ cases.length }} 条</span>
            </div>
            <button>批量核验</button>
          </header>

          <div class="search-row">
            <input value="欠薪 / XX区 / 地图定位" readonly />
            <button>筛选</button>
          </div>

          <div class="status-tabs">
            <button class="active">全部 {{ cases.length }}</button>
            <button>待核验 4</button>
            <button>待补证 6</button>
            <button>已回传 9</button>
          </div>

          <button
            v-for="item in cases"
            :key="item.id"
            class="case-item"
            :class="{ active: item.id === activeCase.id }"
            @click="activeCase = item"
          >
            <div class="risk-badge" :class="item.risk">{{ item.riskText }}</div>
            <div class="case-main">
              <strong>{{ item.title }}</strong>
              <span>{{ item.worker }} · {{ item.location }}</span>
              <small>{{ item.source }} · {{ item.updated }}</small>
            </div>
            <em>{{ item.score }}</em>
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
        <div class="panel-card review-panel">
          <header class="review-header">
            <div>
              <span class="kicker">AI STANDARDIZED CASE FILE</span>
              <h2>欠薪线索核验工作台</h2>
              <p>{{ activeCase.id }} · {{ activeCase.status }}</p>
            </div>
            <div class="header-actions">
              <button>退回补证</button>
              <button class="primary">提交人工复核</button>
            </div>
          </header>

          <nav class="doc-tabs">
            <button :class="{ active: activeTab === 'summary' }" @click="activeTab = 'summary'">AI研判摘要</button>
            <button :class="{ active: activeTab === 'raw' }" @click="activeTab = 'raw'">移动端原始信息</button>
            <button :class="{ active: activeTab === 'notice' }" @click="activeTab = 'notice'">回传通知</button>
          </nav>

          <section v-if="activeTab === 'summary'" class="doc-body">
            <article class="standard-section">
              <header>
                <strong>一、标准化诉求摘要</strong>
                <button @click="draftSaved = true">{{ draftSaved ? '草稿已保存' : '保存草稿' }}</button>
              </header>
              <p>
                申请人{{ activeCase.worker }}反映，其于 2025 年 6 月至 2026 年 1 月在{{ activeCase.location }}项目从事施工工作，
                由李某安排用工，尚有约 32000 元工资未结清。申请人可提供聊天催讨记录、部分转账记录，考勤和结算材料仍需补充。
              </p>
            </article>

            <article class="standard-section">
              <header>
                <strong>二、关键信息抽取</strong>
                <span>来自移动端逐步问询</span>
              </header>
              <div class="field-grid">
                <div v-for="field in standardizedFields" :key="field.label">
                  <span>{{ field.label }}</span>
                  <strong>{{ field.value }}</strong>
                </div>
              </div>
            </article>

            <article class="standard-section">
              <header>
                <strong>三、证据链完整性</strong>
                <span>{{ evidenceCompletion }}%</span>
              </header>
              <div class="evidence-track">
                <div class="evidence-fill" :style="{ width: `${evidenceCompletion}%` }"></div>
              </div>
              <div class="evidence-checks">
                <span v-for="item in evidenceChecks" :key="item.name" :class="{ ok: item.ok }">{{ item.name }}</span>
              </div>
            </article>

            <article class="warning-note">
              提示：本页内容由移动端提交信息与模型整理结果生成，需检察人员复核后用于后续办理。
            </article>
          </section>

          <section v-else-if="activeTab === 'raw'" class="doc-body raw-body">
            <article v-for="message in rawMessages" :key="message.text" :class="['raw-message', message.role]">
              <span>{{ message.role === 'ai' ? 'AI追问' : '申请人口述' }}</span>
              <p>{{ message.text }}</p>
            </article>
          </section>

          <section v-else class="doc-body notice-body">
            <article v-for="notice in notices" :key="notice.title" class="notice-item">
              <div>
                <strong>{{ notice.title }}</strong>
                <p>{{ notice.text }}</p>
              </div>
              <button @click="notice.sent = true">{{ notice.sent ? '已回传' : '发送' }}</button>
            </article>
          </section>
        </div>
      </main>

      <aside class="trace-column">
        <div class="panel-card trace-grid">
          <header class="panel-header compact">
            <div>
              <strong>提交信息全量视图</strong>
              <span>来自农民工移动端</span>
            </div>
          </header>

          <article class="trace-card">
            <header>
              <strong>申请人信息</strong>
              <button>查看实名信息</button>
            </header>
            <p>姓名：张某某</p>
            <p>电话：138****5521</p>
            <p>身份：建筑施工临时用工</p>
          </article>

          <article class="trace-card">
            <header>
              <strong>地图定位与行政区划</strong>
              <button>查看定位</button>
            </header>
            <p>定位点：北京市XX区XX地点</p>
            <p>周边参照：XX地点施工围挡、XX路公交站</p>
            <p>校验：定位点 + 逆地址 + 行政区边界</p>
            <div class="confidence">置信度 92%</div>
          </article>

          <article class="trace-card">
            <header>
              <strong>移动端草稿记录</strong>
              <button>查看草稿</button>
            </header>
            <p>保存时间：2026-05-16 09:41</p>
            <p>草稿状态：已转正式线索</p>
            <p>补证入口：已开放</p>
          </article>

          <article class="trace-card">
            <header>
              <strong>材料清单</strong>
              <button>催补材料</button>
            </header>
            <div v-for="item in materialList" :key="item.name" class="material-row">
              <span :class="{ ok: item.ok }"></span>
              <strong>{{ item.name }}</strong>
              <em>{{ item.ok ? '已提交' : '待补充' }}</em>
            </div>
          </article>

          <article class="trace-card">
            <header>
              <strong>进度同步</strong>
              <button>同步移动端</button>
            </header>
            <div v-for="node in progressNodes" :key="node.name" class="mini-progress" :class="node.state">
              <span></span>
              <div>
                <strong>{{ node.name }}</strong>
                <small>{{ node.desc }}</small>
              </div>
            </div>
          </article>

          <article class="trace-card">
            <header>
              <strong>通知接收</strong>
              <button>查看记录</button>
            </header>
            <p>最新通知：请优先补充考勤或结算单据</p>
            <p>接收状态：移动端已送达，未读</p>
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

const mapRef = ref(null)
const activeTab = ref('summary')
const draftSaved = ref(false)
let mapIns = null

const metrics = [
  { icon: '＋', tone: 'blue', label: '今日移动端线索', value: '128', delta: '较昨日 +16.3%' },
  { icon: '⌁', tone: 'orange', label: '待核验诉求', value: '356', delta: '较昨日 +8.7%' },
  { icon: '!', tone: 'red', label: '材料待补', value: '72', delta: '较昨日 +12.5%' },
  { icon: '人', tone: 'purple', label: '已转人工', value: '31', delta: '较昨日 -5.1%' },
  { icon: '文', tone: 'blue', label: '文书草稿', value: '86', delta: '较昨日 +14.6%' },
  { icon: '时', tone: 'cyan', label: '平均处理时长', value: '2.6h', delta: '较昨日 -0.4h' },
  { icon: '位', tone: 'blue', label: '定位确认率', value: '91.2%', delta: '较昨日 +2.1%' },
  { icon: '✓', tone: 'orange', label: '闭环完成率', value: '8.7%', delta: '较昨日 -1.3%' }
]

const workflowSteps = [
  { icon: '手', name: '移动端提交', desc: '口述与定位' },
  { icon: 'AI', name: 'AI标准化', desc: '字段抽取' },
  { icon: '图', name: '行政区划确认', desc: '边界校验' },
  { icon: '证', name: '证据链校验', desc: '缺口提示' },
  { icon: '检', name: '人工复核', desc: '检察官确认' },
  { icon: '文', name: '文书生成', desc: '建议草稿' },
  { icon: '回', name: '通知回传', desc: '移动端同步' },
  { icon: '档', name: '归档监督', desc: '闭环留痕' }
]

const cases = [
  {
    id: 'BJ-XX-QX-20260516-001',
    title: '北京市XX区XX地点欠薪线索',
    worker: '张某某',
    location: '北京市XX区XX地点',
    source: '移动端地图定位',
    updated: '10分钟前',
    risk: 'high',
    riskText: '高',
    score: '92.0',
    status: '待接收核验'
  },
  {
    id: 'BJ-XX-QX-20260516-002',
    title: '施工班组工资结算争议',
    worker: '王某某',
    location: '北京市XX区XX地点',
    source: '移动端草稿转正式',
    updated: '32分钟前',
    risk: 'medium',
    riskText: '中',
    score: '84.6',
    status: '待补充考勤'
  },
  {
    id: 'BJ-XX-QX-20260516-003',
    title: '劳务公司拖欠尾款诉求',
    worker: '李某某',
    location: '北京市XX区XX地点',
    source: '移动端问询',
    updated: '1小时前',
    risk: 'medium',
    riskText: '中',
    score: '78.4',
    status: '待人工复核'
  },
  {
    id: 'BJ-XX-QX-20260516-004',
    title: '未签合同工友集体欠薪',
    worker: '赵某某',
    location: '北京市XX区XX地点',
    source: '移动端补证',
    updated: '2小时前',
    risk: 'low',
    riskText: '低',
    score: '65.2',
    status: '已回传补证通知'
  }
]

const activeCase = ref(cases[0])

const standardizedFields = computed(() => [
  { label: '申请人', value: activeCase.value.worker },
  { label: '项目地点', value: activeCase.value.location },
  { label: '行政区划', value: '北京市XX区 / XX街道' },
  { label: '用工主体', value: '李某 / XX劳务公司（待核实）' },
  { label: '工作期间', value: '2025.06 - 2026.01' },
  { label: '欠薪金额', value: '约 32000 元' }
])

const rawMessages = [
  { role: 'ai', text: '这个工地大概叫什么名字？在哪个地方？如果记不清，附近商场、小区、路名也可以。' },
  { role: 'user', text: '在北京市XX区XX地点那边，具体项目名字我记不太清。' },
  { role: 'ai', text: '项目名称说不清时，可以用地图点一个大概位置。' },
  { role: 'user', text: '我点的位置是在XX地点施工围挡附近。' },
  { role: 'ai', text: '是谁联系您干活的？是包工头、劳务公司，还是项目上的负责人？' },
  { role: 'user', text: '老李叫我去的，我也不知道公司全名，大家说是 XX 劳务。' },
  { role: 'ai', text: '您大概从什么时候干到什么时候？还差多少钱？可以说大概数。' },
  { role: 'user', text: '去年6月干到快过年，还差我三万二。' }
]

const evidenceChecks = [
  { name: '身份信息', ok: true },
  { name: '事实要素', ok: true },
  { name: '地图定位', ok: true },
  { name: '聊天记录', ok: true },
  { name: '考勤/结算', ok: false },
  { name: '主体证明', ok: false }
]

const materialList = [
  { name: '本人身份信息', ok: true },
  { name: '聊天催讨记录', ok: true },
  { name: '部分转账记录', ok: true },
  { name: '考勤记录', ok: false },
  { name: '结算单/欠条', ok: false },
  { name: '工友证明', ok: false }
]

const progressNodes = [
  { name: '移动端线索提交', desc: '已生成标准化摘要', state: 'done' },
  { name: '检察端接收核验', desc: '待承办人确认', state: 'active' },
  { name: '材料链完善', desc: '已回传补证清单', state: 'active' },
  { name: '处置反馈', desc: '等待后续结果同步', state: 'pending' }
]

const notices = ref([
  { title: '补充材料通知', text: '请优先补充考勤记录、工资结算单或工友证明。', sent: false },
  { title: '受理进度通知', text: '您的线索已进入检察服务窗口接收核验流程。', sent: false },
  { title: '定位确认通知', text: '系统已根据地图点位确认行政区划为北京市XX区/XX街道。', sent: true }
])

const evidenceCompletion = computed(() => Math.round((evidenceChecks.filter((item) => item.ok).length / evidenceChecks.length) * 100))

const initMap = async () => {
  await nextTick()
  if (!mapRef.value) return
  mapIns = echarts.init(mapRef.value)
  echarts.registerMap('tongzhou-appeal', tongzhouGeoJson)
  mapIns.setOption({
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'item',
      formatter: '{b}'
    },
    geo: {
      map: 'tongzhou-appeal',
      roam: false,
      zoom: 1.08,
      itemStyle: {
        areaColor: '#f8fbff',
        borderColor: '#b9cced',
        borderWidth: 1
      },
      emphasis: {
        itemStyle: {
          areaColor: '#e8f1ff'
        },
        label: {
          color: '#173a7a'
        }
      }
    },
    series: [
      {
        type: 'effectScatter',
        coordinateSystem: 'geo',
        rippleEffect: { scale: 3.2, brushType: 'stroke' },
        data: [
          { name: '移动端欠薪线索-北京市XX区XX地点', value: [116.656, 39.909], symbolSize: 15, itemStyle: { color: '#ef4444' } },
          { name: '材料待补线索', value: [116.684, 39.807], symbolSize: 11, itemStyle: { color: '#f59e0b' } },
          { name: '普通诉求定位点', value: [116.735, 39.88], symbolSize: 9, itemStyle: { color: '#2563eb' } }
        ]
      }
    ]
  })
}

onMounted(() => {
  initMap()
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

.metric-strip {
  display: grid;
  grid-template-columns: repeat(8, minmax(132px, 1fr));
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
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-weight: 900;
}

.metric-icon.blue { background: #eaf2ff; color: #1f63d1; }
.metric-icon.orange { background: #fff2e5; color: #e7791a; }
.metric-icon.red { background: #fff0f0; color: #d9363e; }
.metric-icon.purple { background: #f0ecff; color: #5b45d6; }
.metric-icon.cyan { background: #e8f8ff; color: #1282a8; }

.metric-card span,
.metric-card small,
.panel-header span,
.standard-section span {
  color: #687793;
  font-size: 12px;
}

.metric-card strong {
  display: block;
  font-size: 20px;
  color: #173a7a;
  line-height: 1.1;
}

.metric-card small {
  color: #0f8a50;
}

.flow-strip {
  min-height: 100px;
  display: grid;
  grid-template-columns: 108px repeat(8, minmax(110px, 1fr));
  gap: 8px;
  align-items: center;
  padding: 12px;
  margin-bottom: 12px;
}

.flow-title {
  color: #174ea6;
  font-weight: 900;
  border-right: 1px solid #dbe5f3;
  height: 64px;
  display: grid;
  place-items: center;
  text-align: center;
}

.flow-step {
  min-height: 74px;
  border-radius: 8px;
  display: grid;
  justify-items: center;
  align-content: center;
  gap: 4px;
  background: #f7faff;
  color: #65738e;
  position: relative;
}

.flow-step:not(:last-child)::after {
  content: "›";
  position: absolute;
  right: -9px;
  top: 28px;
  color: #8aa3c7;
  font-size: 22px;
}

.flow-step.active {
  background: #eef5ff;
  color: #174ea6;
}

.flow-step span {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  background: #2563eb;
  color: #ffffff;
  font-size: 12px;
  font-weight: 900;
}

.flow-step strong {
  font-size: 13px;
}

.flow-step small {
  color: #7b8798;
  font-size: 11px;
}

.main-grid {
  display: grid;
  grid-template-columns: 410px minmax(520px, 1fr) 410px;
  gap: 12px;
  min-height: calc(100vh - 250px);
}

.queue-column,
.trace-column {
  display: grid;
  grid-template-rows: 1fr 300px;
  gap: 12px;
  min-height: 0;
}

.trace-column {
  grid-template-rows: 1fr;
}

.panel-card {
  padding: 14px;
  min-height: 0;
}

.panel-header,
.review-header,
.standard-section header,
.trace-card header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-start;
}

.panel-header strong,
.review-header h2,
.standard-section strong,
.trace-card strong {
  color: #173a7a;
}

.panel-header button,
.header-actions button,
.standard-section button,
.trace-card button,
.notice-item button {
  border: 1px solid #d5e2f5;
  background: #f7fbff;
  color: #174ea6;
  border-radius: 6px;
  padding: 7px 10px;
  font-weight: 800;
}

.header-actions .primary,
.notice-item button {
  background: #1d5fd3;
  color: #ffffff;
  border-color: #1d5fd3;
}

.search-row {
  display: grid;
  grid-template-columns: 1fr 64px;
  gap: 8px;
  margin: 12px 0;
}

.search-row input {
  border: 1px solid #dce6f5;
  border-radius: 6px;
  padding: 0 10px;
  color: #6b7280;
}

.search-row button,
.status-tabs button {
  border: 0;
  border-radius: 6px;
  background: #eef5ff;
  color: #174ea6;
  font-weight: 800;
}

.status-tabs {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
  margin-bottom: 10px;
}

.status-tabs button {
  min-height: 30px;
  font-size: 12px;
}

.status-tabs .active {
  background: #1d5fd3;
  color: #ffffff;
}

.case-item {
  width: 100%;
  display: grid;
  grid-template-columns: 42px 1fr 42px;
  gap: 10px;
  align-items: center;
  border: 1px solid #edf2fa;
  background: #ffffff;
  border-radius: 8px;
  padding: 10px;
  text-align: left;
  margin-bottom: 8px;
}

.case-item.active {
  border-color: #1d5fd3;
  background: #f5f9ff;
}

.risk-badge {
  width: 34px;
  height: 34px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  font-weight: 900;
}

.risk-badge.high { background: #fff0f0; color: #d9363e; }
.risk-badge.medium { background: #fff7e8; color: #d88900; }
.risk-badge.low { background: #edf8f0; color: #118a45; }

.case-main strong,
.trace-card p {
  display: block;
  color: #253858;
  font-size: 13px;
}

.case-main span,
.case-main small {
  display: block;
  color: #66748a;
  font-size: 12px;
  margin-top: 3px;
}

.case-item em {
  font-style: normal;
  color: #174ea6;
  font-weight: 900;
}

.map-panel {
  min-height: 0;
}

.tongzhou-map {
  height: 220px;
}

.map-legend {
  display: flex;
  gap: 10px;
  font-size: 12px;
  color: #66748a;
}

.map-legend i {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 4px;
}

.map-legend .red { background: #ef4444; }
.map-legend .blue { background: #2563eb; }
.map-legend .orange { background: #f59e0b; }

.review-panel {
  height: 100%;
  overflow: auto;
}

.kicker {
  color: #1d5fd3;
  font-size: 11px;
  font-weight: 900;
  letter-spacing: 0.08em;
}

.review-header h2 {
  margin: 4px 0;
  font-size: 22px;
}

.review-header p {
  margin: 0;
  color: #66748a;
}

.doc-tabs {
  display: flex;
  gap: 8px;
  border-bottom: 1px solid #dfe8f5;
  margin: 16px 0 14px;
}

.doc-tabs button {
  border: 0;
  background: transparent;
  color: #66748a;
  padding: 10px 12px;
  font-weight: 900;
}

.doc-tabs .active {
  color: #1d5fd3;
  border-bottom: 3px solid #1d5fd3;
}

.doc-body {
  display: grid;
  gap: 12px;
}

.standard-section {
  border: 1px solid #e2ebf8;
  border-radius: 8px;
  padding: 14px;
  background: #ffffff;
}

.standard-section p {
  color: #253858;
  line-height: 1.8;
}

.field-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}

.field-grid div {
  background: #f7faff;
  border-radius: 8px;
  padding: 10px;
}

.field-grid span,
.field-grid strong {
  display: block;
}

.field-grid strong {
  color: #253858;
  margin-top: 6px;
}

.evidence-track {
  height: 8px;
  border-radius: 999px;
  background: #e8edf5;
  overflow: hidden;
  margin: 12px 0;
}

.evidence-fill {
  height: 100%;
  background: linear-gradient(90deg, #1d5fd3, #52b788);
}

.evidence-checks {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.evidence-checks span {
  border-radius: 999px;
  background: #fff7e8;
  color: #b26c00;
  padding: 5px 9px;
}

.evidence-checks .ok {
  background: #edf8f0;
  color: #118a45;
}

.warning-note {
  background: #fff0f0;
  color: #bb1d2a;
  border-radius: 8px;
  padding: 12px;
  font-weight: 800;
}

.raw-message {
  border-radius: 8px;
  padding: 12px;
  max-width: 78%;
}

.raw-message.ai {
  background: #f0f5ff;
}

.raw-message.user {
  background: #1d5fd3;
  color: #ffffff;
  justify-self: end;
}

.raw-message span {
  font-size: 12px;
  font-weight: 900;
  opacity: 0.72;
}

.raw-message p {
  margin: 6px 0 0;
}

.notice-item {
  display: grid;
  grid-template-columns: 1fr 90px;
  gap: 12px;
  align-items: center;
  border: 1px solid #e2ebf8;
  border-radius: 8px;
  padding: 12px;
}

.notice-item p {
  color: #66748a;
  margin: 6px 0 0;
}

.trace-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  align-content: start;
  overflow: auto;
}

.trace-grid > .panel-header {
  grid-column: 1 / -1;
}

.trace-card {
  border: 1px solid #e2ebf8;
  border-radius: 8px;
  padding: 12px;
  background: #ffffff;
}

.trace-card button {
  padding: 5px 8px;
  font-size: 12px;
}

.trace-card p {
  margin: 8px 0;
}

.confidence {
  color: #118a45;
  background: #edf8f0;
  border-radius: 6px;
  padding: 6px 8px;
  font-weight: 900;
}

.material-row {
  display: grid;
  grid-template-columns: 12px 1fr 52px;
  gap: 8px;
  align-items: center;
  padding: 7px 0;
  border-top: 1px solid #edf2fa;
}

.material-row span,
.mini-progress > span {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: #d3dae8;
}

.material-row span.ok,
.mini-progress.done > span,
.mini-progress.active > span {
  background: #1d5fd3;
}

.material-row strong {
  font-size: 12px;
}

.material-row em {
  color: #66748a;
  font-size: 12px;
  font-style: normal;
}

.mini-progress {
  display: grid;
  grid-template-columns: 14px 1fr;
  gap: 8px;
  padding: 8px 0;
  border-top: 1px solid #edf2fa;
}

.mini-progress small {
  color: #66748a;
  display: block;
  margin-top: 3px;
}

@media (max-width: 1500px) {
  .metric-strip {
    grid-template-columns: repeat(4, 1fr);
  }

  .main-grid {
    grid-template-columns: 360px minmax(480px, 1fr) 360px;
  }
}
</style>

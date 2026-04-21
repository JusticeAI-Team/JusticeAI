<template>
  <section>
    <header class="hero hero-home">
      <div class="wrap hero-grid">
        <div class="hero-meta" v-reveal="40">
          <div class="eyebrow"><span class="dot"></span>JusticeAI · 主流程入口</div>
          <div class="vol">{{ generatedAt === '-' ? '等待总览接口' : `总览更新时间 · ${generatedAt}` }}</div>
        </div>

        <h1 class="headline" v-reveal="100">
          一条流程主线.<br />
          <em>六个 Agent 协同.</em><br />
          直接落到预警与报告.
        </h1>

        <p class="hero-sub" v-reveal="160">
          先检查环境，再上传数据，然后沿着映射、抽取、图谱、风险、预警、分派、评估、监督和报告继续往下走。
        </p>

        <div class="hero-cta" v-reveal="220">
          <RouterLink class="btn primary" to="/setup">
            从系统准备开始
            <span class="arrow">→</span>
          </RouterLink>
          <RouterLink class="btn" to="/supervision">直接看 Agent 调度</RouterLink>
          <span class="hero-note">
            <span class="bullet">•</span>
            {{ loading ? '总览接口读取中' : error ? '总览接口未接通' : '主流程已接通' }}
          </span>
        </div>

        <div class="hero-stats" v-reveal="280">
          <div v-for="item in heroStats" :key="item.label" class="stat">
            <div class="n">{{ item.value }}<sup v-if="item.unit">{{ item.unit }}</sup></div>
            <div class="l">{{ item.label }}</div>
          </div>
        </div>
      </div>
    </header>

    <TickerBand :items="homeTickerItems" />

    <section class="flow">
      <div class="wrap">
        <SectionHead
          index="§ 01 / 流程轨道"
          title="先把数据 <em>送进来</em>.<br>再让 Agent 依次接手."
          lede="总览页只保留主流程轨道、关键入口、Agent 调度预览和当前异常，不再堆说明性块。"
        />

        <RouteDiagram
          v-reveal
          :start="{ label: '数据入口', name: '多源线索', tags: ['12345', '110', '395', '信访'] }"
          :current="{ label: '流程主线', name: 'JusticeAI Workflow', tags: ['归集', '映射', '抽取', '图谱', '风险', '预警'] }"
          :end="{ label: '输出收口', name: '监督与报告', tags: ['Agent 状态', '人工介入', '专题报告'] }"
          :legend="processLegend"
        />
      </div>
    </section>

    <section class="features">
      <div class="wrap">
        <SectionHead
          index="§ 02 / 当前入口"
          title="当前最该去的.<br><em>就是这四个页面.</em>"
          lede="首页只保留真正高频的入口，不把十二个页面都堆成同等权重。"
        />

        <div class="feat-grid feature-grid-spotlight">
          <RouterLink v-for="item in primaryEntries" :key="item.path" class="feat feat-link" :to="item.path" v-reveal>
            <div class="idx">{{ item.index }}</div>
            <h4 v-html="item.title"></h4>
            <p>{{ item.description }}</p>
            <div class="metric">
              <span class="mv">{{ item.metricValue }}</span>
              <span class="ml">{{ item.metricLabel }}</span>
            </div>
          </RouterLink>
        </div>
      </div>
    </section>

    <section class="providers orchestration-section">
      <div class="wrap">
        <div class="prov-head" v-reveal>
          <h3>Agent 不是口号.<br><em>谁在跑一眼看清.</em></h3>
          <p class="note">这里先看 Agent 入口和当前状态摘要；具体调度、阻塞与人工介入，进入监督协调页继续看。</p>
        </div>

        <div class="agent-summary-grid" v-reveal="80">
          <RouterLink v-for="agent in agentCards" :key="agent.key" class="agent-summary-card" :to="agent.path || '/supervision'">
            <div class="eyebrow">{{ agent.tagline }}</div>
            <h4>{{ agent.label }}</h4>
            <p>{{ agent.duty }}</p>
            <div class="status-line">
              <span class="status-chip" :class="agent.tone">{{ agent.status }}</span>
              <span class="muted-line">{{ agent.meta }}</span>
            </div>
          </RouterLink>
        </div>
      </div>
    </section>

    <section class="receipts">
      <div class="wrap receipts-grid">
        <div class="rcpt-head" v-reveal>
          <h3>当前状态.<br>先看 <em>四个数字</em>.</h3>
        </div>

        <div v-for="item in overviewReceipts" :key="item.label" class="rcpt" v-reveal>
          <div class="rn">
            {{ item.value }}
            <span v-if="item.unit" class="unit">{{ item.unit }}</span>
          </div>
          <div class="rl">{{ item.label }}</div>
          <div class="rd">{{ item.description }}</div>
        </div>
      </div>
    </section>

    <section class="sheet-section surface-section">
      <div class="wrap section-stack">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>待继续处理</h4>
              <p>从这里直接进入当前最可能要操作的页面。</p>
            </div>
          </div>
          <div class="sheet-body sheet-list">
            <div v-for="item in actionQueue" :key="item.title" class="sheet-row">
              <div>
                <strong>{{ item.title }}</strong>
                <p>{{ item.subtitle }}</p>
                <span>{{ item.meta }}</span>
              </div>
              <RouterLink class="btn ghost" :to="item.path">进入</RouterLink>
            </div>
          </div>
        </div>

        <div class="sheet-shell" v-reveal="80">
          <div class="sheet-head">
            <div>
              <h4>全部流程页</h4>
              <p>非高频页面仍保留在这里，继续按主流程向下走。</p>
            </div>
          </div>
          <div class="stage-rail-grid sheet-body">
            <RouterLink v-for="step in workflowSteps" :key="step.key" class="stage-rail-card" :to="step.path">
              <div class="stage-rail-code">{{ step.stageCode }}</div>
              <strong>{{ step.title }}</strong>
              <span>{{ step.shortLabel }}</span>
            </RouterLink>
          </div>
        </div>
      </div>
    </section>

    <section class="cta">
      <div class="cta-inner" v-reveal>
        <div class="eyebrow" style="margin-bottom: 32px"><span class="dot"></span>先跑主线，再补足细节</div>
        <h2>先把流程跑通.<br /><em>再把页面打磨到底.</em></h2>
        <p>重点先看系统准备、数据归集、风险研判、监督协调和报告输出。</p>
        <div class="inline-actions" style="justify-content: center">
          <RouterLink class="btn primary" to="/data-ingestion">
            进入数据归集
            <span class="arrow">→</span>
          </RouterLink>
          <RouterLink class="btn" to="/reports">查看最终报告</RouterLink>
        </div>
        <div class="cta-marquee" aria-hidden="true">JUSTICEAI · INGEST · RISK · SUPERVISION · REPORTS</div>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import SectionHead from '../components/reference/SectionHead.vue'
import RouteDiagram from '../components/reference/RouteDiagram.vue'
import TickerBand from '../components/reference/TickerBand.vue'
import { fetchDashboardOverview } from '../api/dashboard'
import type { DashboardOverviewResponse } from '../types/workspace'
import { formatDateTime, resolveTone } from '../utils/format'
import { homeTickerItems, workflowAgents, workflowSteps } from '../workflow/catalog'

const loading = ref(false)
const error = ref('')
const overview = ref<DashboardOverviewResponse | null>(null)

const generatedAt = computed(() => formatDateTime(overview.value?.generated_at))

const heroStats = computed(() => {
  if (overview.value?.metrics.length) {
    return overview.value.metrics.slice(0, 4).map((metric) => ({
      label: metric.label,
      value: metric.value,
      unit: metric.unit === '%' || metric.unit === 'ms' ? metric.unit : '',
    }))
  }

  return [
    { label: '流程页', value: String(workflowSteps.length), unit: '' },
    { label: 'Agent 数', value: String(workflowAgents.length), unit: '' },
    { label: '高频页', value: '5', unit: '' },
    { label: '输出端', value: '2', unit: '页' },
  ]
})

const processLegend = [
  { key: '01 / 系统准备', value: '先检查后端、模型、图谱和平台设置。' },
  { key: '02 / 数据归集', value: '上传 Excel，生成批次并确认已有导入记录。' },
  { key: '03 / 中段处理', value: '映射、抽取、图谱和风险分析依次推进。' },
  { key: '04 / 收口输出', value: '预警、分派、评估、监督与报告形成闭环。' },
]

const primaryEntries = [
  {
    index: '01 / 归集',
    path: '/data-ingestion',
    title: '把月度数据<br><em>先送进系统</em>',
    description: '上传文件、看批次、核对兼容记录与导入详情。',
    metricValue: 'S02',
    metricLabel: '主入口',
  },
  {
    index: '02 / 风险',
    path: '/risk-analysis',
    title: '把图谱结果<br><em>转成风险判断</em>',
    description: '看高风险对象、风险等级和下发预警的优先顺序。',
    metricValue: 'S06',
    metricLabel: '业务核心',
  },
  {
    index: '03 / 监督',
    path: '/supervision',
    title: '把 Agent 运行<br><em>直接放到台前</em>',
    description: '看谁在跑、谁阻塞、哪里需要人工介入。',
    metricValue: 'S10',
    metricLabel: '调度主场',
  },
  {
    index: '04 / 报告',
    path: '/reports',
    title: '把阶段结果<br><em>收口成报告</em>',
    description: '看专题报告、周期报告和最终输出时间。',
    metricValue: 'S11',
    metricLabel: '成果页',
  },
]

const workflowStatusByKey = computed(() => {
  const entries = overview.value?.workflow ?? []
  return Object.fromEntries(entries.map((item) => [item.key, item]))
})

const agentCards = computed(() => {
  return workflowAgents.map((agent) => {
    const workflowStatus = workflowStatusByKey.value[agent.key]
    const tone = workflowStatus ? resolveTone(workflowStatus.status) : 'neutral'

    return {
      ...agent,
      status: workflowStatus ? `${workflowStatus.label} ${workflowStatus.completed}/${workflowStatus.total}` : '等待状态',
      meta: workflowStatus ? `当前状态 ${workflowStatus.status}` : '进入对应页面查看详情',
      tone,
    }
  })
})

const overviewReceipts = computed(() => {
  if (overview.value?.metrics.length) {
    return overview.value.metrics.slice(0, 4).map((metric) => ({
      label: metric.label,
      value: metric.value,
      unit: metric.unit || '',
      description: [metric.trend, metric.trend_value].filter(Boolean).join(' · ') || '当前总览快照',
    }))
  }

  return [
    { label: '阶段总数', value: String(workflowSteps.length), unit: '', description: '从系统准备到平台设置，共 12 个主路由。' },
    { label: 'Agent 总数', value: String(workflowAgents.length), unit: '', description: '六个 Agent 分别承担归集、研判、推送、执行、评估与监督。' },
    { label: '高频操作页', value: '5', unit: '页', description: '首页优先把五个关键页面放到主入口。' },
    { label: '最终输出', value: '2', unit: '端', description: '监督协调负责运行态，报告输出负责成果收口。' },
  ]
})

const actionQueue = computed(() => {
  const queue = overview.value?.queues ?? []

  if (queue.length > 0) {
    return queue.slice(0, 4).map((item, index) => ({
      title: item.label,
      subtitle: `当前数量 ${item.count}`,
      meta: `状态 ${item.status}`,
      path: index === 0 ? '/data-ingestion' : index === 1 ? '/risk-analysis' : index === 2 ? '/supervision' : '/reports',
    }))
  }

  return [
    { title: '先完成系统准备', subtitle: '确认后端、模型和图谱已联通', meta: '下一步进入数据归集', path: '/setup' },
    { title: '上传第一批数据', subtitle: '生成归集批次与兼容导入记录', meta: '进入映射前先确认已有数据', path: '/data-ingestion' },
    { title: '查看 Agent 运行', subtitle: '从监督协调页看阻塞与人工介入', meta: '这是 Agentic workflow 主入口', path: '/supervision' },
    { title: '检查最终报告', subtitle: '查看周期报告与专题报告', meta: '输出链路在这里收口', path: '/reports' },
  ]
})

async function loadOverview() {
  loading.value = true
  error.value = ''

  try {
    overview.value = await fetchDashboardOverview()
  } catch (reason) {
    overview.value = null
    error.value = reason instanceof Error ? reason.message : '总览接口读取失败'
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  void loadOverview()
})
</script>

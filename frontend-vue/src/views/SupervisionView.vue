<template>
  <section>
    <header class="hero hero-stage hero-supervision">
      <div class="wrap hero-grid">
        <div class="hero-meta" v-reveal="40">
          <div class="eyebrow"><span class="dot"></span>JusticeAI · S10 · 监督协调</div>
          <div class="vol">{{ generatedAt === '-' ? '等待监督接口' : `监督更新时间 · ${generatedAt}` }}</div>
        </div>

        <h1 class="headline" v-reveal="100">
          哪些 Agent 正在运行.<br />
          <em>哪里需要人工介入.</em><br />
          在这里统一看清.
        </h1>

        <p class="hero-sub" v-reveal="160">
          监督协调页就是 Agent orchestration 主场：看谁在跑、谁阻塞、谁等待、哪里要人工接手。
        </p>

        <div class="hero-cta" v-reveal="220">
          <RouterLink class="btn" to="/evaluation">上一步 成效评估</RouterLink>
          <RouterLink class="btn primary" to="/reports">
            下一步 报告输出
            <span class="arrow">→</span>
          </RouterLink>
          <span class="hero-note">
            <span class="bullet">•</span>
            {{ loading ? '监督接口读取中' : error ? '监督接口未接通' : gateLabel }}
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

    <TickerBand :items="step.ticker" />

    <section class="flow">
      <div class="wrap">
        <SectionHead
          index="§ S10 / 调度总线"
          title="多 Agent 不藏在文案里.<br><em>直接放到主屏上.</em>"
          lede="本页重点不是普通列表，而是当前运行态、任务量、人工介入和流程位置。"
        />

        <RouteDiagram
          v-reveal
          :start="{ label: '前段 Agent', name: '归集到评估', tags: ['线索挖掘', '风险研判', '分级推送'] }"
          :current="{ label: '调度中心', name: '监督协调', tags: ['运行态', '阻塞', '人工介入', '日志'] }"
          :end="{ label: '输出页', name: '报告输出', tags: ['收口', '复核', '汇总'] }"
          :legend="legend"
        />
      </div>
    </section>

    <section class="receipts">
      <div class="wrap receipts-grid">
        <div class="rcpt-head" v-reveal>
          <h3>当前运行态.<br>先看 <em>核心计数</em>.</h3>
        </div>

        <div v-for="item in receiptMetrics" :key="item.label" class="rcpt" v-reveal>
          <div class="rn">
            {{ item.value }}
            <span v-if="item.unit" class="unit">{{ item.unit }}</span>
          </div>
          <div class="rl">{{ item.label }}</div>
          <div class="rd">{{ item.description }}</div>
        </div>
      </div>
    </section>

    <section class="sheet-section stage-spotlight-section">
      <div class="wrap section-stack">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>监督面板状态</h4>
              <p>这里先判断监督协调页是否已经具备继续进入报告输出的条件。</p>
            </div>
            <span class="status-chip" :class="gateTone">{{ gateLabel }}</span>
          </div>
          <div class="sheet-body">
            <StateFrame
              v-if="loading && !response"
              kind="loading"
              title="正在读取监督协调面板"
              description="正在获取 Agent 运行状态与监督指标。"
            />
            <StateFrame
              v-else-if="error && !response"
              kind="disconnected"
              title="监督接口暂未接通"
              :description="error"
              action-label="回到成效评估"
              action-to="/evaluation"
            />
            <div v-else class="sheet-grid">
              <div class="sheet-cell">
                <div class="k">进入下一步</div>
                <div class="v">{{ gateLabel }}</div>
                <div class="d">{{ gateDescription }}</div>
              </div>
              <div class="sheet-cell">
                <div class="k">运行 Agent</div>
                <div class="v">{{ runningAgents }}</div>
                <div class="d">当前处于运行中的 Agent 数量。</div>
              </div>
              <div class="sheet-cell">
                <div class="k">需关注 Agent</div>
                <div class="v">{{ attentionAgents }}</div>
                <div class="d">包含异常、阻塞、降级等需要人工关注的对象。</div>
              </div>
            </div>
          </div>
        </div>

        <div class="sheet-shell" v-reveal="80">
          <div class="sheet-head">
            <div>
              <h4>Agent 调度泳道</h4>
              <p>每个 Agent 的状态、任务量和当前所处流程都在这里看。</p>
            </div>
          </div>
          <div class="sheet-body">
            <div v-if="agents.length > 0" class="swimlane-list">
              <article v-for="agent in swimlanes" :key="agent.key" class="swimlane-card">
                <div class="swimlane-main">
                  <div>
                    <div class="eyebrow">{{ agent.stage }}</div>
                    <h4>{{ agent.label }}</h4>
                    <p>{{ agent.description }}</p>
                  </div>
                  <span class="status-chip" :class="agent.tone">{{ agent.statusLabel }}</span>
                </div>
                <div class="swimlane-bar">
                  <div class="swimlane-fill" :style="{ width: agent.width }"></div>
                </div>
                <div class="swimlane-meta">
                  <span>运行任务 {{ agent.running_tasks }}</span>
                  <span>最近更新 {{ agent.updatedAt }}</span>
                </div>
              </article>
            </div>
            <div v-else class="sheet-empty">当前还没有 Agent 状态返回。</div>
          </div>
        </div>
      </div>
    </section>

    <section class="sheet-section">
      <div class="wrap supervision-grid-layout">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>人工介入队列</h4>
              <p>需要人工判断的对象集中摆在这里。</p>
            </div>
          </div>
          <div class="sheet-body supervision-side-list">
            <div v-if="interventions.length > 0" class="sheet-list">
              <div v-for="item in interventions" :key="item.title" class="sheet-row">
                <div>
                  <strong>{{ item.title }}</strong>
                  <p>{{ item.subtitle }}</p>
                  <span>{{ item.meta }}</span>
                </div>
                <span class="status-chip warning">待人工确认</span>
              </div>
            </div>
            <div v-else class="sheet-empty">当前没有需要人工介入的事项。</div>
          </div>
        </div>

        <div class="sheet-shell" v-reveal="80">
          <div class="sheet-head">
            <div>
              <h4>运行日志摘要</h4>
              <p>快速看最近的运行事件和状态变化。</p>
            </div>
          </div>
          <div class="sheet-body supervision-side-list">
            <div class="sheet-list">
              <div v-for="item in timeline" :key="item.title" class="sheet-row">
                <div>
                  <strong>{{ item.title }}</strong>
                  <p>{{ item.subtitle }}</p>
                  <span>{{ item.meta }}</span>
                </div>
                <span class="status-chip" :class="item.tone">{{ item.status }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import RouteDiagram from '../components/reference/RouteDiagram.vue'
import SectionHead from '../components/reference/SectionHead.vue'
import StateFrame from '../components/reference/StateFrame.vue'
import TickerBand from '../components/reference/TickerBand.vue'
import { fetchSupervisionOverview } from '../api/supervision'
import type { AgentStatusItem, SupervisionOverviewResponse } from '../types/workspace'
import { formatDateTime, formatStatusLabel, resolveTone } from '../utils/format'
import { getAgent, getWorkflowStep } from '../workflow/catalog'

const step = getWorkflowStep('supervision')

const loading = ref(false)
const error = ref('')
const response = ref<SupervisionOverviewResponse | null>(null)

const generatedAt = computed(() => formatDateTime(response.value?.generated_at))
const agents = computed(() => response.value?.agents ?? [])
const runningAgents = computed(() => agents.value.filter((item) => item.running_tasks > 0).length)
const attentionAgents = computed(() => agents.value.filter((item) => resolveTone(item.status) !== 'good').length)

const gateLabel = computed(() => (agents.value.length > 0 ? '条件已满足' : error.value ? '监督接口未接通' : '等待 Agent 状态'))
const gateDescription = computed(() =>
  agents.value.length > 0 ? '已可查看 Agent 运行状态和人工介入，可继续进入报告输出。' : '至少返回一类 Agent 状态后再进入报告输出。',
)
const gateTone = computed(() => (agents.value.length > 0 ? 'good' : error.value ? 'danger' : 'warning'))

const heroStats = computed(() => {
  if (response.value?.metrics.length) {
    return response.value.metrics.slice(0, 4).map((metric) => ({
      label: metric.label,
      value: metric.value,
      unit: metric.unit === '%' || metric.unit === 'ms' ? metric.unit : '',
    }))
  }

  return [
    { label: 'Agent 数', value: String(agents.value.length), unit: '' },
    { label: '运行中', value: String(runningAgents.value), unit: '' },
    { label: '需关注', value: String(attentionAgents.value), unit: '' },
    { label: '下一页', value: 'S11', unit: '' },
  ]
})

const receiptMetrics = computed(() => {
  if (response.value?.metrics.length) {
    return response.value.metrics.slice(0, 4).map((metric) => ({
      label: metric.label,
      value: metric.value,
      unit: metric.unit || '',
      description: [metric.trend, metric.trend_value].filter(Boolean).join(' · ') || '当前监督快照',
    }))
  }

  return [
    { label: '全部 Agent', value: String(agents.value.length), unit: '', description: '当前已返回的 Agent 总数。' },
    { label: '运行任务', value: String(agents.value.reduce((sum, item) => sum + item.running_tasks, 0)), unit: '', description: '全部 Agent 当前运行任务总数。' },
    { label: '需人工关注', value: String(attentionAgents.value), unit: '', description: '需要人工关注或介入的 Agent 数量。' },
  ]
})

const swimlanes = computed(() => {
  const maxTasks = Math.max(...agents.value.map((item) => item.running_tasks), 1)

  return agents.value.map((agent) => {
    const meta = safeAgentMeta(agent)
    const tone = resolveTone(agent.status)
    return {
      ...agent,
      stage: meta.stage,
      description: meta.description,
      statusLabel: formatStatusLabel(agent.status),
      updatedAt: formatDateTime(agent.updated_at),
      tone,
      width: `${Math.max(12, Math.round((agent.running_tasks / maxTasks) * 100))}%`,
    }
  })
})

const interventions = computed(() => {
  return swimlanes.value
    .filter((item) => item.tone !== 'good')
    .slice(0, 4)
    .map((item) => ({
      title: item.label,
      subtitle: `${item.running_tasks} 个运行任务`,
      meta: `当前状态 ${item.statusLabel} · 最近更新 ${item.updatedAt}`,
    }))
})

const timeline = computed(() => {
  if (swimlanes.value.length > 0) {
    return swimlanes.value.slice(0, 6).map((item) => ({
      title: item.label,
      subtitle: `${item.stage} · ${item.running_tasks} 个运行任务`,
      meta: `最近更新 ${item.updatedAt}`,
      status: item.statusLabel,
      tone: item.tone,
    }))
  }

  return [
    { title: '等待监督接口返回', subtitle: '当前还没有日志摘要', meta: '请先联通 /supervision/overview', status: '等待中', tone: 'warning' },
  ]
})

const legend = [
  { key: '01 / 看谁在跑', value: '先确认当前有哪些 Agent 正在运行。' },
  { key: '02 / 看压力', value: '通过任务量和状态判断当前处理压力。' },
  { key: '03 / 看介入点', value: '发现异常、阻塞和降级时，从这里触发人工介入。' },
  { key: '04 / 继续输出', value: '监督页确认完成后，继续进入报告输出。' },
]

function safeAgentMeta(agent: AgentStatusItem) {
  try {
    const definition = getAgent(agent.key as Parameters<typeof getAgent>[0])
    return {
      stage: definition.tagline,
      description: definition.duty,
    }
  } catch {
    return {
      stage: '流程节点',
      description: '当前 Agent 已接入监督面板。',
    }
  }
}

async function loadSupervision() {
  loading.value = true
  error.value = ''

  try {
    response.value = await fetchSupervisionOverview()
  } catch (reason) {
    response.value = null
    error.value = reason instanceof Error ? reason.message : '监督接口读取失败'
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  void loadSupervision()
})
</script>

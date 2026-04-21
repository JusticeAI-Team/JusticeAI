<template>
  <section>
    <header class="hero hero-stage hero-risk">
      <div class="wrap hero-grid">
        <div class="hero-meta" v-reveal="40">
          <div class="eyebrow"><span class="dot"></span>JusticeAI · S06 · 风险研判</div>
          <div class="vol">{{ generatedAt === '-' ? '等待风险接口' : `风险更新时间 · ${generatedAt}` }}</div>
        </div>

        <h1 class="headline" v-reveal="100">
          先看高风险对象.<br />
          <em>再决定是否下发预警.</em><br />
          不让研判停在列表上.
        </h1>

        <p class="hero-sub" v-reveal="160">
          本页只看三件事：高风险对象、等级分层、进入预警前的优先顺序。
        </p>

        <div class="hero-cta" v-reveal="220">
          <RouterLink class="btn" to="/knowledge-graph">上一步 知识图谱</RouterLink>
          <RouterLink class="btn primary" to="/alerts">
            下一步 预警中心
            <span class="arrow">→</span>
          </RouterLink>
          <span class="hero-note">
            <span class="bullet">•</span>
            {{ loading ? '风险接口读取中' : error ? '风险接口未接通' : gateLabel }}
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
          index="§ S06 / 研判轨道"
          title="图谱先给出关系.<br>这里把它变成 <em>风险判断</em>."
          lede="高风险对象、风险等级和归因入口放在同一页，不再只给一张通用列表。"
        />

        <RouteDiagram
          v-reveal
          :start="{ label: '上一步', name: '知识图谱', tags: ['关系类型', '关系规模'] }"
          :current="{ label: '当前页', name: '风险研判', tags: ['对象', '等级', '评分', '归因'] }"
          :end="{ label: '下一步', name: '预警中心', tags: ['转成预警', '继续派发'] }"
          :legend="legend"
        />
      </div>
    </section>

    <section class="receipts">
      <div class="wrap receipts-grid">
        <div class="rcpt-head" v-reveal>
          <h3>先看快照.<br>再做 <em>预警判断</em>.</h3>
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
              <h4>进入预警前</h4>
              <p>这里只判断当前是否已经具备进入预警中心的条件。</p>
            </div>
            <span class="status-chip" :class="gateTone">{{ gateLabel }}</span>
          </div>
          <div class="sheet-body">
            <StateFrame
              v-if="loading && !response"
              kind="loading"
              title="正在读取风险研判结果"
              description="正在获取高风险对象和风险指标。"
            />
            <StateFrame
              v-else-if="error && !response"
              kind="disconnected"
              title="风险接口暂未接通"
              :description="error"
              action-label="回到知识图谱"
              action-to="/knowledge-graph"
            />
            <div v-else class="sheet-grid">
              <div class="sheet-cell">
                <div class="k">进入下一步</div>
                <div class="v">{{ gateLabel }}</div>
                <div class="d">{{ gateDescription }}</div>
              </div>
              <div class="sheet-cell">
                <div class="k">当前高风险数</div>
                <div class="v">{{ risks.length }}</div>
                <div class="d">高风险对象越多，越需要及时进入预警中心。</div>
              </div>
              <div class="sheet-cell">
                <div class="k">最近更新时间</div>
                <div class="v">{{ generatedAt }}</div>
                <div class="d">如果没有时间，说明当前接口还未返回快照。</div>
              </div>
            </div>
          </div>
        </div>

        <div class="risk-stage-layout">
          <div class="sheet-shell" v-reveal="80">
            <div class="sheet-head">
              <div>
                <h4>重点风险列表</h4>
                <p>先看最需要进入预警的对象。</p>
              </div>
            </div>
            <div class="sheet-body">
              <StateFrame
                v-if="!loading && !error && risks.length === 0"
                kind="empty"
                title="当前暂无高风险对象"
                description="风险分析接口已接通，但当前没有需要优先处理的对象。"
              />
              <div v-else class="risk-card-grid">
                <article v-for="risk in topRisks" :key="risk.id" class="risk-card">
                  <div class="risk-card-head">
                    <div>
                      <div class="eyebrow">{{ risk.area }}</div>
                      <h4>{{ risk.title }}</h4>
                    </div>
                    <span class="status-chip" :class="risk.tone">{{ risk.level }}</span>
                  </div>
                  <div class="risk-score">{{ risk.score }}</div>
                  <div class="risk-meta">当前状态 {{ risk.statusLabel }}</div>
                </article>
              </div>
            </div>
          </div>

          <div class="sheet-shell" v-reveal="120">
            <div class="sheet-head">
              <div>
                <h4>风险等级分层</h4>
                <p>快速确认当前高、中、低三个层级的分布。</p>
              </div>
            </div>
            <div class="sheet-body risk-band-grid">
              <div v-for="band in riskBands" :key="band.label" class="risk-band-card">
                <div class="k">{{ band.label }}</div>
                <div class="v">{{ band.count }}</div>
                <div class="d">{{ band.description }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="sheet-section">
      <div class="wrap">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>全部风险对象</h4>
              <p>需要完整核对时再看这里。</p>
            </div>
          </div>
          <div class="sheet-body">
            <div v-if="risks.length > 0" class="table-shell">
              <table class="sheet-table">
                <thead>
                  <tr>
                    <th>对象</th>
                    <th>领域</th>
                    <th>等级</th>
                    <th>评分</th>
                    <th>状态</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="risk in risks" :key="risk.id">
                    <td>{{ risk.title }}</td>
                    <td>{{ risk.area }}</td>
                    <td>{{ risk.level }}</td>
                    <td>{{ risk.score }}</td>
                    <td>{{ risk.statusLabel }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div v-else class="sheet-empty">当前没有可展示的风险对象。</div>
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
import { fetchRiskOverview } from '../api/risk'
import type { RiskItem, RiskOverviewResponse } from '../types/workspace'
import { formatDateTime, formatStatusLabel, resolveTone } from '../utils/format'
import { getWorkflowStep } from '../workflow/catalog'

const step = getWorkflowStep('risk-analysis')

const loading = ref(false)
const error = ref('')
const response = ref<RiskOverviewResponse | null>(null)

const generatedAt = computed(() => formatDateTime(response.value?.generated_at))
const risks = computed(() => response.value?.top_risks ?? [])

const gateLabel = computed(() => (risks.value.length > 0 ? '条件已满足' : error.value ? '风险接口未接通' : '等待风险结果'))
const gateDescription = computed(() =>
  risks.value.length > 0 ? '已有高风险对象，可以继续进入预警中心。' : '至少完成一轮风险评分后再进入预警中心。',
)
const gateTone = computed(() => (risks.value.length > 0 ? 'good' : error.value ? 'danger' : 'warning'))

const heroStats = computed(() => {
  if (response.value?.metrics.length) {
    return response.value.metrics.slice(0, 4).map((metric) => ({
      label: metric.label,
      value: metric.value,
      unit: metric.unit === '%' || metric.unit === 'ms' ? metric.unit : '',
    }))
  }

  return [
    { label: '高风险对象', value: String(risks.value.length), unit: '' },
    { label: '是否可预警', value: risks.value.length > 0 ? '可继续' : '待生成', unit: '' },
    { label: '下一页', value: 'S07', unit: '' },
    { label: '当前阶段', value: 'S06', unit: '' },
  ]
})

const receiptMetrics = computed(() => {
  if (response.value?.metrics.length) {
    return response.value.metrics.slice(0, 4).map((metric) => ({
      label: metric.label,
      value: metric.value,
      unit: metric.unit || '',
      description: [metric.trend, metric.trend_value].filter(Boolean).join(' · ') || '当前风险快照',
    }))
  }

  return [
    { label: '高风险对象', value: String(risks.value.length), unit: '', description: '当前返回的高风险对象数量。' },
    { label: '最高评分', value: risks.value.length > 0 ? String(Math.max(...risks.value.map((item) => item.score))) : '0', unit: '', description: '当前列表中的最高风险评分。' },
  ]
})

const topRisks = computed(() => {
  return risks.value.slice(0, 6).map((risk) => ({
    ...risk,
    statusLabel: formatStatusLabel(risk.status),
    tone: resolveTone(risk.status),
  }))
})

const riskBands = computed(() => {
  const high = risks.value.filter((item) => item.level.includes('高')).length
  const medium = risks.value.filter((item) => item.level.includes('中')).length
  const low = risks.value.length - high - medium

  return [
    { label: '高风险', count: high, description: '优先进入预警中心。' },
    { label: '中风险', count: medium, description: '继续观察并等待更多证据。' },
    { label: '低风险', count: low, description: '暂不优先下发。' },
  ]
})

const legend = [
  { key: '01 / 读图谱', value: '先确认图谱关系已可供风险评分使用。' },
  { key: '02 / 看对象', value: '重点看高风险对象和对应领域。' },
  { key: '03 / 判断优先级', value: '按等级和评分排序，决定是否进入预警。' },
  { key: '04 / 继续下发', value: '把需要处置的对象转入预警中心。' },
]

async function loadRisk() {
  loading.value = true
  error.value = ''

  try {
    response.value = await fetchRiskOverview()
  } catch (reason) {
    response.value = null
    error.value = reason instanceof Error ? reason.message : '风险接口读取失败'
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  void loadRisk()
})
</script>

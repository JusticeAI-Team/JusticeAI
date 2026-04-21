<template>
  <section>
    <header class="hero hero-stage hero-reports">
      <div class="wrap hero-grid">
        <div class="hero-meta" v-reveal="40">
          <div class="eyebrow"><span class="dot"></span>JusticeAI · S11 · 报告输出</div>
          <div class="vol">{{ generatedAt === '-' ? '等待报告接口' : `报告更新时间 · ${generatedAt}` }}</div>
        </div>

        <h1 class="headline" v-reveal="100">
          阶段结果在这里收口.<br />
          <em>专题报告与周期报告并排看.</em><br />
          不再只是普通列表.
        </h1>

        <p class="hero-sub" v-reveal="160">
          报告页就是成果页：先看本周期输出，再看报告家族分布和最近生成结果。
        </p>

        <div class="hero-cta" v-reveal="220">
          <RouterLink class="btn" to="/supervision">上一步 监督协调</RouterLink>
          <RouterLink class="btn primary" to="/settings">
            下一步 平台设置
            <span class="arrow">→</span>
          </RouterLink>
          <span class="hero-note">
            <span class="bullet">•</span>
            {{ loading ? '报告接口读取中' : error ? '报告接口未接通' : gateLabel }}
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
          index="§ S11 / 成果出口"
          title="监督页确认运行态.<br>这里把结果变成 <em>可交付产物</em>."
          lede="报告输出页按报告家族和最近产出组织，不再只把全部报告扁平列出来。"
        />

        <RouteDiagram
          v-reveal
          :start="{ label: '上一步', name: '监督协调', tags: ['运行态', '人工介入', '异常日志'] }"
          :current="{ label: '当前页', name: '报告输出', tags: ['周期报告', '专题报告', '生成状态'] }"
          :end="{ label: '下一步', name: '平台设置', tags: ['配置收口', '长期运维'] }"
          :legend="legend"
        />
      </div>
    </section>

    <section class="receipts">
      <div class="wrap receipts-grid">
        <div class="rcpt-head" v-reveal>
          <h3>当前产出.<br>先看 <em>报告快照</em>.</h3>
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
              <h4>报告输出状态</h4>
              <p>这里只判断当前是否已经有成果可供查看或继续收口。</p>
            </div>
            <span class="status-chip" :class="gateTone">{{ gateLabel }}</span>
          </div>
          <div class="sheet-body">
            <StateFrame
              v-if="loading && !response"
              kind="loading"
              title="正在读取报告输出"
              description="正在获取报告列表和当前输出状态。"
            />
            <StateFrame
              v-else-if="error && !response"
              kind="disconnected"
              title="报告接口暂未接通"
              :description="error"
              action-label="回到监督协调"
              action-to="/supervision"
            />
            <div v-else class="sheet-grid">
              <div class="sheet-cell">
                <div class="k">当前状态</div>
                <div class="v">{{ gateLabel }}</div>
                <div class="d">{{ gateDescription }}</div>
              </div>
              <div class="sheet-cell">
                <div class="k">专题报告</div>
                <div class="v">{{ specialReports.length }}</div>
                <div class="d">按专题生成的报告数量。</div>
              </div>
              <div class="sheet-cell">
                <div class="k">周期报告</div>
                <div class="v">{{ periodicReports.length }}</div>
                <div class="d">按周期持续生成的报告数量。</div>
              </div>
            </div>
          </div>
        </div>

        <div class="reports-spotlight-layout">
          <div class="sheet-shell" v-reveal="80">
            <div class="sheet-head">
              <div>
                <h4>本周期主报告</h4>
                <p>先看当前最值得打开的报告。</p>
              </div>
            </div>
            <div class="sheet-body">
              <div v-if="featuredReport" class="report-hero-card">
                <div class="eyebrow">{{ featuredReport.report_type }}</div>
                <h4>{{ featuredReport.title }}</h4>
                <p>周期 {{ featuredReport.period }}</p>
                <div class="report-hero-meta">
                  <span class="status-chip" :class="featuredReport.tone">{{ featuredReport.statusLabel }}</span>
                  <span class="muted-line">生成时间 {{ featuredReport.generatedAt }}</span>
                </div>
              </div>
              <div v-else class="sheet-empty">当前还没有可作为主报告展示的产物。</div>
            </div>
          </div>

          <div class="sheet-shell" v-reveal="120">
            <div class="sheet-head">
              <div>
                <h4>报告家族</h4>
                <p>先按报告家族区分，再决定从哪里继续看。</p>
              </div>
            </div>
            <div class="sheet-body report-family-grid">
              <div class="report-family-card">
                <div class="k">专题报告</div>
                <div class="v">{{ specialReports.length }}</div>
                <div class="d">更适合专题汇报和专项研判。</div>
              </div>
              <div class="report-family-card">
                <div class="k">周期报告</div>
                <div class="v">{{ periodicReports.length }}</div>
                <div class="d">更适合月度、季度或持续追踪。</div>
              </div>
              <div class="report-family-card">
                <div class="k">待生成</div>
                <div class="v">{{ pendingReports.length }}</div>
                <div class="d">当前已存在但状态仍未完成的报告。</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="sheet-section">
      <div class="wrap reports-grid-layout">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>专题报告</h4>
              <p>优先看专项、专题和重点事项报告。</p>
            </div>
          </div>
          <div class="sheet-body supervision-side-list">
            <div v-if="specialReports.length > 0" class="sheet-list">
              <div v-for="item in specialReports" :key="item.id" class="sheet-row">
                <div>
                  <strong>{{ item.title }}</strong>
                  <p>{{ item.report_type }}</p>
                  <span>周期 {{ item.period }} · {{ item.generatedAt }}</span>
                </div>
                <span class="status-chip" :class="item.tone">{{ item.statusLabel }}</span>
              </div>
            </div>
            <div v-else class="sheet-empty">当前没有专题报告。</div>
          </div>
        </div>

        <div class="sheet-shell" v-reveal="80">
          <div class="sheet-head">
            <div>
              <h4>周期报告</h4>
              <p>看月报、季报等持续性输出。</p>
            </div>
          </div>
          <div class="sheet-body supervision-side-list">
            <div v-if="periodicReports.length > 0" class="sheet-list">
              <div v-for="item in periodicReports" :key="item.id" class="sheet-row">
                <div>
                  <strong>{{ item.title }}</strong>
                  <p>{{ item.report_type }}</p>
                  <span>周期 {{ item.period }} · {{ item.generatedAt }}</span>
                </div>
                <span class="status-chip" :class="item.tone">{{ item.statusLabel }}</span>
              </div>
            </div>
            <div v-else class="sheet-empty">当前没有周期报告。</div>
          </div>
        </div>
      </div>
    </section>

    <section class="sheet-section">
      <div class="wrap">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>全部报告</h4>
              <p>需要完整核对时再看这一张总表。</p>
            </div>
          </div>
          <div class="sheet-body">
            <div v-if="reports.length > 0" class="table-shell">
              <table class="sheet-table">
                <thead>
                  <tr>
                    <th>报告标题</th>
                    <th>报告类型</th>
                    <th>周期</th>
                    <th>状态</th>
                    <th>生成时间</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="report in reports" :key="report.id">
                    <td>{{ report.title }}</td>
                    <td>{{ report.report_type }}</td>
                    <td>{{ report.period }}</td>
                    <td>{{ report.statusLabel }}</td>
                    <td>{{ report.generatedAt }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div v-else class="sheet-empty">当前没有可展示的报告。</div>
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
import { fetchReports } from '../api/reports'
import type { ReportListResponse } from '../types/workspace'
import { formatDateTime, formatStatusLabel, resolveTone } from '../utils/format'
import { getWorkflowStep } from '../workflow/catalog'

const step = getWorkflowStep('reports')

const loading = ref(false)
const error = ref('')
const response = ref<ReportListResponse | null>(null)

const generatedAt = computed(() => formatDateTime(response.value?.generated_at))
const reports = computed(() => {
  return (response.value?.items ?? []).map((item) => ({
    ...item,
    generatedAt: formatDateTime(item.generated_at),
    statusLabel: formatStatusLabel(item.status),
    tone: resolveTone(item.status),
  }))
})

const featuredReport = computed(() => reports.value[0] ?? null)
const pendingReports = computed(() => reports.value.filter((item) => item.tone !== 'good'))
const specialReports = computed(() => reports.value.filter((item) => !/月|季|周|年/.test(item.period)).slice(0, 6))
const periodicReports = computed(() => reports.value.filter((item) => /月|季|周|年/.test(item.period)).slice(0, 6))

const gateLabel = computed(() => (reports.value.length > 0 ? '条件已满足' : error.value ? '报告接口未接通' : '等待报告产物'))
const gateDescription = computed(() =>
  reports.value.length > 0 ? '已有报告产物，可继续进入平台设置或后续精修。' : '至少生成或返回一类报告后，这一页才会有实际成果。',
)
const gateTone = computed(() => (reports.value.length > 0 ? 'good' : error.value ? 'danger' : 'warning'))

const heroStats = computed(() => [
  { label: '报告总数', value: String(reports.value.length), unit: '' },
  { label: '专题报告', value: String(specialReports.value.length), unit: '' },
  { label: '周期报告', value: String(periodicReports.value.length), unit: '' },
  { label: '待生成', value: String(pendingReports.value.length), unit: '' },
])

const receiptMetrics = computed(() => [
  { label: '全部报告', value: String(reports.value.length), unit: '', description: '当前已返回的报告总数。' },
  { label: '主报告', value: featuredReport.value ? '1' : '0', unit: '', description: '本页会优先展示一份主报告。' },
  { label: '专题占比', value: reports.value.length > 0 ? String(specialReports.value.length) : '0', unit: '', description: '非周期类专题报告数量。' },
  { label: '待完成', value: String(pendingReports.value.length), unit: '', description: '仍未完成或需关注的报告数量。' },
])

const legend = [
  { key: '01 / 看主报告', value: '先看本周期最值得打开的一份报告。' },
  { key: '02 / 分家族', value: '把专题报告和周期报告分开看。' },
  { key: '03 / 看状态', value: '快速识别已完成、待处理和需关注的报告。' },
  { key: '04 / 继续收口', value: '报告确认后，进入平台设置作为流程尾部收口。' },
]

async function loadReports() {
  loading.value = true
  error.value = ''

  try {
    response.value = await fetchReports()
  } catch (reason) {
    response.value = null
    error.value = reason instanceof Error ? reason.message : '报告接口读取失败'
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  void loadReports()
})
</script>

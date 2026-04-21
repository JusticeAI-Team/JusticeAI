<template>
  <section>
    <header class="hero">
      <div class="wrap hero-grid">
        <div class="hero-meta" v-reveal="40">
          <div class="eyebrow"><span class="dot"></span>JusticeAI · {{ step.stageCode }} · {{ step.title }}</div>
          <div class="vol">{{ generatedAt === '-' ? '等待阶段接口' : `阶段更新时间 · ${generatedAt}` }}</div>
        </div>

        <h1 class="headline" v-reveal="100">
          先完成 {{ step.title }}.<br />
          <em>{{ step.shortLabel }}</em><br />
          再进入下一步.
        </h1>

        <p class="hero-sub" v-reveal="160">{{ step.usageLead }}</p>

        <div class="hero-cta" v-reveal="220">
          <RouterLink v-if="previousStep" class="btn" :to="previousStep.path">
            上一步 {{ previousStep.label }}
          </RouterLink>
          <RouterLink v-if="nextStep" class="btn primary" :to="nextStep.path">
            下一步 {{ nextStep.label }}
            <span class="arrow">→</span>
          </RouterLink>
          <span class="hero-note">
            <span class="bullet">•</span>
            {{ loading ? '阶段接口读取中' : error ? '阶段接口未接通' : readinessLabel }}
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
          :index="`§ ${step.stageCode} / 使用流程`"
          :title="`${step.title} 怎么 <em>使用</em>.<br>本页就看三件事.`"
          :lede="step.summary"
        />

        <RouteDiagram
          v-reveal
          :start="{
            label: previousStep ? '上一步' : '入口',
            name: previousStep ? previousStep.label : '流程总览',
            tags: previousStep ? ['已完成后再进入'] : ['从总览进入'],
          }"
          :current="{
            label: '当前页',
            name: step.title,
            tags: step.focus,
          }"
          :end="{
            label: nextStep ? '下一步' : '流程尾部',
            name: nextStep ? nextStep.label : '当前已是最后一步',
            tags: [step.nextRequirement],
          }"
          :legend="stepLegend"
        />
      </div>
    </section>

    <section class="swap">
      <div class="wrap swap-grid">
        <div class="swap-copy" v-reveal>
          <div class="eyebrow" style="margin-bottom: 16px"><span class="dot"></span>§ {{ step.stageCode }} / 操作顺序</div>
          <h3>先按步骤用.<br /><em>再看状态.</em></h3>
          <p>当前页只说明怎么操作、什么时候能进下一步，以及本阶段由哪些 Agent 参与。</p>
          <ul>
            <li v-for="item in step.operatorGuide" :key="item.index">
              <span class="n">{{ item.index }}</span>
              <span><strong>{{ item.title }}</strong> · {{ item.description }}</span>
            </li>
          </ul>
        </div>

        <div class="swap-code" v-reveal="120">
          <div class="code">
            <header>
              <div class="tabs">
                <div class="tab on">阶段状态</div>
                <div class="tab">进入条件</div>
              </div>
              <div class="filename">~/workflow/{{ step.key }}.status</div>
            </header>
            <pre>{{ stageStatusLog }}</pre>
          </div>
        </div>
      </div>
    </section>

    <section class="providers">
      <div class="wrap">
        <div class="prov-head" v-reveal>
          <h3>本阶段用了哪些 Agent.<br><em>直接看这里.</em></h3>
          <p class="note">每一页都明确展示参与 Agent，不让 Agentic workflow 藏在文案背后。</p>
        </div>

        <AgentGrid :agents="stepAgents" />
      </div>
    </section>

    <section v-if="presentation?.metrics.length" class="receipts">
      <div class="wrap receipts-grid">
        <div class="rcpt-head" v-reveal>
          <h3>先看快照,<br>再继续 <em>操作</em>.</h3>
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

    <section class="sheet-section">
      <div class="wrap">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>{{ step.title }} 当前状态</h4>
              <p>是否满足进入下一步条件，会直接显示在这里。</p>
            </div>
            <span class="status-chip" :class="toneClass(readinessTone)">{{ readinessLabel }}</span>
          </div>
          <div class="sheet-body">
            <StateFrame
              v-if="loading && !presentation"
              kind="loading"
              :title="`正在读取 ${step.title}`"
              :description="`正在获取 ${step.title} 阶段的当前快照。`"
            />
            <StateFrame
              v-else-if="error && !presentation"
              kind="disconnected"
              :title="`${step.title} 暂未接通`"
              :description="error"
              action-label="先回系统准备"
              action-to="/setup"
            />
            <StateFrame
              v-else-if="presentation?.state === 'empty'"
              kind="empty"
              :title="presentation.emptyTitle"
              :description="presentation.emptyDescription"
            />
            <div v-else class="sheet-grid">
              <div class="sheet-cell">
                <div class="k">进入下一步</div>
                <div class="v">{{ readinessLabel }}</div>
                <div class="d">{{ gateDescription }}</div>
              </div>
              <div class="sheet-cell">
                <div class="k">当前步骤</div>
                <div class="v">{{ step.stageCode }}</div>
                <div class="d">{{ step.title }}</div>
              </div>
              <div class="sheet-cell">
                <div class="k">更新时间</div>
                <div class="v">{{ generatedAt }}</div>
                <div class="d">如果接口未返回，这里会保持占位。</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section
      v-for="section in presentation?.sections ?? []"
      :key="`${step.key}-${section.title}`"
      class="sheet-section"
    >
      <div class="wrap">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>{{ section.title }}</h4>
              <p>{{ section.description }}</p>
            </div>
          </div>
          <div class="sheet-body">
            <template v-if="section.kind === 'key-value'">
              <div v-if="section.items.length > 0" class="sheet-grid">
                <div v-for="item in section.items" :key="`${section.title}-${item.label}`" class="sheet-cell">
                  <div class="k">{{ item.label }}</div>
                  <div class="v">{{ item.value }}</div>
                  <div class="d">{{ item.hint || '当前无附加说明。' }}</div>
                </div>
              </div>
              <div v-else class="sheet-empty">{{ section.emptyLabel || '当前没有可展示内容。' }}</div>
            </template>

            <template v-else-if="section.kind === 'table'">
              <div v-if="section.rows.length > 0" class="table-shell">
                <table class="sheet-table">
                  <thead>
                    <tr>
                      <th v-for="column in section.columns" :key="`${section.title}-${column}`">{{ column }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="row in section.rows" :key="`${section.title}-${row.join('|')}`">
                      <td v-for="cell in row" :key="`${section.title}-${row.join('|')}-${cell}`">{{ cell }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="sheet-empty">{{ section.emptyLabel || '当前没有可展示内容。' }}</div>
            </template>

            <template v-else>
              <div v-if="section.items.length > 0" class="sheet-list">
                <div v-for="item in section.items" :key="`${section.title}-${item.title}-${item.meta}`" class="sheet-row">
                  <div>
                    <strong>{{ item.title }}</strong>
                    <p v-if="item.subtitle">{{ item.subtitle }}</p>
                    <span v-if="item.meta">{{ item.meta }}</span>
                  </div>
                  <span v-if="item.status" class="status-chip" :class="toneClass(item.status)">{{ item.status }}</span>
                </div>
              </div>
              <div v-else class="sheet-empty">{{ section.emptyLabel || '当前没有可展示内容。' }}</div>
            </template>
          </div>
        </div>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { RouterLink } from 'vue-router'
import AgentGrid from '../components/reference/AgentGrid.vue'
import RouteDiagram from '../components/reference/RouteDiagram.vue'
import SectionHead from '../components/reference/SectionHead.vue'
import StateFrame from '../components/reference/StateFrame.vue'
import TickerBand from '../components/reference/TickerBand.vue'
import {
  getNextWorkflowStep,
  getPreviousWorkflowStep,
  getStepAgents,
  getWorkflowStep,
  toNavigationLink,
} from '../workflow/catalog'
import { stageLoaderMap, type DataDrivenStepKey, type StagePresentation } from '../workflow/presenters'
import { formatDateTime } from '../utils/format'

const props = defineProps<{
  stepKey: DataDrivenStepKey
}>()

const loading = ref(false)
const error = ref('')
const presentation = ref<StagePresentation | null>(null)

const step = computed(() => getWorkflowStep(props.stepKey))
const previousStep = computed(() => toNavigationLink(getPreviousWorkflowStep(props.stepKey)))
const nextStep = computed(() => toNavigationLink(getNextWorkflowStep(props.stepKey)))
const stepAgents = computed(() => getStepAgents(props.stepKey))
const readinessLabel = computed(() => presentation.value?.readiness.label ?? '等待接通')
const gateDescription = computed(() => presentation.value?.readiness.detail ?? step.value.nextRequirement)
const generatedAt = computed(() => formatDateTime(presentation.value?.generatedAt))
const readinessTone = computed(() => {
  if (error.value) {
    return 'danger'
  }

  return presentation.value?.readiness.ready ? 'good' : 'warning'
})

const stepLegend = computed(() => {
  return step.value.operatorGuide.map((item) => ({
    key: `${item.index} / ${item.title}`,
    value: item.description,
  }))
})

const heroStats = computed(() => {
  if (presentation.value?.metrics.length) {
    return presentation.value.metrics.slice(0, 4).map((item) => {
      const match = item.value.match(/^([0-9.]+)(.*)$/)
      return {
        label: item.label,
        value: match ? match[1] : item.value,
        unit: match ? match[2].trim() : '',
      }
    })
  }

  return [
    { label: '当前步骤', value: step.value.stageCode.replace('S', ''), unit: '' },
    { label: '参与 Agent', value: String(stepAgents.value.length), unit: '' },
    { label: '下一页条件', value: presentation.value?.readiness.ready ? '已满足' : '待满足', unit: '' },
    { label: '更新状态', value: error.value ? '未接通' : '可读取', unit: '' },
  ]
})

const receiptMetrics = computed(() => {
  return (
    presentation.value?.metrics.slice(0, 4).map((item) => {
      const match = item.value.match(/^([0-9.]+)(.*)$/)
      return {
        label: item.label,
        value: match ? match[1] : item.value,
        unit: match ? match[2].trim() : '',
        description: item.hint || '当前阶段接口已返回可展示快照。',
      }
    }) ?? []
  )
})

const stageStatusLog = computed(() => {
  return [
    `// ${step.value.title}`,
    `状态: ${loading.value ? '读取中' : error.value ? '未接通' : readinessLabel.value}`,
    `更新时间: ${generatedAt.value}`,
    `进入下一步: ${gateDescription.value}`,
    `参与 Agent: ${stepAgents.value.map((item) => item.label).join(' / ')}`,
    '',
    ...step.value.operatorGuide.map((item) => `${item.index} ${item.title} -> ${item.description}`),
  ].join('\n')
})

async function loadStagePresentation() {
  loading.value = true
  error.value = ''

  try {
    presentation.value = await stageLoaderMap[props.stepKey]()
  } catch (reason) {
    presentation.value = null
    error.value = reason instanceof Error ? reason.message : `${step.value.title} 阶段数据读取失败`
  } finally {
    loading.value = false
  }
}

function toneClass(source: string) {
  if (['good', '条件已满足', '配置可维护'].includes(source) || source.includes('已满足')) {
    return 'good'
  }

  if (['danger'].includes(source) || source.includes('未接通') || source.includes('失败')) {
    return 'danger'
  }

  return 'warning'
}

watch(
  () => props.stepKey,
  () => {
    void loadStagePresentation()
  },
  { immediate: true },
)
</script>

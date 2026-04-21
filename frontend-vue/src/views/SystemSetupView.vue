<template>
  <section>
    <header class="hero">
      <div class="wrap hero-grid">
        <div class="hero-meta" v-reveal="40">
          <div class="eyebrow"><span class="dot"></span>JusticeAI · {{ step.stageCode }} · {{ step.title }}</div>
          <div class="vol">{{ healthTimestamp === '-' ? '等待健康检查' : `最近检查 · ${healthTimestamp}` }}</div>
        </div>

        <h1 class="headline" v-reveal="100">
          先检查环境.<br />
          <em>确认基础服务已连通.</em><br />
          再进入归集.
        </h1>

        <p class="hero-sub" v-reveal="160">{{ step.usageLead }}</p>

        <div class="hero-cta" v-reveal="220">
          <button class="btn primary" type="button" :disabled="checking" @click="checkSystemStatus">
            {{ checking ? '检查中' : '重新检查' }}
            <span class="arrow">→</span>
          </button>
          <RouterLink class="btn" to="/data-ingestion">下一步 数据归集</RouterLink>
          <span class="hero-note">
            <span class="bullet">•</span>
            {{ checking ? '检查中' : canProceed ? '基础接口可进入下一步' : '先完成接口检查' }}
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
          index="§ S01 / 检查顺序"
          title="本页先看什么.<br>按 <em>这个顺序</em> 检查."
          lede="后端接口、健康状态和平台配置都返回后，再进入数据归集，不需要在本页停留太久。"
        />

        <RouteDiagram
          v-reveal
          :start="{ label: '前一步', name: '流程总览', tags: ['先看全流程'] }"
          :current="{ label: '当前页', name: '系统准备', tags: step.focus }"
          :end="{ label: '下一步', name: '数据归集', tags: ['上传文件', '生成批次'] }"
          :legend="stepLegend"
        />
      </div>
    </section>

    <section class="swap">
      <div class="wrap swap-grid">
        <div class="swap-copy" v-reveal>
          <div class="eyebrow" style="margin-bottom: 16px"><span class="dot"></span>§ S01 / 使用路径</div>
          <h3>先开服务.<br /><em>再点检查.</em></h3>
          <p>这里不做业务操作，只确认当前环境已经满足进入归集页的条件。</p>
          <ul>
            <li v-for="item in commandSteps" :key="item.index">
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
                <div class="tab">启动命令</div>
              </div>
              <div class="filename">~/workflow/setup.status</div>
            </header>
            <pre>{{ statusLog }}</pre>
          </div>
        </div>
      </div>
    </section>

    <section class="providers">
      <div class="wrap">
        <div class="prov-head" v-reveal>
          <h3>本阶段主要由<br><em>监督协调 Agent</em> 持续记录.</h3>
          <p class="note">系统准备阶段先以人工检查为主，监督协调 Agent 负责持续记录运行状态和后续联调基础。</p>
        </div>

        <AgentGrid :agents="stepAgents" />
      </div>
    </section>

    <section class="sheet-section">
      <div class="wrap">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>进入数据归集前</h4>
              <p>这里直接显示当前是否已经满足下一步条件。</p>
            </div>
            <span class="status-chip" :class="canProceed ? 'good' : 'warning'">{{ gateLabel }}</span>
          </div>
          <div class="sheet-body">
            <StateFrame
              v-if="checking && !systemInfo && !healthInfo && !settings"
              kind="loading"
              title="正在检查系统联通"
              description="正在读取系统信息、健康状态和平台设置。"
            />
            <StateFrame
              v-else-if="!systemInfo && !healthInfo && !settings && errors.length > 0"
              kind="disconnected"
              title="当前仍未满足进入条件"
              description="至少有一个基础接口不可读。先启动后端，再回来重新检查。"
            />
            <div v-else class="sheet-grid">
              <div class="sheet-cell">
                <div class="k">进入下一步</div>
                <div class="v">{{ gateLabel }}</div>
                <div class="d">{{ gateDescription }}</div>
              </div>
              <div class="sheet-cell">
                <div class="k">系统信息</div>
                <div class="v">{{ systemInfo ? '已返回' : '未返回' }}</div>
                <div class="d">决定是否允许继续进入归集页。</div>
              </div>
              <div class="sheet-cell">
                <div class="k">平台设置</div>
                <div class="v">{{ settings ? '已返回' : '未返回' }}</div>
                <div class="d">用于补充模型名、环境和接口入口。</div>
              </div>
            </div>

            <div v-if="errors.length > 0" class="sheet-empty">
              {{ errors.join('；') }}
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
              <h4>运行配置</h4>
              <p>检查当前应用、模型、图谱和目录配置。</p>
            </div>
          </div>
          <div class="sheet-body">
            <div v-if="runtimeItems.length > 0" class="sheet-grid">
              <div v-for="item in runtimeItems" :key="item.label" class="sheet-cell">
                <div class="k">{{ item.label }}</div>
                <div class="v">{{ item.value }}</div>
                <div class="d">{{ item.hint }}</div>
              </div>
            </div>
            <div v-else class="sheet-empty">等待 system/info 接口返回后显示。</div>
          </div>
        </div>
      </div>
    </section>

    <section class="sheet-section">
      <div class="wrap">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>依赖健康</h4>
              <p>只看依赖状态，不在这里展开额外解释。</p>
            </div>
          </div>
          <div class="sheet-body">
            <div v-if="dependencyItems.length > 0" class="sheet-list">
              <div v-for="item in dependencyItems" :key="item.label" class="sheet-row">
                <div>
                  <strong>{{ item.label }}</strong>
                  <p>{{ item.subtitle }}</p>
                </div>
                <span class="status-chip" :class="toneClass(item.status)">{{ item.status }}</span>
              </div>
            </div>
            <div v-else class="sheet-empty">等待 /health 返回 PostgreSQL、HugeGraph、vLLM 和 Milvus 状态。</div>
          </div>
        </div>
      </div>
    </section>

    <section class="sheet-section">
      <div class="wrap">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>平台设置与集成</h4>
              <p>平台基础配置和集成清单都在这里看。</p>
            </div>
          </div>
          <div class="sheet-body">
            <div v-if="platformItems.length > 0" class="sheet-grid" style="margin-bottom: 28px">
              <div v-for="item in platformItems" :key="item.label" class="sheet-cell">
                <div class="k">{{ item.label }}</div>
                <div class="v">{{ item.value }}</div>
                <div class="d">{{ item.hint }}</div>
              </div>
            </div>

            <div v-if="integrationItems.length > 0" class="sheet-list">
              <div v-for="item in integrationItems" :key="item.title" class="sheet-row">
                <div>
                  <strong>{{ item.title }}</strong>
                  <p>{{ item.subtitle }}</p>
                  <span>{{ item.meta }}</span>
                </div>
                <span class="status-chip" :class="toneClass(item.status)">{{ item.status }}</span>
              </div>
            </div>
            <div v-else class="sheet-empty">等待 settings/platform 返回平台配置与集成清单。</div>
          </div>
        </div>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import AgentGrid from '../components/reference/AgentGrid.vue'
import RouteDiagram from '../components/reference/RouteDiagram.vue'
import SectionHead from '../components/reference/SectionHead.vue'
import StateFrame from '../components/reference/StateFrame.vue'
import TickerBand from '../components/reference/TickerBand.vue'
import { fetchHealth, fetchPlatformSettings, fetchSystemInfo, type HealthResponse, type SystemInfoResponse } from '../api/system'
import type { PlatformSettingsResponse } from '../types/workspace'
import { formatDateTime } from '../utils/format'
import { getStepAgents, getWorkflowStep } from '../workflow/catalog'

const step = getWorkflowStep('setup')
const stepAgents = getStepAgents('setup')

const checking = ref(false)
const errors = ref<string[]>([])
const systemInfo = ref<SystemInfoResponse | null>(null)
const healthInfo = ref<HealthResponse | null>(null)
const settings = ref<PlatformSettingsResponse | null>(null)

const canProceed = computed(() => systemInfo.value !== null)
const healthTimestamp = computed(() => formatDateTime(healthInfo.value?.timestamp))

const gateLabel = computed(() => {
  if (!systemInfo.value) {
    return '等待接口返回'
  }

  if (healthInfo.value?.status === 'degraded') {
    return '带告警可继续'
  }

  return '条件已满足'
})

const gateDescription = computed(() => {
  if (!systemInfo.value) {
    return step.nextRequirement
  }

  if (healthInfo.value?.status === 'degraded') {
    return '系统信息已返回，但部分依赖处于降级状态。可以继续联调，同时保留后续修正。'
  }

  return '基础接口已经联通，可以进入数据归集。'
})

const heroStats = computed(() => [
  { label: '系统信息', value: systemInfo.value ? '1' : '0', unit: '' },
  { label: '健康检查', value: healthInfo.value ? '1' : '0', unit: '' },
  { label: '平台设置', value: settings.value ? '1' : '0', unit: '' },
  { label: '下一步条件', value: canProceed.value ? '已满足' : '待满足', unit: '' },
])

const stepLegend = computed(() => step.operatorGuide.map((item) => ({ key: `${item.index} / ${item.title}`, value: item.description })))

const runtimeItems = computed(() => {
  if (!systemInfo.value) {
    return []
  }

  return [
    { label: '应用名称', value: systemInfo.value.app.name, hint: `环境 ${systemInfo.value.app.env}` },
    { label: '监听地址', value: `${systemInfo.value.app.host}:${systemInfo.value.app.port}`, hint: `版本 ${systemInfo.value.runtime.version}` },
    { label: 'vLLM 模型', value: systemInfo.value.vllm.model_name, hint: systemInfo.value.vllm.base_url },
    { label: 'HugeGraph', value: systemInfo.value.hugegraph.base_url, hint: systemInfo.value.hugegraph.gremlin_url },
    { label: 'Milvus', value: systemInfo.value.milvus.address, hint: '向量检索地址' },
    { label: '上传目录', value: systemInfo.value.storage.upload_dir, hint: `报告目录 ${systemInfo.value.storage.report_dir}` },
  ]
})

const dependencyItems = computed(() => {
  if (!healthInfo.value) {
    return []
  }

  return [
    { label: 'PostgreSQL', subtitle: '关系型数据库', status: healthInfo.value.dependencies.postgres },
    { label: 'HugeGraph', subtitle: '图谱服务', status: healthInfo.value.dependencies.hugegraph },
    { label: 'vLLM', subtitle: '模型推理服务', status: healthInfo.value.dependencies.vllm },
    { label: 'Milvus', subtitle: '向量数据库', status: healthInfo.value.dependencies.milvus },
  ]
})

const platformItems = computed(() => {
  if (!settings.value) {
    return []
  }

  return [
    { label: '平台名称', value: settings.value.platform.app_name, hint: `环境 ${settings.value.platform.environment}` },
    { label: 'API 基路径', value: settings.value.platform.api_base_path, hint: '前端调用入口' },
    { label: '默认模型', value: settings.value.platform.model_name, hint: '当前主模型配置' },
  ]
})

const integrationItems = computed(() => {
  return (
    settings.value?.integrations.map((item) => ({
      title: item.label,
      subtitle: item.endpoint,
      meta: `集成键 ${item.key}`,
      status: item.status,
    })) ?? []
  )
})

const commandSteps = [
  { index: '01', title: '启动后端', description: '先确保后端服务已经运行。' },
  { index: '02', title: '点击检查', description: '返回本页读取系统信息、健康状态和平台设置。' },
  { index: '03', title: '进入归集', description: '看到系统信息已返回后，直接进入数据归集页。' },
]

const statusLog = computed(() => {
  return [
    '// 系统准备',
    `系统信息: ${systemInfo.value ? '已返回' : '未返回'}`,
    `健康状态: ${healthInfo.value ? healthInfo.value.status : '未返回'}`,
    `平台设置: ${settings.value ? '已返回' : '未返回'}`,
    `进入下一步: ${gateDescription.value}`,
    '',
    '01 cargo run --manifest-path backend-rust/Cargo.toml',
    '02 npm --prefix frontend-vue run dev',
    '03 返回本页点击“重新检查”',
  ].join('\n')
})

function normalizeError(prefix: string, reason: unknown) {
  const message = reason instanceof Error ? reason.message : `${prefix} 读取失败`
  return `${prefix}：${message}`
}

async function checkSystemStatus() {
  checking.value = true
  errors.value = []

  const [systemResult, healthResult, settingsResult] = await Promise.allSettled([
    fetchSystemInfo(),
    fetchHealth(),
    fetchPlatformSettings(),
  ])

  const nextErrors: string[] = []

  if (systemResult.status === 'fulfilled') {
    systemInfo.value = systemResult.value
  } else {
    systemInfo.value = null
    nextErrors.push(normalizeError('系统信息', systemResult.reason))
  }

  if (healthResult.status === 'fulfilled') {
    healthInfo.value = healthResult.value
  } else {
    healthInfo.value = null
    nextErrors.push(normalizeError('健康检查', healthResult.reason))
  }

  if (settingsResult.status === 'fulfilled') {
    settings.value = settingsResult.value
  } else {
    settings.value = null
    nextErrors.push(normalizeError('平台设置', settingsResult.reason))
  }

  errors.value = nextErrors
  checking.value = false
}

function toneClass(status: string) {
  const value = status.toLowerCase()

  if (['healthy', 'ready', 'success', '已返回'].some((item) => value.includes(item))) {
    return 'good'
  }

  if (['critical', 'failed', 'error', 'down'].some((item) => value.includes(item)) || status.includes('失败')) {
    return 'danger'
  }

  return 'warning'
}

onMounted(() => {
  void checkSystemStatus()
})
</script>

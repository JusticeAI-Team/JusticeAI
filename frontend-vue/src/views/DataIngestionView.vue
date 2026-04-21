<template>
  <section>
    <header class="hero hero-stage hero-ingestion">
      <div class="wrap hero-grid">
        <div class="hero-meta" v-reveal="40">
          <div class="eyebrow"><span class="dot"></span>JusticeAI · {{ step.stageCode }} · {{ step.title }}</div>
          <div class="vol">{{ snapshotUpdatedAt === '-' ? '等待归集接口' : `归集更新时间 · ${snapshotUpdatedAt}` }}</div>
        </div>

        <h1 class="headline" v-reveal="100">
          数据先送进来.<br />
          <em>批次先跑起来.</em><br />
          再往后走映射与抽取.
        </h1>

        <p class="hero-sub" v-reveal="160">
          本页只做上传、看批次、看兼容记录、看导入详情四件事；先把第一批数据送进系统，再进入下一步。
        </p>

        <div class="hero-cta" v-reveal="220">
          <button class="btn primary" type="button" :disabled="isBusy" @click="refreshAll">
            {{ isBusy ? '刷新中' : '刷新本页' }}
            <span class="arrow">→</span>
          </button>
          <RouterLink class="btn" to="/data-mapping">下一步 数据映射</RouterLink>
          <span class="hero-note">
            <span class="bullet">•</span>
            {{ stageLoading ? '归集接口读取中' : stageErrors.length > 0 && !hasSnapshot ? '归集接口未接通' : gateLabel }}
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
          index="§ S02 / 上传主场"
          title="先把文件 <em>放进系统</em>.<br>再看它有没有真正进来."
          lede="数据归集页是整个系统最强的真实交互入口：先上传，再看批次，再核对兼容记录和导入详情。"
        />

        <RouteDiagram
          v-reveal
          :start="{ label: '上一步', name: '系统准备', tags: ['先确认接口联通'] }"
          :current="{ label: '当前页', name: '数据归集', tags: ['上传', '批次', '导入记录', '详情'] }"
          :end="{ label: '下一步', name: '数据映射', tags: ['至少已有一批数据'] }"
          :legend="stepLegend"
        />
      </div>
    </section>

    <section class="sheet-section ingestion-stage-section">
      <div class="wrap section-stack">
        <div class="ingestion-hero-grid">
          <div class="sheet-shell upload-stage-shell" v-reveal>
            <div class="sheet-head">
              <div>
                <h4>上传入口</h4>
                <p>先把本月 Excel 放进系统，上传后立刻看批次是否生成。</p>
              </div>
              <span class="status-chip" :class="canProceed ? 'good' : 'warning'">{{ gateLabel }}</span>
            </div>
            <div class="sheet-body upload-stage-body">
              <div class="upload-stage-box">
                <div class="upload-stage-copy">
                  <div class="eyebrow">支持 csv / xls / xlsx</div>
                  <h3>把文件拖进来或直接选择文件</h3>
                  <p>上传完成后先看批次，再看兼容导入记录和详情。</p>
                </div>

                <div class="upload-stage-controls">
                  <input
                    ref="fileInput"
                    class="file-input file-input-wide"
                    type="file"
                    accept=".csv,.xls,.xlsx"
                    :disabled="isBusy"
                    @change="handleFileChange"
                  />
                  <button class="btn primary btn-wide" type="button" :disabled="!selectedFile || isBusy" @click="handleUpload">
                    {{ uploading ? '上传中' : '上传文件' }}
                  </button>
                </div>

                <div class="upload-stage-feedback">
                  <p class="muted-line">单文件不超过 10 MB。</p>
                  <p v-if="selectedFile" class="muted-line">当前文件 · {{ selectedFile.name }}</p>
                  <p v-if="uploadSuccessMessage" class="muted-line msg-good">{{ uploadSuccessMessage }}</p>
                  <p v-if="uploadError" class="muted-line msg-bad">{{ uploadError }}</p>
                  <p v-if="stageErrors.length > 0" class="muted-line msg-bad">{{ stageErrors.join('；') }}</p>
                </div>
              </div>
            </div>
          </div>

          <div class="sheet-shell upload-side-shell" v-reveal="80">
            <div class="sheet-head">
              <div>
                <h4>进入映射前</h4>
                <p>这里只看是否已经具备继续进入下一页的条件。</p>
              </div>
            </div>
            <div class="sheet-body">
              <StateFrame
                v-if="stageLoading && !hasSnapshot"
                kind="loading"
                title="正在读取归集快照"
                description="正在请求归集总览和归集批次接口。"
              />
              <StateFrame
                v-else-if="!hasSnapshot && stageErrors.length > 0"
                kind="disconnected"
                title="归集接口暂未接通"
                description="请先完成系统准备，再返回本页继续联调。"
                action-label="回到系统准备"
                action-to="/setup"
              />
              <div v-else class="sheet-grid">
                <div class="sheet-cell">
                  <div class="k">进入下一步</div>
                  <div class="v">{{ gateLabel }}</div>
                  <div class="d">{{ step.nextRequirement }}</div>
                </div>
                <div class="sheet-cell">
                  <div class="k">归集批次</div>
                  <div class="v">{{ batches.value?.items.length ?? 0 }}</div>
                  <div class="d">至少出现一批数据后，再进入映射页。</div>
                </div>
                <div class="sheet-cell">
                  <div class="k">兼容记录</div>
                  <div class="v">{{ items.length }}</div>
                  <div class="d">旧 /imports 的兼容记录继续在这里看。</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="receipts">
      <div class="wrap receipts-grid">
        <div class="rcpt-head" v-reveal>
          <h3>上传之后.<br>先看 <em>批次快照</em>.</h3>
        </div>

        <div v-for="item in snapshotMetrics" :key="item.label" class="rcpt" v-reveal>
          <div class="rn">{{ item.value }}</div>
          <div class="rl">{{ item.label }}</div>
          <div class="rd">{{ item.hint }}</div>
        </div>
      </div>
    </section>

    <section class="sheet-section">
      <div class="wrap">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>来源汇总</h4>
              <p>先看各来源是否已经形成记录和批次。</p>
            </div>
          </div>
          <div class="sheet-body">
            <div v-if="sourceItems.length > 0" class="source-summary-grid">
              <div v-for="item in sourceItems" :key="item.title" class="source-summary-card">
                <div class="k">{{ item.title }}</div>
                <div class="v">{{ item.value }}</div>
                <div class="d">{{ item.hint }}</div>
              </div>
            </div>
            <div v-else class="sheet-empty">当前还没有来源汇总统计。</div>
          </div>
        </div>
      </div>
    </section>

    <section class="sheet-section">
      <div class="wrap ingestion-layout-grid">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>归集批次</h4>
              <p>上传后先看这里，确认批次是否已经生成。</p>
            </div>
          </div>
          <div class="sheet-body">
            <div v-if="batchRows.length > 0" class="table-shell">
              <table class="sheet-table">
                <thead>
                  <tr>
                    <th>批次 ID</th>
                    <th>来源</th>
                    <th>文件名</th>
                    <th>状态</th>
                    <th>记录数</th>
                    <th>错误数</th>
                    <th>导入时间</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="row in batchRows" :key="row.id">
                    <td>{{ row.id }}</td>
                    <td>{{ row.sourceLabel }}</td>
                    <td>{{ row.fileName }}</td>
                    <td>{{ row.status }}</td>
                    <td>{{ row.recordCount }}</td>
                    <td>{{ row.errorCount }}</td>
                    <td>{{ row.importedAt }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div v-else class="sheet-empty">当前还没有归集批次。</div>
          </div>
        </div>

        <div class="sheet-shell" v-reveal="80">
          <div class="sheet-head">
            <div>
              <h4>阶段状态</h4>
              <p>当前上传、批次、兼容记录和详情都会反映在这里。</p>
            </div>
          </div>
          <div class="sheet-body">
            <div class="code code-inline">
              <header>
                <div class="tabs">
                  <div class="tab on">阶段状态</div>
                </div>
                <div class="filename">~/workflow/data-ingestion.status</div>
              </header>
              <pre>{{ statusLog }}</pre>
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
              <h4>兼容导入记录</h4>
              <p>旧 /imports 主入口已收束到这里继续使用。</p>
            </div>
            <div class="inline-actions">
              <label class="muted-line">
                状态筛选
                <select v-model="statusFilter" class="select-input" :disabled="isBusy" @change="handleStatusChange">
                  <option value="">全部</option>
                  <option value="uploaded">uploaded</option>
                </select>
              </label>
              <button class="btn ghost" type="button" :disabled="isBusy" @click="refreshImportList">
                {{ listLoading ? '刷新中' : '刷新记录' }}
              </button>
            </div>
          </div>
          <div class="sheet-body">
            <StateFrame
              v-if="listLoading && items.length === 0"
              kind="loading"
              title="正在读取导入记录"
              description="兼容导入记录接口读取中。"
            />
            <StateFrame
              v-else-if="listError && items.length === 0"
              kind="disconnected"
              title="导入记录暂未接通"
              :description="listError"
            />
            <StateFrame
              v-else-if="items.length === 0"
              kind="empty"
              title="当前没有兼容导入记录"
              description="如果旧记录为空，这里保持空态。"
            />
            <template v-else>
              <div class="table-shell">
                <table class="sheet-table">
                  <thead>
                    <tr>
                      <th>导入 ID</th>
                      <th>来源类型</th>
                      <th>状态</th>
                      <th>创建时间</th>
                      <th>更新时间</th>
                      <th>操作</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="item in items" :key="item.id">
                      <td>{{ item.id }}</td>
                      <td>{{ item.source_type }}</td>
                      <td>{{ item.status }}</td>
                      <td>{{ formatDateTime(item.created_at) }}</td>
                      <td>{{ formatDateTime(item.updated_at) }}</td>
                      <td>
                        <button class="btn ghost" type="button" :disabled="detailLoading" @click="handleSelectImport(item.id)">
                          {{ detailLoading && selectedImportId === item.id ? '读取中' : '查看详情' }}
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div class="route-pager" style="margin-top: 18px">
                <button class="btn ghost" type="button" :disabled="!hasPrevPage || isBusy" @click="handlePreviousPage">
                  上一页
                </button>
                <span class="muted-line">第 {{ page }} 页 / 共 {{ totalPages || 1 }} 页</span>
                <button class="btn ghost" type="button" :disabled="!hasNextPage || isBusy" @click="handleNextPage">
                  下一页
                </button>
              </div>
            </template>
          </div>
        </div>
      </div>
    </section>

    <section class="sheet-section">
      <div class="wrap">
        <div class="sheet-shell" v-reveal>
          <div class="sheet-head">
            <div>
              <h4>导入详情</h4>
              <p>从兼容记录里选一条后，在这里看文件明细。</p>
            </div>
          </div>
          <div class="sheet-body">
            <StateFrame
              v-if="detailLoading && !detail"
              kind="loading"
              title="正在读取导入详情"
              description="请稍候，正在获取当前导入记录详情。"
            />
            <StateFrame
              v-else-if="detailError && !detail"
              kind="disconnected"
              title="导入详情读取失败"
              :description="detailError"
            />
            <StateFrame
              v-else-if="!detail"
              kind="empty"
              title="尚未选择导入记录"
              description="从上方兼容导入记录中选择一条后，这里才会显示详情。"
            />
            <template v-else>
              <div class="sheet-grid" style="margin-bottom: 28px">
                <div v-for="item in detailCards" :key="item.label" class="sheet-cell">
                  <div class="k">{{ item.label }}</div>
                  <div class="v">{{ item.value }}</div>
                  <div class="d">{{ item.hint || '当前字段无附加说明。' }}</div>
                </div>
              </div>

              <div class="table-shell">
                <table class="sheet-table">
                  <thead>
                    <tr>
                      <th>原文件名</th>
                      <th>存储文件名</th>
                      <th>相对路径</th>
                      <th>大小</th>
                      <th>MIME</th>
                      <th>创建时间</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="file in detail.files" :key="file.id">
                      <td>{{ file.original_filename }}</td>
                      <td>{{ file.stored_filename }}</td>
                      <td>{{ file.stored_path }}</td>
                      <td>{{ formatFileSize(file.file_size) }}</td>
                      <td>{{ file.mime_type || '-' }}</td>
                      <td>{{ formatDateTime(file.created_at) }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </template>
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
import {
  fetchImportDetail,
  fetchImportList,
  uploadImportFile,
  type ImportDetailResponse,
  type ImportListItem,
} from '../api/imports'
import { fetchIngestionBatches, fetchIngestionSummary } from '../api/workflow'
import type { IngestionBatchListResponse, IngestionSummaryResponse } from '../types/workspace'
import { formatDateTime, formatFileSize, formatMetricValue, formatNumber } from '../utils/format'
import { getWorkflowStep } from '../workflow/catalog'

const step = getWorkflowStep('data-ingestion')

const pageSize = 20
const maxUploadFileBytes = 10 * 1024 * 1024
const allowedUploadExtensions = ['csv', 'xls', 'xlsx']

const stageLoading = ref(false)
const stageErrors = ref<string[]>([])
const summary = ref<IngestionSummaryResponse | null>(null)
const batches = ref<IngestionBatchListResponse | null>(null)

const items = ref<ImportListItem[]>([])
const total = ref(0)
const page = ref(1)
const statusFilter = ref('')
const listLoading = ref(false)
const listError = ref('')

const selectedImportId = ref('')
const detail = ref<ImportDetailResponse | null>(null)
const detailLoading = ref(false)
const detailError = ref('')

const selectedFile = ref<File | null>(null)
const uploading = ref(false)
const uploadError = ref('')
const uploadSuccessMessage = ref('')
const fileInput = ref<HTMLInputElement | null>(null)

const isBusy = computed(() => stageLoading.value || listLoading.value || detailLoading.value || uploading.value)
const totalPages = computed(() => (total.value === 0 ? 0 : Math.ceil(total.value / pageSize)))
const hasPrevPage = computed(() => page.value > 1)
const hasNextPage = computed(() => totalPages.value > 0 && page.value < totalPages.value)
const hasSnapshot = computed(() => summary.value !== null || batches.value !== null)
const snapshotUpdatedAt = computed(() => formatDateTime(summary.value?.generated_at || batches.value?.generated_at))

const canProceed = computed(() => {
  return (
    (summary.value?.sources.some((item) => item.batch_count > 0 || item.record_count > 0) ?? false) ||
    (batches.value?.items.length ?? 0) > 0 ||
    items.value.length > 0
  )
})

const gateLabel = computed(() => (canProceed.value ? '条件已满足' : '等待首批数据'))

const heroStats = computed(() => [
  { label: '来源汇总', value: String(summary.value?.sources.length ?? 0), unit: '' },
  { label: '归集批次', value: String(batches.value?.items.length ?? 0), unit: '' },
  { label: '兼容记录', value: String(items.value.length), unit: '' },
  { label: '进入下一页', value: canProceed.value ? '已满足' : '待满足', unit: '' },
])

const stepLegend = [
  { key: '01 / 上传文件', value: '先把本月 Excel 上传到本页。' },
  { key: '02 / 看批次', value: '确认是否已生成批次以及记录数、错误数。' },
  { key: '03 / 看兼容记录', value: '旧导入入口继续在本页兼容可用。' },
  { key: '04 / 看详情', value: '选一条记录后继续看文件详情。' },
]

const snapshotMetrics = computed(() => {
  if (summary.value?.totals.length) {
    return summary.value.totals.slice(0, 4).map((metric) => ({
      label: metric.label,
      value: formatMetricValue(metric.value, metric.unit),
      hint: [metric.trend, metric.trend_value].filter(Boolean).join(' · ') || '当前归集总览指标',
    }))
  }

  return [
    { label: '兼容导入记录', value: formatNumber(items.value.length), hint: '来自旧 /imports 列表。' },
    { label: '归集批次数', value: formatNumber(batches.value?.items.length ?? 0), hint: '来自 ingestion/batches。' },
    { label: '进入映射条件', value: canProceed.value ? '已满足' : '待满足', hint: step.nextRequirement },
  ]
})

const sourceItems = computed(() => {
  return (
    summary.value?.sources.map((item) => ({
      title: item.source_label,
      value: `${formatNumber(item.record_count)} 条`,
      hint: `批次 ${formatNumber(item.batch_count)} · 最近导入 ${formatDateTime(item.latest_import_at)}`,
    })) ?? []
  )
})

const batchRows = computed(() => {
  return (
    batches.value?.items.map((item) => ({
      id: item.id,
      sourceLabel: item.source_label,
      fileName: item.file_name,
      status: item.status,
      recordCount: formatNumber(item.record_count),
      errorCount: formatNumber(item.error_count),
      importedAt: formatDateTime(item.imported_at),
    })) ?? []
  )
})

const detailCards = computed(() => {
  if (!detail.value) {
    return []
  }

  return [
    { label: '导入 ID', value: detail.value.id, hint: '当前选中的兼容导入记录。' },
    { label: '来源类型', value: detail.value.source_type, hint: '用于识别来源类别。' },
    { label: '当前状态', value: detail.value.status, hint: '导入记录当前状态。' },
    { label: '创建时间', value: formatDateTime(detail.value.created_at), hint: '记录创建时间。' },
    { label: '更新时间', value: formatDateTime(detail.value.updated_at), hint: '记录最近更新时间。' },
    { label: '文件数量', value: formatNumber(detail.value.files.length), hint: '当前记录下的关联文件数。' },
  ]
})

const statusLog = computed(() => {
  return [
    '// 数据归集',
    `批次接口: ${stageLoading.value ? '读取中' : stageErrors.length > 0 && !hasSnapshot.value ? '未接通' : '已返回'}`,
    `兼容记录: ${listLoading.value ? '读取中' : listError.value ? '未接通' : `${items.value.length} 条`}`,
    `导入详情: ${detailLoading.value ? '读取中' : detail.value ? '已返回' : '未选择'}`,
    `进入下一步: ${canProceed.value ? '已满足' : step.nextRequirement}`,
    '',
    '01 上传文件',
    '02 查看归集批次',
    '03 查看兼容导入记录',
    '04 选择记录查看详情',
  ].join('\n')
})

function normalizeError(prefix: string, reason: unknown) {
  const message = reason instanceof Error ? reason.message : `${prefix} 读取失败`
  return `${prefix}：${message}`
}

function clearSelectedFile() {
  selectedFile.value = null

  if (fileInput.value) {
    fileInput.value.value = ''
  }
}

function handleFileChange(event: Event) {
  const target = event.target as HTMLInputElement
  const [file] = Array.from(target.files ?? [])

  uploadError.value = ''
  uploadSuccessMessage.value = ''

  if (!file) {
    selectedFile.value = null
    return
  }

  const extension = file.name.split('.').pop()?.toLowerCase() ?? ''

  if (!allowedUploadExtensions.includes(extension)) {
    clearSelectedFile()
    uploadError.value = '仅支持 xlsx、xls、csv 文件'
    return
  }

  if (file.size === 0) {
    clearSelectedFile()
    uploadError.value = '上传文件不能为空'
    return
  }

  if (file.size > maxUploadFileBytes) {
    clearSelectedFile()
    uploadError.value = '上传文件不能超过 10 MB'
    return
  }

  selectedFile.value = file
}

async function loadSnapshot() {
  stageLoading.value = true
  stageErrors.value = []

  const [summaryResult, batchResult] = await Promise.allSettled([fetchIngestionSummary(), fetchIngestionBatches()])
  const nextErrors: string[] = []

  if (summaryResult.status === 'fulfilled') {
    summary.value = summaryResult.value
  } else {
    summary.value = null
    nextErrors.push(normalizeError('归集总览', summaryResult.reason))
  }

  if (batchResult.status === 'fulfilled') {
    batches.value = batchResult.value
  } else {
    batches.value = null
    nextErrors.push(normalizeError('归集批次', batchResult.reason))
  }

  stageErrors.value = nextErrors
  stageLoading.value = false
}

async function loadImportList(targetPage = page.value) {
  listLoading.value = true
  listError.value = ''

  try {
    const response = await fetchImportList({
      page: targetPage,
      pageSize,
      status: statusFilter.value || undefined,
    })

    items.value = response.items
    total.value = response.total
    page.value = response.page

    if (response.items.length === 0) {
      selectedImportId.value = ''
      detail.value = null
      detailError.value = ''
      return
    }

    const nextImportId =
      selectedImportId.value && response.items.some((item) => item.id === selectedImportId.value)
        ? selectedImportId.value
        : response.items[0].id

    await loadImportDetail(nextImportId)
  } catch (reason) {
    items.value = []
    total.value = 0
    listError.value = reason instanceof Error ? reason.message : '导入记录读取失败'
  } finally {
    listLoading.value = false
  }
}

async function loadImportDetail(importId: string) {
  selectedImportId.value = importId
  detailLoading.value = true
  detailError.value = ''

  try {
    detail.value = await fetchImportDetail(importId)
  } catch (reason) {
    detail.value = null
    detailError.value = reason instanceof Error ? reason.message : '导入详情读取失败'
  } finally {
    detailLoading.value = false
  }
}

async function refreshImportList() {
  await loadImportList(page.value)
}

async function refreshAll() {
  await Promise.all([loadSnapshot(), loadImportList(page.value)])
}

async function handleUpload() {
  const file = selectedFile.value

  if (!file) {
    return
  }

  uploading.value = true
  uploadError.value = ''
  uploadSuccessMessage.value = ''

  try {
    const response = await uploadImportFile(file)
    uploadSuccessMessage.value = `上传成功：${response.file.original_filename}`
    clearSelectedFile()
    await refreshAll()
    await loadImportDetail(response.import_id)
  } catch (reason) {
    uploadError.value = reason instanceof Error ? reason.message : '上传失败'
  } finally {
    uploading.value = false
  }
}

async function handleStatusChange() {
  await loadImportList(1)
}

async function handlePreviousPage() {
  if (!hasPrevPage.value) {
    return
  }

  await loadImportList(page.value - 1)
}

async function handleNextPage() {
  if (!hasNextPage.value) {
    return
  }

  await loadImportList(page.value + 1)
}

async function handleSelectImport(importId: string) {
  await loadImportDetail(importId)
}

onMounted(() => {
  void refreshAll()
})
</script>

<template>
  <main class="page">
    <h1>导入中心</h1>
    <p class="hint">上传 → 列表 → 详情 最小闭环联调页。</p>

    <section class="panel">
      <h2>上传区</h2>
      <div class="upload-row">
        <input
          ref="fileInput"
          type="file"
          accept=".csv,.xls,.xlsx"
          :disabled="uploading"
          @change="handleFileChange"
        />
        <button type="button" :disabled="!selectedFile || uploading" @click="handleUpload">
          {{ uploading ? '上传中...' : '上传文件' }}
        </button>
      </div>
      <p v-if="selectedFile" class="hint">已选择：{{ selectedFile.name }}</p>
      <p class="hint">仅支持 csv、xls、xlsx，且文件不能超过 10 MB。</p>
      <p v-if="uploadSuccessMessage" class="success">{{ uploadSuccessMessage }}</p>
      <p v-if="uploadError" class="error">{{ uploadError }}</p>
    </section>

    <section class="panel">
      <h2>筛选与刷新</h2>
      <div class="toolbar">
        <label>
          状态：
          <select v-model="statusFilter" :disabled="uploading || listLoading" @change="handleStatusChange">
            <option value="">全部</option>
            <option value="uploaded">uploaded</option>
          </select>
        </label>

        <button type="button" :disabled="listLoading || uploading" @click="refreshList">
          {{ listLoading ? '刷新中...' : '刷新列表' }}
        </button>
      </div>
    </section>

    <section class="panel">
      <div class="section-header">
        <h2>导入列表</h2>
        <span v-if="total > 0" class="hint">第 {{ page }} 页 / 共 {{ totalPages }} 页 · 共 {{ total }} 条</span>
        <span v-else class="hint">{{ listSummaryText }}</span>
      </div>

      <p v-if="listLoading" class="hint">列表加载中...</p>
      <p v-else-if="listError" class="error">{{ listError }}</p>
      <p v-else-if="items.length === 0" class="hint">{{ listEmptyMessage }}</p>

      <div v-else class="table-wrapper">
        <table class="table">
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
            <tr v-for="item in items" :key="item.id" :class="{ selected: isSelectedImport(item.id) }">
              <td>{{ item.id }}</td>
              <td>{{ item.source_type }}</td>
              <td>{{ item.status }}</td>
              <td>{{ formatDateTime(item.created_at) }}</td>
              <td>{{ formatDateTime(item.updated_at) }}</td>
              <td>
                <button
                  type="button"
                  :disabled="uploading || (detailLoading && selectedImportId === item.id)"
                  @click="handleSelectImport(item.id)"
                >
                  {{ detailLoading && selectedImportId === item.id ? '加载中...' : '查看详情' }}
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-if="total > 0" class="pagination">
        <button type="button" :disabled="!hasPrevPage || listLoading || uploading" @click="handlePreviousPage">
          上一页
        </button>
        <button type="button" :disabled="!hasNextPage || listLoading || uploading" @click="handleNextPage">
          下一页
        </button>
      </div>
    </section>

    <section class="panel">
      <h2>导入详情</h2>

      <p v-if="detailLoading" class="hint">详情加载中...</p>
      <p v-else-if="detailError" class="error">{{ detailError }}</p>
      <p v-else-if="!selectedImportId || !detail" class="hint">请选择导入记录。</p>

      <div v-else>
        <dl class="detail-grid">
          <div>
            <dt>导入 ID</dt>
            <dd>{{ detail.id }}</dd>
          </div>
          <div>
            <dt>来源类型</dt>
            <dd>{{ detail.source_type }}</dd>
          </div>
          <div>
            <dt>状态</dt>
            <dd>{{ detail.status }}</dd>
          </div>
          <div>
            <dt>创建时间</dt>
            <dd>{{ formatDateTime(detail.created_at) }}</dd>
          </div>
          <div>
            <dt>更新时间</dt>
            <dd>{{ formatDateTime(detail.updated_at) }}</dd>
          </div>
        </dl>

        <h3>关联文件</h3>
        <p v-if="detail.files.length === 0" class="hint">当前导入记录没有关联文件。</p>

        <div v-else class="table-wrapper">
          <table class="table">
            <thead>
              <tr>
                <th>原始文件名</th>
                <th>存储文件名</th>
                <th>相对存储路径</th>
                <th>文件大小</th>
                <th>MIME 类型</th>
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
      </div>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  fetchImportDetail,
  fetchImportList,
  uploadImportFile,
  type ImportDetailResponse,
  type ImportListItem,
} from '../api/imports'

const pageSize = ref(20)
const maxUploadFileBytes = 10 * 1024 * 1024
const allowedUploadExtensions = ['csv', 'xls', 'xlsx']

type LoadResult = 'success' | 'error' | 'stale'

const items = ref<ImportListItem[]>([])
const total = ref(0)
const page = ref(1)
const statusFilter = ref('')
const appliedStatusFilter = ref('')
const listLoading = ref(false)
const listError = ref('')
const listRequestId = ref(0)
const selectedImportId = ref('')
const detail = ref<ImportDetailResponse | null>(null)
const detailLoading = ref(false)
const detailError = ref('')
const detailRequestId = ref(0)
const selectedFile = ref<File | null>(null)
const uploading = ref(false)
const uploadError = ref('')
const uploadSuccessMessage = ref('')
const fileInput = ref<HTMLInputElement | null>(null)

const totalPages = computed(() => (total.value === 0 ? 0 : Math.ceil(total.value / pageSize.value)))
const hasPrevPage = computed(() => page.value > 1)
const hasNextPage = computed(() => totalPages.value > 0 && page.value < totalPages.value)
const hasAppliedStatusFilter = computed(() => appliedStatusFilter.value !== '')
const listSummaryText = computed(() =>
  hasAppliedStatusFilter.value ? '当前筛选条件下暂无导入记录' : '暂无导入记录',
)
const listEmptyMessage = computed(() =>
  hasAppliedStatusFilter.value ? '当前筛选条件下没有导入记录。' : '当前没有导入记录。',
)

function formatDateTime(value?: string) {
  if (!value) {
    return '-'
  }

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }

  return date.toLocaleString('zh-CN', { hour12: false })
}

function formatFileSize(value?: number) {
  if (value == null || Number.isNaN(value)) {
    return '-'
  }

  if (value < 1024) {
    return `${value} B`
  }

  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`
  }

  return `${(value / 1024 / 1024).toFixed(2)} MB`
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

    const listResult = await loadList({
      page: 1,
      status: statusFilter.value,
      preferredImportId: response.import_id,
      skipDetail: true,
    })

    if (listResult === 'success') {
      if (items.value.some((item) => item.id === response.import_id)) {
        await loadDetail(response.import_id)
      } else {
        uploadSuccessMessage.value = `上传成功：${response.file.original_filename}。列表尚未定位到新记录，请手动刷新重试。`
        await loadDetail(response.import_id)
      }
      return
    }

    uploadSuccessMessage.value = `上传成功：${response.file.original_filename}。列表刷新失败，请手动刷新后查看最新记录。`
    await loadDetail(response.import_id)
  } catch (error) {
    uploadError.value = error instanceof Error ? error.message : '上传失败'
  } finally {
    uploading.value = false
  }
}

async function loadDetail(importId: string): Promise<LoadResult> {
  selectedImportId.value = importId

  const requestId = ++detailRequestId.value
  detailLoading.value = true
  detailError.value = ''

  try {
    const response = await fetchImportDetail(importId)

    if (requestId !== detailRequestId.value) {
      return 'stale'
    }

    detail.value = response
    return 'success'
  } catch (error) {
    if (requestId !== detailRequestId.value) {
      return 'stale'
    }

    detailError.value = error instanceof Error ? error.message : '详情加载失败'
    return 'error'
  } finally {
    if (requestId === detailRequestId.value) {
      detailLoading.value = false
    }
  }
}

async function loadList(options: {
  page?: number
  status?: string
  preferredImportId?: string
  skipDetail?: boolean
} = {}): Promise<LoadResult> {
  const targetPage = options.page ?? page.value
  const targetStatus = options.status ?? statusFilter.value
  const requestId = ++listRequestId.value

  listLoading.value = true
  listError.value = ''

  try {
    const response = await fetchImportList({
      page: targetPage,
      pageSize: pageSize.value,
      status: targetStatus || undefined,
    })

    pageSize.value = response.page_size

    if (requestId !== listRequestId.value) {
      return 'stale'
    }

    items.value = response.items
    total.value = response.total
    page.value = response.page
    appliedStatusFilter.value = targetStatus

    if (response.items.length === 0) {
      detailRequestId.value += 1
      selectedImportId.value = ''
      detail.value = null
      detailError.value = ''
      detailLoading.value = false
      return 'success'
    }

    const preferredImportId = options.preferredImportId ?? selectedImportId.value
    const nextSelectedImportId =
      preferredImportId && response.items.some((item) => item.id === preferredImportId)
        ? preferredImportId
        : response.items[0].id

    detailRequestId.value += 1
    selectedImportId.value = nextSelectedImportId

    if (options.skipDetail) {
      detailError.value = ''
      detailLoading.value = false
      return 'success'
    }

    return await loadDetail(nextSelectedImportId)
  } catch (error) {
    if (requestId !== listRequestId.value) {
      return 'stale'
    }

    listError.value = error instanceof Error ? error.message : '列表加载失败'
    return 'error'
  } finally {
    if (requestId === listRequestId.value) {
      listLoading.value = false
    }
  }
}

async function refreshList() {
  await loadList({
    page: page.value,
    status: statusFilter.value,
    preferredImportId: selectedImportId.value || undefined,
  })
}

async function handleStatusChange() {
  const result = await loadList({
    page: 1,
    status: statusFilter.value,
  })

  if (result === 'error') {
    statusFilter.value = appliedStatusFilter.value
  }
}

async function handlePreviousPage() {
  if (!hasPrevPage.value) {
    return
  }

  await loadList({
    page: page.value - 1,
    status: statusFilter.value,
  })
}

async function handleNextPage() {
  if (!hasNextPage.value) {
    return
  }

  await loadList({
    page: page.value + 1,
    status: statusFilter.value,
  })
}

async function handleSelectImport(importId: string) {
  await loadDetail(importId)
}

function isSelectedImport(importId: string) {
  return selectedImportId.value === importId
}

onMounted(() => {
  void loadList({
    page: page.value,
    status: statusFilter.value,
  })
})
</script>

<style scoped>
.page {
  padding: 24px;
  font-family: Arial, sans-serif;
  color: #1f2937;
}

.panel {
  margin-top: 16px;
  padding: 16px;
  border: 1px solid #dcdfe6;
  border-radius: 8px;
  background: #fff;
}

.upload-row,
.toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.table-wrapper {
  overflow: auto;
}

.table {
  width: 100%;
  border-collapse: collapse;
  font-size: 14px;
}

.table th,
.table td {
  padding: 10px 8px;
  border-bottom: 1px solid #ebeef5;
  text-align: left;
  vertical-align: top;
  word-break: break-all;
}

.selected td {
  background: #f8fafc;
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
  margin: 0;
}

.detail-grid div {
  padding: 12px;
  border: 1px solid #ebeef5;
  border-radius: 8px;
  background: #f8fafc;
}

.detail-grid dt {
  color: #666;
  font-weight: 600;
}

.detail-grid dd {
  margin: 8px 0 0;
}

.hint {
  color: #666;
}

.error {
  color: #c62828;
}

button,
select,
input[type='file'] {
  font: inherit;
}

button {
  padding: 6px 12px;
  cursor: pointer;
}
</style>

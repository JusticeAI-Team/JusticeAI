<template>
  <main class="page">
    <h1>导入中心</h1>
    <p class="hint">上传 → 列表 → 详情 最小闭环联调页。</p>

    <section class="panel">
      <h2>上传区</h2>
      <div class="upload-row">
        <input type="file" accept=".csv,.xls,.xlsx" />
        <button type="button">上传文件</button>
      </div>
      <p class="hint">仅支持 csv、xls、xlsx，且文件不能超过 10 MB。</p>
    </section>

    <section class="panel">
      <h2>筛选与刷新</h2>
      <div class="toolbar">
        <label>
          状态：
          <select>
            <option>全部</option>
            <option>uploaded</option>
          </select>
        </label>

        <button type="button">刷新列表</button>
      </div>
    </section>

    <section class="panel">
      <div class="section-header">
        <h2>导入列表</h2>
        <span v-if="total > 0" class="hint">第 {{ page }} 页 · 共 {{ total }} 条</span>
        <span v-else class="hint">暂无导入记录</span>
      </div>

      <p v-if="listLoading" class="hint">列表加载中...</p>
      <p v-else-if="listError" class="error">{{ listError }}</p>
      <p v-else-if="items.length === 0" class="hint">当前没有导入记录。</p>

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
            <tr v-for="item in items" :key="item.id">
              <td>{{ item.id }}</td>
              <td>{{ item.source_type }}</td>
              <td>{{ item.status }}</td>
              <td>{{ formatDateTime(item.created_at) }}</td>
              <td>{{ formatDateTime(item.updated_at) }}</td>
              <td>
                <button type="button" disabled>查看详情</button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <section class="panel">
      <h2>导入详情</h2>
      <p class="hint">请选择导入记录。</p>
    </section>
  </main>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { fetchImportList, type ImportListItem } from '../api/imports'

const pageSize = ref(20)
const items = ref<ImportListItem[]>([])
const total = ref(0)
const page = ref(1)
const listLoading = ref(false)
const listError = ref('')

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

async function loadList() {
  listLoading.value = true
  listError.value = ''

  try {
    const response = await fetchImportList({
      page: page.value,
      pageSize: pageSize.value,
    })

    items.value = response.items
    total.value = response.total
    page.value = response.page
    pageSize.value = response.page_size
  } catch (error) {
    listError.value = error instanceof Error ? error.message : '列表加载失败'
  } finally {
    listLoading.value = false
  }
}

onMounted(() => {
  void loadList()
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

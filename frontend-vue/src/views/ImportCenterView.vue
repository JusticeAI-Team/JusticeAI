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
        <span class="hint">后续接入分页统计</span>
      </div>

      <div class="table-wrapper">
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
            <tr>
              <td colspan="6" class="hint">下一轮将在这里接入导入列表数据。</td>
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

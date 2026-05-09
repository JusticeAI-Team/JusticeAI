<template>
  <div class="data-integration-hud">
    <!-- 顶部状态栏 -->
    <header class="hud-top-bar">
      <div class="bar-left">
        <span class="blinking-cursor">_</span>
        <span class="sys-title">政务异构数据 AI 清洗与图谱构建流水线 (ETL PIPELINE)</span>
      </div>
      <div class="bar-right">
        <span class="status-dot"></span>
        <span class="status-text">系统全链路状态: {{ apiError ? '需人工关注' : '运转中' }}</span>
      </div>
    </header>

    <section class="live-summary">
      <div class="summary-card" v-for="item in summaryCards" :key="item.key">
        <div class="summary-label">{{ item.label }}</div>
        <div class="summary-value">{{ item.value }}</div>
        <div :class="['summary-status', item.status]">{{ item.note }}</div>
      </div>
      <button class="ops-button" @click="loadPipelineStats" :disabled="isRefreshing">
        {{ isRefreshing ? '刷新中...' : '刷新后端统计' }}
      </button>
    </section>

    <div v-if="apiError" class="api-error">
      后端提示：{{ apiError }}
    </div>

    <main class="pipeline-board">
      
      <!-- ================= 节点 1：原始数据接入 ================= -->
      <section class="flow-step">
        <div class="step-header">
          <div class="step-num">01</div>
          <div class="step-info">
            <h3 class="step-title">异构原始数据接入</h3>
            <p class="step-sub">将 12345/110 等非结构化文件汇入</p>
          </div>
        </div>
        
        <div class="step-content">
          <div
            :class="['upload-dropzone', { 'is-processing': isProcessing }]"
            @click="triggerFileInput"
            @dragover.prevent
            @drop.prevent="handleFileDrop"
          >
            <div class="scan-beam" v-if="isProcessing"></div>
            <i class="el-icon-upload cloud-icon"></i>
            <p class="u-main">{{ isProcessing ? '后端处理链路执行中...' : '拖拽通州多分页 Excel / CSV 至此' }}</p>
            <p class="u-sub">支持 .xlsx / .xls / .csv；截图页将以 OCR 待处理合同入库</p>
            <input type="file" ref="fileInput" class="hidden-input" accept=".xlsx,.xls,.csv" @change="handleFileSelect" />
          </div>

          <div class="file-queue">
            <div class="queue-title">接入解析队列</div>
            <div class="file-item" v-for="(f, i) in fileList" :key="i">
              <i class="el-icon-document"></i>
              <div class="f-info">
                <div class="f-name">{{ f.name }}</div>
                <div class="f-status" :style="{ color: f.color }">{{ f.status }}</div>
              </div>
            </div>
            <div v-if="latestCases.length" class="case-action-panel">
              <div class="queue-title">最新批次案件</div>
              <div class="case-action-head">
                <div>
                  <strong>{{ latestCases.length }} 条标准案件</strong>
                  <p>批次 {{ latestImportShort }} 已入库，可继续触发真实抽取、HugeGraph 同步和 Milvus 向量回写。</p>
                </div>
                <button class="mini-primary" @click="runExtractionForLatestCases" :disabled="isExtracting">
                  {{ isExtracting ? '抽取同步中...' : '执行抽取同步' }}
                </button>
              </div>
              <div class="case-mini-row" v-for="item in latestCases.slice(0, 4)" :key="item.id">
                <span>{{ item.case_code }}</span>
                <b>{{ item.title }}</b>
                <em>{{ item.graph_sync_status }} / {{ item.vector_sync_status }}</em>
              </div>
              <div class="extract-status">{{ extractionStatus }}</div>
            </div>
          </div>
        </div>
      </section>

      <div class="flow-arrow">
        <div class="arrow-line"></div>
        <div class="arrow-head">></div>
      </div>

      <!-- ================= 节点 2：大模型智能提炼 ================= -->
      <section class="flow-step">
        <div class="step-header">
          <div class="step-num">02</div>
          <div class="step-info">
            <h3 class="step-title">OpenAI-Compatible 智能抽取提炼</h3>
            <p class="step-sub">vLLM / Embedding / HugeGraph / Milvus 真实链路</p>
          </div>
        </div>
        
        <div class="step-content terminal-bg">
          <div class="terminal-header">
            <span class="t-dot red"></span><span class="t-dot yellow"></span><span class="t-dot green"></span>
            <span class="t-title">AI_Entity_Extraction_Log</span>
          </div>
          
          <div class="terminal-body" ref="terminalRef">
            <div class="log-block" v-for="(log, i) in logs" :key="i">
              <div class="log-raw"><span class="label">【读入原文本】</span>"{{ log.raw }}"</div>
              <div class="log-arrow">↓ GLM-5.1 语义肢解...</div>
              <div class="log-json">
                <span class="label">【提取结构化实体】</span><br/>
                <span class="json-code" v-html="log.json"></span>
              </div>
            </div>
            
            <div v-if="isProcessing" class="log-block loading-block">
              后端正在执行：上传落盘 → 多 sheet 解析 → 标准案件生成 → AI 摘要/风险原因 → 图谱/向量同步 <span class="blinking-cursor">_</span>
            </div>
          </div>
        </div>
      </section>

      <div class="flow-arrow">
        <div class="arrow-line"></div>
        <div class="arrow-head">></div>
      </div>

      <!-- ================= 节点 3：图数据库网络构建 ================= -->
      <section class="flow-step">
        <div class="step-header">
          <div class="step-num">03</div>
          <div class="step-info">
            <h3 class="step-title">HugeGraph 动态拓扑结网</h3>
            <p class="step-sub">实时生成节点(Nodes)与关系边(Edges)</p>
          </div>
        </div>
        
        <div class="step-content graph-bg">
          <div class="graph-wrapper" ref="graphRef"></div>
          
          <div class="graph-stats">
            <div class="stat-box">
              <div class="s-name">已入库实体 (Nodes)</div>
              <div class="s-val cyan">{{ animatedNodes.toLocaleString() }}</div>
            </div>
            <div class="stat-box">
              <div class="s-name">已确立连线 (Edges)</div>
              <div class="s-val purple">{{ animatedEdges.toLocaleString() }}</div>
            </div>
          </div>
        </div>
      </section>

    </main>
  </div>
</template>

<script setup>
import { computed, ref, onMounted, onBeforeUnmount, nextTick } from 'vue'
import * as echarts from 'echarts'
import { apiGet, apiPost, apiUploadImport } from '../api/platform'

const isProcessing = ref(false)
const isExtracting = ref(false)
const isRefreshing = ref(false)
const apiError = ref('')
const fileInput = ref(null)
const terminalRef = ref(null)
const graphRef = ref(null)
const latestImportId = ref('')
const latestCases = ref([])
const extractionStatus = ref('等待案件入库')

let myChart = null

const animatedNodes = ref(142589)
const animatedEdges = ref(384102)

const summaryCards = ref([
  { key: 'batch_total', label: '导入批次', value: '--', note: '等待后端', status: 'warn' },
  { key: 'risk_cases', label: '标准案件', value: '--', note: '等待后端', status: 'warn' },
  { key: 'entities', label: '抽取实体', value: '--', note: '等待后端', status: 'warn' },
  { key: 'graph_nodes', label: '图谱节点', value: '--', note: '等待后端', status: 'warn' }
])

const fileList = ref([
  { name: '2026Q2_110警情记录.csv', status: '后台监听中', color: '#666' },
  { name: '上月12345欠薪诉求.xlsx', status: '后台监听中', color: '#666' }
])

const logs = ref([
  {
    raw: "系统例行扫描：通州区工商登记变更记录... 发现宏远劳务高管变更异常。",
    json: `{<br/>  <span style="color:#122E8A">"企业"</span>: "宏远劳务分包有限公司",<br/>  <span style="color:#8B5CF6">"事件"</span>: "高管突击变更",<br/>  <span style="color:#0F7E3B">"状态"</span>: "已入库"<br/>}`
  },
  {
    raw: "12345历史回溯：梨园镇多名群众反映某工地夜间施工扰民及拖欠结款...",
    json: `{<br/>  <span style="color:#122E8A">"关联节点"</span>: "梨园镇项目",<br/>  <span style="color:#D9363E">"风险标签"</span>: "环保/历史纠纷",<br/>  <span style="color:#0F7E3B">"权重升级"</span>: "+15%"<br/>}`
  }
])

const graphNodes = ref([
  { id: '0', name: '华丰建设', category: 0, symbolSize: 35 },
  { id: '1', name: '王大拿(法人)', category: 1, symbolSize: 25 },
  { id: '2', name: '京运理财', category: 0, symbolSize: 30 },
  { id: '3', name: '宏远劳务', category: 0, symbolSize: 20 },
  { id: '4', name: '历史欠薪工单', category: 2, symbolSize: 22 },
  { id: '5', name: '李某某(财务)', category: 1, symbolSize: 18 },
  { id: '6', name: '公对私账户(资金池)', category: 3, symbolSize: 20 },
  { id: '7', name: '梨园镇烂尾项目', category: 0, symbolSize: 25 },
  { id: '8', name: '110纠纷警情', category: 2, symbolSize: 20 }
])

const graphLinks = ref([
  { source: '1', target: '0', name: '控股' },
  { source: '1', target: '2', name: '实控' },
  { source: '3', target: '0', name: '分包商' },
  { source: '4', target: '0', name: '历史指向' },
  { source: '5', target: '0', name: '高管任职' },
  { source: '5', target: '6', name: '频发转账' },
  { source: '2', target: '6', name: '资金回流' },
  { source: '0', target: '7', name: '承接建设' },
  { source: '8', target: '7', name: '事发地' },
  { source: '8', target: '3', name: '涉事方' }
])

const latestImportShort = computed(() => latestImportId.value ? latestImportId.value.slice(0, 8) : '--')

const getMetric = (summary, keyCandidates, fallback = '--') => {
  const metrics = summary?.metrics || summary?.totals || []
  const found = metrics.find((item) => keyCandidates.includes(item.key))
  return found?.value ?? fallback
}

const setSummaryFromBackend = (ingestion, extraction, graph, risk) => {
  summaryCards.value = [
    {
      key: 'batch_total',
      label: '导入批次',
      value: getMetric(ingestion, ['batch_total', 'import_batches']),
      note: 'imports',
      status: 'ok'
    },
    {
      key: 'risk_cases',
      label: '标准案件',
      value: getMetric(risk, ['case_total', 'risk_cases', 'total_cases', 'total']),
      note: 'risk_cases',
      status: 'ok'
    },
    {
      key: 'entities',
      label: '抽取实体',
      value: getMetric(extraction, ['entity_total', 'entities', 'knowledge_entities']),
      note: 'extraction',
      status: 'ok'
    },
    {
      key: 'graph_nodes',
      label: '图谱节点',
      value: getMetric(graph, ['entity_total', 'node_total', 'nodes']),
      note: 'HugeGraph ready',
      status: 'ok'
    }
  ]
}

const loadPipelineStats = async () => {
  isRefreshing.value = true
  try {
    const [ingestion, extraction, graph, risk] = await Promise.all([
      apiGet('/ingestion/summary'),
      apiGet('/extraction/summary'),
      apiGet('/graph/summary'),
      apiGet('/risk/summary')
    ])
    setSummaryFromBackend(ingestion, extraction, graph, risk)
    apiError.value = ''
  } catch (error) {
    apiError.value = error.message
  } finally {
    isRefreshing.value = false
  }
}

const refreshCasesForImport = async (importId) => {
  if (!importId) return []
  const result = await apiGet(`/risk/cases?import_id=${encodeURIComponent(importId)}&page_size=50`)
  latestCases.value = result?.items || []
  renderGraphFromCases(latestCases.value)
  return latestCases.value
}

const escapeHtml = (value) =>
  String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')

const initGraph = () => {
  if (!graphRef.value) return
  myChart = echarts.init(graphRef.value)
  updateGraph()
}

const updateGraph = () => {
  const option = {
    backgroundColor: 'transparent',
    color: ['#122E8A', '#8B5CF6', '#D9363E', '#F5A623'], 
    series: [{
      type: 'graph',
      layout: 'force',
      animation: true,
      data: graphNodes.value,
      links: graphLinks.value,
      categories: [{ name: '企业' }, { name: '自然人' }, { name: '风险线索' }, { name: '资金流' }],
      roam: true, // 允许鼠标缩放和平移
      draggable: true, // 允许鼠标拖拽节点（极其重要，方便手动拨开重叠节点）
      
      // 🚀 优化 1：节点文字样式
      label: { 
        show: true, 
        position: 'right', 
        distance: 8, // 让文字离节点圆圈稍微远一点，防遮挡
        color: '#122E8A', 
        fontSize: 12, 
        fontWeight: '900',
        textBorderColor: '#FFFFFF', 
        textBorderWidth: 3
      },
      
      // 🚀 优化 2：核心！加大排斥力和边长，彻底散开图谱
      force: { 
        repulsion: 1500,       // 极大增加节点之间的排斥力 (原先是 350)
        edgeLength: [120, 200], // 拉长连线的长度 (原先是 60-110)
        gravity: 0.1           // 降低向中心靠拢的引力，让它们往外扩
      }, 
      
      // 🚀 优化 3：连线标签稍微做小一点，减少碰撞体积
      edgeLabel: {
        show: true, 
        fontSize: 10,  // 稍微改小一号
        fontWeight: 'bold',
        color: '#122E8A', 
        backgroundColor: '#FFFFFF', 
        borderColor: '#122E8A',     
        borderWidth: 1,
        padding: [2, 4], // 缩小上下左右的内边距
        borderRadius: 4,
        formatter: (params) => params.data.name
      },
      
      // 🚀 优化 4：增加连线弧度，防止双向关系线重合
      lineStyle: { 
        color: '#8C98B0', 
        curveness: 0.3, // 弧度改大，让线条更弯，避开中间的文字
        width: 1.5 
      },
      emphasis: {
        focus: 'adjacency',
        lineStyle: { width: 3, color: '#122E8A' }
      }
    }]
  }
  myChart.setOption(option)
}

const renderGraphFromCases = (cases) => {
  if (!cases.length) return
  const nextNodes = []
  const nextLinks = []
  cases.slice(0, 10).forEach((item, index) => {
    const caseNodeId = `case-${item.id}`
    const areaNodeId = `area-${item.area_name || 'unknown'}`
    const riskNodeId = `risk-${item.risk_level || 'unknown'}`
    nextNodes.push({
      id: caseNodeId,
      name: item.title || item.case_code,
      category: 2,
      symbolSize: item.risk_level === 'high' ? 30 : 22
    })
    if (!nextNodes.some((node) => node.id === areaNodeId)) {
      nextNodes.push({ id: areaNodeId, name: item.area_name || '未分配地区', category: 0, symbolSize: 24 })
    }
    if (!nextNodes.some((node) => node.id === riskNodeId)) {
      nextNodes.push({ id: riskNodeId, name: `${item.risk_level || 'unknown'} 风险`, category: 3, symbolSize: 20 })
    }
    nextLinks.push({ source: caseNodeId, target: areaNodeId, name: '属地' })
    nextLinks.push({ source: caseNodeId, target: riskNodeId, name: '风险等级' })
    if (index > 0) {
      nextLinks.push({ source: caseNodeId, target: `case-${cases[index - 1].id}`, name: '同批次' })
    }
  })
  graphNodes.value = nextNodes
  graphLinks.value = nextLinks
  animatedNodes.value = nextNodes.length
  animatedEdges.value = nextLinks.length
  updateGraph()
}

const triggerFileInput = () => { fileInput.value.click() }

const scrollToBottom = () => {
  nextTick(() => {
    if (terminalRef.value) terminalRef.value.scrollTop = terminalRef.value.scrollHeight
  })
}

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

const isFinalJobStatus = (status) => ['completed', 'completed_with_warnings', 'failed', 'cancelled'].includes(String(status || '').toLowerCase())

const waitForJob = async (jobId, onTick, timeoutMs = 15 * 60 * 1000) => {
  const startedAt = Date.now()
  let lastMessage = ''

  while (Date.now() - startedAt < timeoutMs) {
    const job = await apiGet(`/jobs/${jobId}`)
    const tickMessage = `${job.status} · ${job.progress_percent}% · ${job.message || 'running'}`
    if (tickMessage !== lastMessage) {
      lastMessage = tickMessage
      onTick?.(job)
    }
    if (isFinalJobStatus(job.status)) {
      return job
    }
    await delay(2000)
  }

  throw new Error(`后台任务 ${jobId} 超过等待时间，请到技术运维后台查看任务状态。`)
}

const handleFileSelect = (e) => {
  if (e.target.files.length > 0) {
    startPipeline(e.target.files[0])
    e.target.value = ''
  }
}

const handleFileDrop = (e) => {
  const file = e.dataTransfer?.files?.[0]
  if (file) startPipeline(file)
}

const startPipeline = async (file) => {
  if (isProcessing.value) return
  isProcessing.value = true
  apiError.value = ''
  const queueItem = { name: file.name, status: '上传中...', color: '#F5A623' }
  fileList.value.unshift(queueItem)

  try {
    const uploaded = await apiUploadImport(file)
    latestImportId.value = uploaded.import_id
    latestCases.value = []
    extractionStatus.value = '等待案件生成'
    queueItem.status = `已上传 ${uploaded.import_id.slice(0, 8)}，处理中...`
    queueItem.importId = uploaded.import_id

    logs.value.push({
      raw: `上传完成：${file.name}，后端批次 ${uploaded.import_id}`,
      json: `{<br/>  <span style="color:#122E8A">"导入状态"</span>: "uploaded",<br/>  <span style="color:#0F7E3B">"文件"</span>: "${escapeHtml(uploaded.file.original_filename)}"<br/>}`
    })
    scrollToBottom()

    const processJob = await apiPost(`/ingestion/${uploaded.import_id}/process/async`)
    queueItem.status = `处理任务已排队 ${processJob.id.slice(0, 8)}`
    logs.value.push({
      raw: `后台任务已创建：${processJob.id}`,
      json: `{<br/>  <span style="color:#122E8A">"job_type"</span>: "${escapeHtml(processJob.job_type)}",<br/>  <span style="color:#F5A623">"status"</span>: "${escapeHtml(processJob.status)}"<br/>}`
    })
    scrollToBottom()

    const completedProcessJob = await waitForJob(processJob.id, (job) => {
      queueItem.status = `处理进度 ${job.progress_percent}%：${job.message}`
    })
    if (completedProcessJob.status === 'failed') {
      throw new Error(completedProcessJob.error_message || completedProcessJob.message || '导入处理任务失败')
    }

    const processed = completedProcessJob.result || {}
    queueItem.status = processed.status === 'processed' ? '入库完毕，待抽取同步' : completedProcessJob.status
    queueItem.color = completedProcessJob.status === 'completed' ? '#0F7E3B' : '#D9363E'

    logs.value.push({
      raw: `后端处理结果：${completedProcessJob.message}`,
      json: `{<br/>  <span style="color:#122E8A">"状态"</span>: "${escapeHtml(processed.status || completedProcessJob.status)}",<br/>  <span style="color:#8B5CF6">"批次"</span>: "${escapeHtml(uploaded.import_id)}",<br/>  <span style="color:#0F7E3B">"外部同步"</span>: "案件生成后由抽取链路写回 graph/vector 状态"<br/>}`
    })

    const cases = await refreshCasesForImport(uploaded.import_id)
    extractionStatus.value = cases.length
      ? `已生成 ${cases.length} 条案件；点击“执行抽取同步”进入实体抽取、HugeGraph、Milvus 回写。`
      : '未查询到本批次案件，请检查处理日志。'
    logs.value.push({
      raw: `标准案件查询：本批次返回 ${cases.length} 条风险案件`,
      json: `{<br/>  <span style="color:#122E8A">"case_count"</span>: ${cases.length},<br/>  <span style="color:#0F7E3B">"next_action"</span>: "extraction_run"<br/>}`
    })
    await loadPipelineStats()
  } catch (error) {
    queueItem.status = '处理失败'
    queueItem.color = '#D9363E'
    apiError.value = error.message
    logs.value.push({
      raw: `处理失败：${error.message}`,
      json: `{<br/>  <span style="color:#D9363E">"error"</span>: "${escapeHtml(error.message)}"<br/>}`
    })
  } finally {
    isProcessing.value = false
    scrollToBottom()
  }
}

const runExtractionForLatestCases = async () => {
  if (isExtracting.value || !latestCases.value.length) return
  isExtracting.value = true
  apiError.value = ''
  extractionStatus.value = '后端正在执行抽取 → HugeGraph 同步 → Milvus 向量回写；该步骤会真实调用模型和外部服务。'
  try {
    const caseIds = latestCases.value.map((item) => item.id)
    const extractionJob = await apiPost('/extraction/run/async', {
      case_ids: caseIds,
      mode: 'incremental'
    })
    extractionStatus.value = `抽取任务已排队：${extractionJob.id}`
    const completedExtractionJob = await waitForJob(extractionJob.id, (job) => {
      extractionStatus.value = `抽取同步进度 ${job.progress_percent}%：${job.message}`
    })
    if (completedExtractionJob.status === 'failed') {
      throw new Error(completedExtractionJob.error_message || completedExtractionJob.message || '抽取同步任务失败')
    }
    const result = completedExtractionJob.result || {}
    extractionStatus.value = `${completedExtractionJob.status}: ${completedExtractionJob.message}`
    logs.value.push({
      raw: `抽取同步完成：${completedExtractionJob.message}`,
      json: `{<br/>  <span style="color:#122E8A">"job_id"</span>: "${escapeHtml(completedExtractionJob.id)}",<br/>  <span style="color:#8B5CF6">"extraction_run_id"</span>: "${escapeHtml(result.run_id || completedExtractionJob.target_id || '')}",<br/>  <span style="color:#0F7E3B">"status"</span>: "${escapeHtml(completedExtractionJob.status)}"<br/>}`
    })
    await refreshCasesForImport(latestImportId.value)
    await loadPipelineStats()
  } catch (error) {
    apiError.value = error.message
    extractionStatus.value = `抽取同步失败：${error.message}`
    logs.value.push({
      raw: `抽取同步失败：${error.message}`,
      json: `{<br/>  <span style="color:#D9363E">"error"</span>: "${escapeHtml(error.message)}"<br/>}`
    })
  } finally {
    isExtracting.value = false
    scrollToBottom()
  }
}

onMounted(() => {
  initGraph()
  scrollToBottom()
  loadPipelineStats()
  window.addEventListener('resize', () => myChart && myChart.resize())
})

onBeforeUnmount(() => {
  if (myChart) myChart.dispose()
  window.removeEventListener('resize', () => myChart && myChart.resize())
})
</script>

<style scoped>
.data-integration-hud {
  display: flex; flex-direction: column; width: 100%; height: 94vh;
  background-color: #F5EFEA; color: #333; overflow: hidden;
  font-family: 'PingFang SC', sans-serif;
}

.hud-top-bar { height: 60px; background: #FFFFFF; border-bottom: 1px solid rgba(18, 46, 138, 0.1); display: flex; justify-content: space-between; align-items: center; padding: 0 30px; box-shadow: 0 2px 10px rgba(0,0,0,0.02); }
.bar-left { font-size: 16px; font-weight: bold; color: #122E8A; display: flex; align-items: center; gap: 10px;}
.blinking-cursor { animation: blink 1s step-end infinite; }
.bar-right { display: flex; align-items: center; gap: 8px; font-size: 13px; color: #666; font-weight: bold; }
.status-dot { width: 8px; height: 8px; background: #52C41A; border-radius: 50%; }

.live-summary { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)) 140px; gap: 12px; padding: 14px 30px 0; align-items: stretch; }
.summary-card { background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.12); border-radius: 8px; padding: 12px 14px; box-shadow: 0 4px 12px rgba(18, 46, 138, 0.04); }
.summary-label { font-size: 12px; color: #666; font-weight: bold; }
.summary-value { margin-top: 5px; font-size: 23px; line-height: 1; font-family: 'JetBrains Mono', Consolas, monospace; font-weight: 900; color: #122E8A; }
.summary-status { margin-top: 7px; font-size: 11px; font-weight: bold; }
.summary-status.ok { color: #0F7E3B; }
.summary-status.warn { color: #F5A623; }
.summary-status.bad { color: #D9363E; }
.ops-button { border: 1px solid #122E8A; background: #122E8A; color: #FFFFFF; border-radius: 8px; font-weight: 900; cursor: pointer; box-shadow: 0 4px 12px rgba(18, 46, 138, 0.12); }
.ops-button:disabled { opacity: 0.6; cursor: not-allowed; }
.api-error { margin: 10px 30px 0; padding: 10px 14px; background: rgba(217, 54, 62, 0.08); border: 1px solid rgba(217, 54, 62, 0.2); color: #D9363E; border-radius: 6px; font-size: 12px; font-weight: bold; }

.pipeline-board { flex: 1; display: flex; align-items: stretch; padding: 18px 30px 30px; gap: 15px; overflow: hidden; }

.flow-arrow { display: flex; align-items: center; justify-content: center; position: relative; width: 30px; }
.arrow-line { position: absolute; width: 100%; height: 2px; background: rgba(18, 46, 138, 0.2); }
.arrow-head { color: #122E8A; font-weight: bold; z-index: 2; font-family: monospace; font-size: 20px; }

.flow-step { flex: 1; background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.15); border-radius: 8px; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 4px 15px rgba(18, 46, 138, 0.05); }
.step-header { display: flex; gap: 15px; padding: 15px 20px; background: rgba(18, 46, 138, 0.03); border-bottom: 1px solid rgba(18, 46, 138, 0.08); align-items: center; }
.step-num { font-size: 28px; font-weight: 900; color: rgba(18, 46, 138, 0.2); font-family: 'JetBrains Mono', monospace; font-style: italic; line-height: 1; }
.step-title { margin: 0 0 4px; font-size: 16px; color: #122E8A; font-weight: bold; letter-spacing: 1px; }
.step-sub { margin: 0; font-size: 12px; color: #666; }
.step-content { flex: 1; padding: 20px; display: flex; flex-direction: column; gap: 20px; position: relative; overflow-y: auto;}

.upload-dropzone { height: 140px; border: 1px dashed rgba(18, 46, 138, 0.4); border-radius: 6px; background: #F9F9F9; display: flex; flex-direction: column; align-items: center; justify-content: center; cursor: pointer; position: relative; overflow: hidden; transition: 0.2s;}
.upload-dropzone:hover { background: rgba(18, 46, 138, 0.02); border-color: #122E8A; }
.cloud-icon { font-size: 36px; color: #122E8A; margin-bottom: 10px; }
.u-main { margin: 0 0 5px; font-size: 14px; font-weight: bold; color: #333; }
.u-sub { margin: 0; font-size: 12px; color: #666; }
.hidden-input { display: none; }
.scan-beam { position: absolute; inset: 0; background: linear-gradient(to bottom, transparent, rgba(18, 46, 138, 0.1) 50%, transparent); animation: scan 1.5s linear infinite; }

.file-queue { flex: 1; background: #F5EFEA; padding: 15px; border-radius: 6px; border: 1px solid rgba(18, 46, 138, 0.1); }
.queue-title { font-size: 12px; color: #122E8A; margin-bottom: 10px; border-left: 3px solid #122E8A; padding-left: 8px; font-weight: bold; }
.file-item { display: flex; align-items: center; gap: 10px; padding: 10px; border-bottom: 1px solid rgba(18, 46, 138, 0.05); font-size: 13px; }
.f-info { display: flex; justify-content: space-between; flex: 1; }
.f-name { color: #333; font-weight: 500; }
.case-action-panel { margin-top: 14px; padding-top: 12px; border-top: 1px dashed rgba(18, 46, 138, 0.18); }
.case-action-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 10px; }
.case-action-head strong { color: #122E8A; font-size: 14px; }
.case-action-head p { margin: 4px 0 0; color: #666; font-size: 12px; line-height: 1.5; }
.mini-primary { border: 1px solid #122E8A; background: #122E8A; color: #FFFFFF; border-radius: 6px; padding: 8px 10px; font-size: 12px; font-weight: 900; cursor: pointer; white-space: nowrap; }
.mini-primary:disabled { opacity: 0.62; cursor: not-allowed; }
.case-mini-row { display: grid; grid-template-columns: 100px 1fr 130px; gap: 8px; align-items: center; padding: 8px 0; border-bottom: 1px solid rgba(18, 46, 138, 0.06); font-size: 12px; }
.case-mini-row span { color: #122E8A; font-family: 'JetBrains Mono', Consolas, monospace; font-weight: 900; }
.case-mini-row b { color: #333; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.case-mini-row em { color: #666; font-style: normal; font-family: 'JetBrains Mono', Consolas, monospace; text-align: right; }
.extract-status { margin-top: 10px; color: #0F7E3B; font-size: 12px; line-height: 1.6; font-weight: bold; }

.terminal-bg { padding: 0; background: #FAFAFA; }
.terminal-header { background: #EBEBEB; padding: 8px 15px; display: flex; align-items: center; border-bottom: 1px solid #DDD; }
.t-dot { width: 10px; height: 10px; border-radius: 50%; margin-right: 6px; }
.t-dot.red { background: #FF5F56; } .t-dot.yellow { background: #FFBD2E; } .t-dot.green { background: #27C93F; }
.t-title { font-family: 'JetBrains Mono', monospace; font-size: 12px; color: #333; margin-left: 10px; font-weight: bold; }
.terminal-body { flex: 1; padding: 15px; overflow-y: auto; font-family: 'PingFang SC'; }
.log-block { margin-bottom: 25px; border-left: 3px solid rgba(18, 46, 138, 0.3); padding-left: 15px; }
.log-raw { font-size: 13px; color: #333; line-height: 1.6; margin-bottom: 8px; font-weight: bold; }
.label { color: #122E8A; font-size: 12px; font-weight: bold; }
.log-arrow { color: #F5A623; font-size: 12px; margin-bottom: 8px; font-family: 'JetBrains Mono', monospace; font-weight: bold; }
.log-json { background: #FFFFFF; padding: 12px; border-radius: 6px; border: 1px solid rgba(18, 46, 138, 0.1); box-shadow: 0 2px 8px rgba(0,0,0,0.02); }
.json-code { font-family: 'JetBrains Mono', Consolas, monospace; font-size: 13px; color: #333; line-height: 1.6; font-weight: bold; }
.loading-block { border-left-color: transparent; color: #122E8A; font-size: 13px; font-weight: bold; }

.graph-bg { display: flex; flex-direction: column; gap: 15px; }
.graph-wrapper { flex: 1; min-height: 200px; background: #FAFAFA; border-radius: 6px; border: 1px solid rgba(18, 46, 138, 0.1); }
.graph-stats { display: flex; justify-content: space-between; gap: 10px; }
.stat-box { flex: 1; background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.1); padding: 12px 15px; display: flex; justify-content: space-between; align-items: center; border-radius: 6px; box-shadow: 0 2px 6px rgba(0,0,0,0.02); }
.s-name { font-size: 12px; color: #666; font-weight: bold; }
.s-val { font-size: 20px; font-weight: bold; font-family: 'JetBrains Mono', monospace; }
.s-val.cyan { color: #122E8A; }
.s-val.purple { color: #8B5CF6; }

::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: rgba(18, 46, 138, 0.2); border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: rgba(18, 46, 138, 0.4); }

@keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0; } }
@keyframes scan { 0% { transform: translateY(-100%); } 100% { transform: translateY(140px); } }
</style>

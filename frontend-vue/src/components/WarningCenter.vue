<template>
  <div class="warning-center-hud">
    <!-- 1. 顶部指令过滤栏 (重构为现代政务表单风格) -->
    <header class="terminal-header">
      <div class="header-left">
        <span class="prompt-symbol">></span>
        <span class="blinking-cursor">_</span>
        <span class="header-title">系统指令: SELECT * FROM 风险预警池 WHERE 状态 = 待核准</span>
      </div>
      <div class="header-right">
        <div class="filter-group">
          <label>风险级别:</label>
          <select v-model="filters.level" class="hud-select">
            <option value="">ALL (全部)</option>
            <option value="P1">P1 - 极高危</option>
            <option value="P2">P2 - 高危</option>
          </select>
        </div>
        <div class="filter-group">
          <label>主体检索:</label>
          <input 
            v-model="filters.subject" 
            class="hud-input" 
            placeholder="输入企业名称或信用代码..." 
          />
        </div>
        <button class="hud-btn primary" :disabled="isLoading" @click="handleSearch">
          {{ isLoading ? '[ 同步中... ]' : '[ 执行检索 ]' }}
        </button>
      </div>
    </header>

    <!-- 2. 主体分栏内容区 -->
    <main class="split-layout">
      <!-- 左侧：高密度线索网格 -->
      <section class="grid-panel">
        <div class="panel-corner top-left"></div>
        <div class="panel-corner bottom-right"></div>
        
        <div class="table-scroller">
          <table class="bloomberg-table">
            <thead>
              <tr>
                <th width="140">发现时间 (UTC+8)</th>
                <th width="60">级别</th>
                <th>高危线索主体</th>
                <th width="100">涉险领域</th>
                <th width="140">双域印证状态</th>
                <th width="120">AI 风险置信度</th>
                <th width="80">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="!filteredWarnings.length">
                <td colspan="7" class="empty-table-cell">
                  {{ apiError || '暂无符合条件的预警线索，后端 /alerts 当前没有返回待处理记录。' }}
                </td>
              </tr>
              <tr 
                v-for="row in filteredWarnings" 
                :key="row.id"
                :class="['data-row', { 'selected-row': currentWarning?.id === row.id }]"
                @click="selectRow(row)"
              >
                <td class="mono-text dim">{{ row.date }}</td>
                <td>
                  <span :class="['level-badge', row.level]">{{ row.level }}</span>
                </td>
                <td class="subject-cell">{{ row.subject }}</td>
                <td class="industry-cell">{{ row.industry }}</td>
                <td>
                  <div class="dual-domain-tags">
                    <span :class="['d-tag', row.has12345 ? 'active-12345' : '']">12345</span>
                    <span :class="['d-tag', row.has110 ? 'active-110' : '']">110</span>
                  </div>
                </td>
                <td>
                  <div class="confidence-bar-wrapper">
                    <div 
                      class="c-bar" 
                      :style="{ width: row.confidence + '%', backgroundColor: row.confidence >= 85 ? '#D9363E' : '#122E8A' }"
                    ></div>
                    <span class="c-text mono-text">{{ row.confidence }}%</span>
                  </div>
                </td>
                <td>
                  <button class="action-btn">审查 ></button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <!-- 右侧：AI 穿透核查报告 -->
      <section class="inspector-panel">
        <div class="panel-corner top-right"></div>
        <div class="panel-corner bottom-left"></div>

        <template v-if="currentWarning">
          <div class="inspector-header">
            <span class="record-id mono-text">ID: {{ currentWarning.creditCode }}</span>
            <span class="status-tag pulse-warn">{{ statusLabel(currentWarning.status) }}</span>
          </div>

          <h2 class="target-name">{{ currentWarning.subject }}</h2>
          
          <div class="info-grid">
            <div class="info-item">
              <span class="i-label">法定代表人:</span>
            <span class="i-val">{{ currentWarning.legalPerson || '待核实' }}</span>
            </div>
            <div class="info-item">
              <span class="i-label">线索类型:</span>
              <span class="i-val highlight-red">{{ currentWarning.clueType }}</span>
            </div>
          </div>

          <div class="ai-report-box">
            <div class="box-title">> OpenAI-Compatible 风险核查摘要</div>
            <div class="report-content">
              <div class="report-line">
                <span class="r-label">【12345 热线】</span>
                <span class="r-val">{{ currentWarning.summary12345 }}</span>
              </div>
              <div class="report-line">
                <span class="r-label">【110 警情】</span>
                <span class="r-val">{{ currentWarning.summary110 }}</span>
              </div>
              <div class="report-line">
                <span class="r-label">【关联图谱】</span>
                <span class="r-val">{{ currentWarning.graphInfo }}</span>
              </div>
            </div>
          </div>

          <div class="evidence-tags">
            <span v-for="tag in currentWarning.evidenceTags" :key="tag" class="e-tag">{{ tag }}</span>
          </div>

          <div class="action-footer">
            <button class="hud-btn danger block" :disabled="isActing" @click="setAlertStatus('acknowledged')">
              确认预警并推送监管
            </button>
            <button class="hud-btn ghost block" :disabled="isActing" @click="setAlertStatus('ignored')">
              标记为低风险/忽略
            </button>
          </div>
        </template>

        <template v-else>
          <div class="empty-state">
            <div class="scan-radar"></div>
            <p>等待选中侦测目标...</p>
          </div>
        </template>
      </section>
    </main>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { apiGet, apiPost } from '../api/platform'

const filters = ref({ level: '', subject: '' })
const warnings = ref([])
const currentWarning = ref(null)
const isLoading = ref(false)
const isActing = ref(false)
const apiError = ref('')

const severityLevel = (severity) => {
  const normalized = String(severity || '').toLowerCase()
  if (['critical', 'high'].includes(normalized)) return 'P1'
  if (['medium', 'warning'].includes(normalized)) return 'P2'
  return 'P3'
}

const confidenceFromSeverity = (severity) => {
  const normalized = String(severity || '').toLowerCase()
  if (normalized === 'critical') return 96
  if (normalized === 'high') return 90
  if (normalized === 'medium') return 78
  return 62
}

const formatDate = (value) => {
  if (!value) return '--'
  return new Date(value).toLocaleString('zh-CN', { hour12: false })
}

const cleanAlertTitle = (title) => String(title || '').replace(/^Alert-|^Demo 预警：/, '').trim()

const statusLabel = (status) => {
  const map = {
    open: '待人工核准',
    acknowledged: '已确认推送',
    ignored: '已忽略',
    closed: '已关闭'
  }
  return map[String(status || '').toLowerCase()] || status || '待核准'
}

const mapAlert = (alert) => {
  const title = cleanAlertTitle(alert.title)
  const mergedText = `${title} ${alert.summary || ''}`
  return {
    id: alert.id,
    caseId: alert.case_id,
    date: formatDate(alert.created_at),
    subject: title || alert.id,
    industry: alert.severity || '风险线索',
    confidence: confidenceFromSeverity(alert.severity),
    level: severityLevel(alert.severity),
    has12345: mergedText.includes('12345') || mergedText.includes('热线') || mergedText.includes('信访'),
    has110: mergedText.includes('110') || mergedText.includes('警情') || mergedText.includes('接警'),
    creditCode: String(alert.case_id || alert.id).slice(0, 8),
    legalPerson: '',
    clueType: alert.severity || 'risk_alert',
    summary12345: alert.summary || '后端未返回 12345 专项摘要，需进入案件详情查看原始来源。',
    summary110: mergedText.includes('110') || mergedText.includes('警情') ? alert.summary : '暂无 110 警情交叉印证。',
    graphInfo: '等待加载案件图谱实体与关系。',
    status: alert.status,
    evidenceTags: [alert.severity || 'risk', alert.status || 'open'],
    raw: alert
  }
}

const enrichWarningWithCase = async (warning) => {
  if (!warning?.caseId) return warning
  const detail = await apiGet(`/risk/cases/${warning.caseId}`)
  const caseInfo = detail?.case_info || {}
  const entities = detail?.entities || []
  const relations = detail?.relations || []
  const recommendations = detail?.recommendations || {}
  const tags = [
    ...(caseInfo.risk_tags || []),
    caseInfo.graph_sync_status ? `graph:${caseInfo.graph_sync_status}` : '',
    caseInfo.vector_sync_status ? `vector:${caseInfo.vector_sync_status}` : ''
  ].filter(Boolean)

  return {
    ...warning,
    subject: caseInfo.title || warning.subject,
    industry: caseInfo.source_type || warning.industry,
    confidence: Math.round(caseInfo.risk_score || warning.confidence),
    level: caseInfo.risk_level === 'high' ? 'P1' : warning.level,
    has12345: warning.has12345 || caseInfo.source_type === 'hotline_12345',
    has110: warning.has110 || caseInfo.source_type === 'police_110',
    creditCode: caseInfo.case_code || warning.creditCode,
    legalPerson: caseInfo.assignee || '责任人待分派',
    clueType: `${caseInfo.risk_level || warning.raw?.severity || 'unknown'} / ${caseInfo.review_status || 'pending'}`,
    summary12345: recommendations.reason_summary || caseInfo.risk_reason_summary || warning.summary12345,
    summary110: caseInfo.disposal_advice?.join('；') || warning.summary110,
    graphInfo: `已抽取 ${entities.length} 个实体、${relations.length} 条关系；图谱同步 ${caseInfo.graph_sync_status || 'pending'}，向量索引 ${caseInfo.vector_sync_status || 'pending'}。`,
    evidenceTags: tags.length ? tags.slice(0, 8) : warning.evidenceTags
  }
}

const loadAlerts = async () => {
  isLoading.value = true
  apiError.value = ''
  try {
    const result = await apiGet('/alerts?page_size=50')
    warnings.value = (result?.items || []).map(mapAlert)
    if (warnings.value.length) {
      await selectRow(warnings.value[0])
    } else {
      currentWarning.value = null
    }
  } catch (error) {
    apiError.value = error.message
    ElMessage.error(`预警列表加载失败：${error.message}`)
  } finally {
    isLoading.value = false
  }
}

const filteredWarnings = computed(() => {
  return warnings.value.filter((item) => {
    const hitLevel = !filters.value.level || item.level === filters.value.level
    const keyword = filters.value.subject.trim()
    const hitSubject = !keyword || item.subject.includes(keyword) || item.creditCode.includes(keyword)
    return hitLevel && hitSubject
  })
})

const selectRow = async (row) => {
  currentWarning.value = row
  try {
    currentWarning.value = await enrichWarningWithCase(row)
  } catch (error) {
    currentWarning.value = row
    ElMessage.warning(`案件聚合详情加载失败：${error.message}`)
  }
}

const handleSearch = async () => {
  if (!warnings.value.length) {
    await loadAlerts()
    return
  }
  if (filteredWarnings.value.length > 0 && !filteredWarnings.value.includes(currentWarning.value)) {
    await selectRow(filteredWarnings.value[0])
  } else if (filteredWarnings.value.length === 0) {
    currentWarning.value = null
  }
}

const setAlertStatus = async (status) => {
  if (!currentWarning.value || isActing.value) return
  isActing.value = true
  try {
    await apiPost(`/alerts/${currentWarning.value.id}/status`, { status })
    ElMessage.success(status === 'acknowledged' ? '预警已确认并推送处置' : '预警已标记为忽略')
    await loadAlerts()
  } catch (error) {
    ElMessage.error(`预警状态更新失败：${error.message}`)
  } finally {
    isActing.value = false
  }
}

onMounted(loadAlerts)
</script>

<style scoped>
/* 核心容器：柔奶白 */
.warning-center-hud {
  display: flex; flex-direction: column; width: 100%; height: 95vh;
  background-color: #F5EFEA; color: #333333; overflow: hidden;
  font-family: 'PingFang SC', 'Microsoft YaHei', sans-serif;
}

.mono-text { font-family: 'JetBrains Mono', Consolas, monospace; }

/* 1. 顶部终端过滤栏 (改为白底高对比) */
.terminal-header {
  display: flex; justify-content: space-between; align-items: center; padding: 15px 24px;
  background: #FFFFFF; border-bottom: 1px solid rgba(18, 46, 138, 0.15);
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.02); z-index: 10;
}
.header-left { display: flex; align-items: center; gap: 8px; font-size: 14px; font-weight: bold; color: #122E8A; }
.prompt-symbol { font-weight: 900; }
.blinking-cursor { animation: blink 1s step-end infinite; }
.header-title { font-family: 'JetBrains Mono', monospace; font-size: 13px; color: #333; }

.header-right { display: flex; gap: 15px; align-items: center; }
.filter-group { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: bold; color: #122E8A; }
.hud-select, .hud-input {
  background: #F9F9F9; border: 1px solid rgba(18, 46, 138, 0.2); color: #333;
  padding: 8px 12px; font-size: 13px; outline: none; border-radius: 4px; transition: 0.2s;
  font-family: 'JetBrains Mono', 'PingFang SC', sans-serif; font-weight: 500;
}
.hud-input { width: 220px; }
.hud-input::placeholder { color: #999; font-weight: normal; }
.hud-select:focus, .hud-input:focus { border-color: #122E8A; background: #FFFFFF; box-shadow: 0 0 0 2px rgba(18, 46, 138, 0.1); }

.hud-btn {
  background: transparent; border: 1px solid; padding: 8px 16px; font-size: 13px; font-weight: bold; cursor: pointer; transition: 0.2s; border-radius: 4px; font-family: 'JetBrains Mono', sans-serif;
}
.hud-btn.primary { background: #122E8A; color: #FFFFFF; border-color: #122E8A; }
.hud-btn.primary:hover { background: #0D226A; }
.hud-btn.danger { background: #D9363E; color: #FFFFFF; border-color: #D9363E; }
.hud-btn.danger:hover { background: #B32D33; }
.hud-btn.ghost { color: #666; border-color: #CCC; background: #FFF; }
.hud-btn.ghost:hover { border-color: #122E8A; color: #122E8A; background: rgba(18, 46, 138, 0.05); }
.hud-btn:disabled { opacity: 0.62; cursor: not-allowed; }
.hud-btn.block { display: block; width: 100%; margin-top: 10px; }

/* 2. 主体分栏区 */
.split-layout { flex: 1; display: flex; padding: 20px; gap: 20px; overflow: hidden; }

/* 左侧表格面板 */
.grid-panel {
  flex: 65; position: relative; background: #FFFFFF; border-radius: 6px;
  border: 1px solid rgba(18, 46, 138, 0.15); display: flex; flex-direction: column;
  box-shadow: 0 4px 20px rgba(18, 46, 138, 0.05);
}
.table-scroller { flex: 1; overflow-y: auto; padding: 0; border-radius: 6px; }

/* 高密度纯白政务表格 */
.bloomberg-table { width: 100%; border-collapse: collapse; font-size: 13px; text-align: left; }
.bloomberg-table th {
  position: sticky; top: 0; background: #F4F5F9; color: #122E8A; padding: 12px 16px;
  font-weight: bold; border-bottom: 2px solid rgba(18, 46, 138, 0.1); z-index: 2;
}
.data-row { cursor: pointer; border-bottom: 1px solid #EAEAEA; transition: all 0.2s; background: #FFFFFF; }
.data-row:hover { background: rgba(18, 46, 138, 0.02); }
.data-row.selected-row { background: rgba(18, 46, 138, 0.06); border-left: 3px solid #122E8A; }
.bloomberg-table td { padding: 12px 16px; color: #333; }
.empty-table-cell { text-align: center; color: #999 !important; padding: 36px !important; font-weight: bold; }

.dim { color: #666; font-size: 12px; }
.subject-cell { font-weight: 800; color: #122E8A; font-size: 14px; }
.industry-cell { font-size: 12px; color: #666; }

/* 风险级别徽章 */
.level-badge { padding: 3px 8px; border-radius: 4px; font-size: 11px; font-weight: 900; font-family: 'JetBrains Mono', sans-serif; }
.P1 { color: #D9363E; background: rgba(217, 54, 62, 0.1); border: 1px solid rgba(217, 54, 62, 0.3); }
.P2 { color: #F5A623; background: rgba(245, 166, 35, 0.1); border: 1px solid rgba(245, 166, 35, 0.3); }
.P3 { color: #0F7E3B; background: rgba(15, 126, 59, 0.1); border: 1px solid rgba(15, 126, 59, 0.25); }

/* 双域印证标签 */
.dual-domain-tags { display: flex; gap: 6px; }
.d-tag { font-size: 10px; font-weight: bold; padding: 2px 6px; border: 1px solid #CCC; color: #999; border-radius: 2px; }
.active-12345 { border-color: #122E8A; color: #122E8A; background: rgba(18, 46, 138, 0.05); }
.active-110 { border-color: #D9363E; color: #D9363E; background: rgba(217, 54, 62, 0.05); }

/* 置信度进度条 */
.confidence-bar-wrapper { display: flex; align-items: center; gap: 10px; }
.c-bar { height: 6px; border-radius: 3px; background: #122E8A; }
.c-text { font-size: 12px; font-weight: bold; color: #333; width: 35px; text-align: right; }

/* 列表操作按钮 */
.action-btn { background: none; border: none; color: #122E8A; font-weight: bold; font-size: 12px; cursor: pointer; text-decoration: underline; padding: 0; }
.action-btn:hover { color: #D9363E; }

/* 右侧核查面板 */
.inspector-panel {
  flex: 35; position: relative; background: #FFFFFF; border-radius: 6px;
  border: 1px solid rgba(18, 46, 138, 0.15); padding: 24px; display: flex; flex-direction: column;
  box-shadow: 0 4px 20px rgba(18, 46, 138, 0.05); overflow-y: auto;
}
.inspector-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 15px; }
.record-id { font-size: 12px; color: #666; font-weight: bold; }
.status-tag { font-size: 12px; font-weight: bold; padding: 4px 10px; border-radius: 4px; background: rgba(245, 166, 35, 0.1); border: 1px solid #F5A623; color: #D98C12; }
.pulse-warn { animation: pulse-border 2s infinite; }

.target-name { font-size: 24px; margin: 0 0 20px 0; font-weight: 900; color: #122E8A; letter-spacing: 0.5px; }

.info-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 15px; margin-bottom: 25px; padding-bottom: 20px; border-bottom: 1px dashed rgba(18, 46, 138, 0.2); }
.info-item { display: flex; flex-direction: column; gap: 6px; }
.i-label { font-size: 12px; color: #666; font-weight: bold; }
.i-val { font-size: 15px; color: #333; font-weight: bold; }
.highlight-red { color: #D9363E; }

.ai-report-box { background: #F9F9F9; border: 1px solid rgba(18, 46, 138, 0.15); border-radius: 6px; padding: 18px; margin-bottom: 20px; }
.box-title { font-size: 14px; color: #122E8A; font-weight: 900; margin-bottom: 12px; border-bottom: 2px solid #122E8A; display: inline-block; padding-bottom: 4px; }
.report-content { display: flex; flex-direction: column; gap: 12px; }
.report-line { font-size: 13px; line-height: 1.6; display: flex; flex-direction: column; gap: 4px; }
.r-label { color: #122E8A; font-weight: bold; font-size: 12px; }
.r-val { color: #333; font-weight: 500; padding-left: 6px; }

.evidence-tags { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 30px; }
.e-tag { background: rgba(217, 54, 62, 0.05); border: 1px solid rgba(217, 54, 62, 0.2); color: #D9363E; font-size: 12px; font-weight: bold; padding: 6px 12px; border-radius: 4px; }

.action-footer { margin-top: auto; padding-top: 20px; }

/* 空状态 */
.empty-state { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; color: #999; font-size: 14px; font-weight: bold; gap: 20px; }
.scan-radar { width: 60px; height: 60px; border: 2px dashed #CCC; border-radius: 50%; position: relative; animation: rotate 4s linear infinite; opacity: 0.5; }
.scan-radar::after { content: ''; position: absolute; top: 50%; left: 50%; width: 50%; height: 3px; background: #CCC; transform-origin: left center; }

/* 装饰边角 - 改为深海蓝实线块，增加严谨科研感 */
.panel-corner { position: absolute; width: 14px; height: 14px; border: 3px solid #122E8A; border-radius: 2px; }
.top-left { top: -1px; left: -1px; border-right: 0; border-bottom: 0; }
.bottom-right { bottom: -1px; right: -1px; border-left: 0; border-top: 0; }
.top-right { top: -1px; right: -1px; border-left: 0; border-bottom: 0; }
.bottom-left { bottom: -1px; left: -1px; border-right: 0; border-top: 0; }

/* 滚动条美化 */
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: rgba(18, 46, 138, 0.2); border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: rgba(18, 46, 138, 0.4); }

@keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0; } }
@keyframes pulse-border { 0%, 100% { box-shadow: 0 0 0 0 rgba(245, 166, 35, 0.4); } 50% { box-shadow: 0 0 0 6px rgba(245, 166, 35, 0); } }
@keyframes rotate { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
</style>

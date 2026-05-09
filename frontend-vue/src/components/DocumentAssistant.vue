<template>
  <div class="document-assistant-hud">
    <header class="hud-top-bar">
      <div class="bar-left">
        <span class="blinking-cursor">_</span>
        <span class="sys-title">AI_LEGAL_DRAFTING_SYSTEM_v2.0</span>
      </div>
      <div class="bar-right">
        <button class="hud-btn ghost" :disabled="isGenerating" @click="handleRegenerate">
          <i class="el-icon-refresh"></i> [ 重新生成并校对 ]
        </button>
        <button class="hud-btn primary" :disabled="!latestReport?.file_path" @click="handleExport">
          [ 签发并导出 PDF ]
        </button>
      </div>
    </header>

    <main class="workspace-layout">
      <!-- 左侧栏 -->
      <aside class="side-panel left-panel">
        <div class="panel-decorator"></div>
        <div class="queue-section">
          <div class="panel-title">高危待办线索池 (Queue)</div>
          <div class="case-list">
            <div 
              v-for="c in caseQueue" :key="c.id" 
              :class="['case-item', { active: activeCase.id === c.id }]"
              @click="selectCase(c)"
            >
              <div class="c-header">
                <span class="c-id">ID: {{ c.shortId }}</span>
                <span :class="['c-tag', c.level]">{{ c.levelText }}</span>
              </div>
              <div class="c-subject">{{ c.subject }}</div>
              <div class="c-type">{{ c.type }}</div>
            </div>
          </div>
        </div>

        <div class="divider"></div>

        <div class="outline-section">
          <div class="panel-title">文书生成大纲 & 校验</div>
          <div class="outline-list">
            <div class="outline-item success">
              <span class="icon">✓</span>
              <div class="text-box">
                <div class="o-title">文书抬头与管辖权声明</div>
                <div class="o-desc">符合通州区人民检察院规范</div>
              </div>
            </div>
            <div class="outline-item success">
              <span class="icon">✓</span>
              <div class="text-box">
                <div class="o-title">核心违法事实陈述</div>
                <div class="o-desc">已融合 12345 与 110 数据印证</div>
              </div>
            </div>
            <div class="outline-item generating" v-if="isGenerating">
              <span class="icon spinner">⟳</span>
              <div class="text-box">
                <div class="o-title">法律适用与处置建议</div>
                <div class="o-desc">GLM-5.1 正在检索相关法条...</div>
              </div>
            </div>
            <div class="outline-item pending" v-else>
              <span class="icon">✓</span>
              <div class="text-box">
                <div class="o-title">法律适用与处置建议</div>
                <div class="o-desc">已生成模型推荐处置方案</div>
              </div>
            </div>
            <div class="outline-item pending">
              <span class="icon">○</span>
              <div class="text-box">
                <div class="o-title">检察官人工复核</div>
                <div class="o-desc">等待签名确认</div>
              </div>
            </div>
          </div>
        </div>
      </aside>

      <!-- 中间文书编辑器 -->
      <section class="editor-panel">
        <div class="panel-decorator"></div>
        <div class="editor-header">
          <span class="file-name">DOCUMENT: 检察建议书草稿_{{ activeCase.subject }}.docx</span>
          <span class="word-count">状态: {{ isGenerating ? 'AI 撰写中...' : '起草完毕' }}</span>
        </div>

        <div :class="['paper-container', { 'is-generating': isGenerating }]">
          <div class="paper-scanner" v-if="isGenerating"></div>
          <textarea
            v-model="draftContent"
            class="holographic-textarea"
            readonly
            spellcheck="false"
          ></textarea>
        </div>

        <div class="editor-footer">
          <span class="warning-text">
            <i class="el-icon-warning-outline"></i> 警告：本平台生成结果仅供参考，不构成最终法定文书，必须经由承办检察官人工复核。
          </span>
        </div>
      </section>

      <!-- 右侧溯源栏 -->
      <aside class="side-panel right-panel">
        <div class="panel-decorator"></div>
        <div class="panel-title">事实锚点与证据溯源</div>

        <div class="evidence-box">
          <div class="e-header">锚定主体</div>
          <div class="e-val highlight">{{ activeCase.subject }}</div>
          <div class="e-meta">统一信用代码: {{ activeCase.code }}</div>
        </div>

        <div class="evidence-box">
          <div class="e-header">涉嫌违规领域</div>
          <div class="e-val alert">{{ activeCase.type }}</div>
        </div>

        <div class="evidence-box">
          <div class="e-header">双域印证事实摘要 (溯源记录)</div>
          <ul class="trace-list">
            <li v-if="activeCase.trace12345">
              <span class="tag tag-blue">12345</span>
              <p>{{ activeCase.trace12345 }}<a href="#">[查看原始工单]</a></p>
            </li>
            <li v-if="activeCase.trace110">
              <span class="tag tag-red">110</span>
              <p>{{ activeCase.trace110 }}<a href="#">[查看出警记录]</a></p>
            </li>
            <li>
              <span class="tag tag-purple">图谱</span>
              <p>{{ activeCase.traceGraph }}<a href="#">[查看实体图谱]</a></p>
            </li>
          </ul>
        </div>
      </aside>
    </main>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { apiGet, apiPost } from '../api/platform'

const defaultCase = {
  id: '',
  shortId: '--',
  subject: '等待加载高危线索',
  code: '--',
  level: 'crit',
  levelText: '待加载',
  type: '后端风险案件',
  trace12345: '正在从 /risk/cases 加载案件队列。',
  trace110: '',
  traceGraph: '等待图谱/向量同步状态。',
  baseDraft: '正在加载后端案件与报告生成能力...'
}

const caseQueue = ref([])
const activeCase = ref(defaultCase)
const draftContent = ref(defaultCase.baseDraft)
const isGenerating = ref(false)
const latestReport = ref(null)
const apiError = ref('')

const formatDateCompact = () => new Date().toISOString().slice(0, 10)

const splitAdvice = (value) => {
  if (Array.isArray(value)) return value
  return String(value || '')
    .split(/[|；;]\s*/)
    .map((item) => item.trim())
    .filter(Boolean)
}

const levelText = (riskLevel) => {
  const normalized = String(riskLevel || '').toLowerCase()
  if (normalized === 'high') return '高危'
  if (normalized === 'medium') return '中危'
  if (normalized === 'low') return '低危'
  return '待研判'
}

const levelClass = (riskLevel) => (String(riskLevel || '').toLowerCase() === 'high' ? 'crit' : 'warn')

const buildDraftFromCase = (caseItem, detail = null, report = null) => {
  if (report?.content_markdown) return report.content_markdown
  const caseInfo = detail?.case_info || {}
  const recommendations = detail?.recommendations || {}
  const advice = splitAdvice(caseInfo.disposal_advice || recommendations.disposal_advice)
  return [
    '《检察建议书（AI 草稿）》',
    '',
    `线索名称：${caseInfo.title || caseItem.subject}`,
    `案件编号：${caseInfo.case_code || caseItem.code}`,
    `风险等级：${levelText(caseInfo.risk_level || caseItem.levelText)}，风险评分：${caseInfo.risk_score ?? '--'}`,
    '',
    '一、风险事实摘要',
    caseInfo.risk_reason_summary || recommendations.reason_summary || caseItem.trace12345 || '后端暂未返回风险原因摘要。',
    '',
    '二、多源数据与图谱印证',
    `来源类型：${caseInfo.source_type || '--'}；属地：${caseInfo.area_name || '--'}。`,
    `图谱同步：${caseInfo.graph_sync_status || '--'}；向量索引：${caseInfo.vector_sync_status || '--'}。`,
    caseItem.traceGraph,
    '',
    '三、建议处置措施',
    ...(advice.length ? advice.map((item, index) => `${index + 1}. ${item}`) : ['1. 请承办检察官复核后补充处置建议。']),
    '',
    '【复核提示】本草稿由 JusticeAI 调用 OpenAI-compatible 报告/建议链路生成或整理，签发前必须人工复核事实、法条与措辞。'
  ].join('\n')
}

const mapCase = (item) => ({
  id: item.id,
  shortId: item.case_code || String(item.id).slice(0, 8),
  subject: item.title || item.case_code,
  code: item.case_code || String(item.id).slice(0, 8),
  level: levelClass(item.risk_level),
  levelText: levelText(item.risk_level),
  type: `${item.source_type || 'unknown'} / ${item.review_status || 'pending'}`,
  trace12345: item.risk_reason_summary || '等待案件详情加载风险原因摘要。',
  trace110: splitAdvice(item.disposal_advice).join('；'),
  traceGraph: `图谱同步 ${item.graph_sync_status || 'pending'}，向量索引 ${item.vector_sync_status || 'pending'}。`,
  baseDraft: buildDraftFromCase({
    subject: item.title,
    code: item.case_code,
    trace12345: item.risk_reason_summary,
    traceGraph: `图谱同步 ${item.graph_sync_status || 'pending'}，向量索引 ${item.vector_sync_status || 'pending'}。`
  }, { case_info: item })
})

const loadCases = async () => {
  apiError.value = ''
  try {
    const result = await apiGet('/risk/cases?page_size=20')
    caseQueue.value = (result?.items || []).map(mapCase)
    if (caseQueue.value.length) {
      await selectCase(caseQueue.value[0])
    } else {
      activeCase.value = defaultCase
      draftContent.value = '暂无风险案件，需先在“异构数据接入”完成导入、处理与抽取。'
    }
  } catch (error) {
    apiError.value = error.message
    draftContent.value = `案件队列加载失败：${error.message}`
    ElMessage.error(`案件队列加载失败：${error.message}`)
  }
}

const selectCase = async (c) => {
  if (isGenerating.value) return
  activeCase.value = c
  draftContent.value = c.baseDraft
  latestReport.value = null
  try {
    const detail = await apiGet(`/risk/cases/${c.id}`)
    const caseInfo = detail?.case_info || {}
    const nextCase = {
      ...c,
      subject: caseInfo.title || c.subject,
      code: caseInfo.case_code || c.code,
      level: levelClass(caseInfo.risk_level),
      levelText: levelText(caseInfo.risk_level),
      type: `${caseInfo.source_type || 'unknown'} / ${caseInfo.review_status || 'pending'}`,
      trace12345: detail?.recommendations?.reason_summary || caseInfo.risk_reason_summary || c.trace12345,
      trace110: splitAdvice(caseInfo.disposal_advice || detail?.recommendations?.disposal_advice).join('；'),
      traceGraph: `实体 ${detail?.entities?.length || 0} 个，关系 ${detail?.relations?.length || 0} 条；图谱 ${caseInfo.graph_sync_status || 'pending'}，向量 ${caseInfo.vector_sync_status || 'pending'}。`
    }
    activeCase.value = nextCase
    draftContent.value = buildDraftFromCase(nextCase, detail)
  } catch (error) {
    ElMessage.warning(`案件详情加载失败：${error.message}`)
  }
}

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

const waitForJob = async (jobId, timeoutMs = 12 * 60 * 1000) => {
  const startedAt = Date.now()
  while (Date.now() - startedAt < timeoutMs) {
    const job = await apiGet(`/jobs/${jobId}`)
    const normalized = String(job.status || '').toLowerCase()
    draftContent.value = `报告生成任务：${job.status} · ${job.progress_percent}%\n\n${job.message || '等待模型生成报告内容...'}`
    if (['completed', 'completed_with_warnings', 'failed', 'cancelled'].includes(normalized)) return job
    await delay(2000)
  }
  throw new Error(`报告生成任务 ${jobId} 超时，请到技术运维后台查看。`)
}

const handleRegenerate = async () => {
  if (isGenerating.value) return
  if (!activeCase.value?.id) {
    ElMessage.warning('暂无可生成文书的案件')
    return
  }
  isGenerating.value = true
  latestReport.value = null
  try {
    const job = await apiPost('/reports/generate/async', {
      report_type: 'procuratorial_suggestion_draft',
      period: formatDateCompact(),
      title: `检察建议书草稿-${activeCase.value.code}`
    })
    draftContent.value = `报告生成任务已入队：${job.id}\n\n正在调用 OpenAI-compatible 报告生成链路，请稍候...`
    const completed = await waitForJob(job.id)
    if (completed.status === 'failed') {
      throw new Error(completed.error_message || completed.message || '报告生成失败')
    }
    const reportId = completed.result?.report_id || completed.target_id
    const report = reportId ? await apiGet(`/reports/${reportId}`) : null
    latestReport.value = report || completed.result || null
    draftContent.value = buildDraftFromCase(activeCase.value, null, report) || completed.message
    ElMessage.success('AI 文书草稿已生成')
  } catch (error) {
    draftContent.value = `AI 文书生成失败：${error.message}\n\n${activeCase.value.baseDraft}`
    ElMessage.error(`AI 文书生成失败：${error.message}`)
  } finally {
    isGenerating.value = false
  }
}

const handleExport = () => {
  if (!latestReport.value?.file_path) {
    ElMessage.warning('当前草稿尚未生成后端报告文件')
    return
  }
  ElMessage.info(`报告文件已生成：${latestReport.value.file_path}。PDF 导出服务尚未接入，当前提供 Markdown 草稿审签。`)
}

onMounted(loadCases)
</script>

<style scoped>
.document-assistant-hud {
  display: flex; flex-direction: column; width: 100%; height: 93vh;
  background-color: #F5EFEA; color: #333; overflow: hidden;
  font-family: 'PingFang SC', sans-serif;
}

.hud-top-bar {
  height: 60px; background: #FFFFFF; border-bottom: 1px solid rgba(18, 46, 138, 0.15);
  display: flex; justify-content: space-between; align-items: center; padding: 0 24px;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.02);
}
.bar-left { font-family: 'JetBrains Mono', monospace; font-size: 15px; color: #122E8A; font-weight: bold; }
.bar-right { display: flex; gap: 15px; }

.hud-btn { border: 1px solid; padding: 8px 16px; font-size: 13px; cursor: pointer; transition: 0.2s; font-family: 'JetBrains Mono', sans-serif; border-radius: 4px; }
.hud-btn.primary { background: #122E8A; color: #FFFFFF; border-color: #122E8A; font-weight: bold; }
.hud-btn.primary:hover { background: #0D226A; }
.hud-btn.ghost { background: transparent; color: #122E8A; border-color: #122E8A; }
.hud-btn.ghost:hover { background: rgba(18, 46, 138, 0.05); }
.hud-btn:disabled { opacity: 0.62; cursor: not-allowed; }

.workspace-layout { flex: 1; display: flex; padding: 20px; gap: 20px; overflow: hidden; }

.side-panel {
  width: 320px; background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.15);
  display: flex; flex-direction: column; position: relative; border-radius: 6px;
  box-shadow: 0 4px 15px rgba(18, 46, 138, 0.04);
}
.left-panel { overflow: hidden; }
.right-panel { padding: 20px; overflow-y: auto; }

.panel-title { font-size: 14px; color: #122E8A; font-weight: bold; margin-bottom: 15px; border-bottom: 2px solid rgba(18, 46, 138, 0.1); padding-bottom: 10px; }
.divider { height: 1px; background: rgba(18, 46, 138, 0.1); margin: 0; }

.queue-section { flex: 4; display: flex; flex-direction: column; padding: 20px 20px 10px; overflow: hidden; }
.case-list { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 10px; }
.case-item { background: #F9F9F9; border: 1px solid rgba(18, 46, 138, 0.08); padding: 12px; border-radius: 6px; cursor: pointer; transition: 0.2s; }
.case-item:hover { border-color: #122E8A; }
.case-item.active { background: rgba(18, 46, 138, 0.05); border-left: 4px solid #122E8A; }
.c-id { font-size: 11px; color: #666; font-family: 'JetBrains Mono'; }
.c-tag { font-size: 10px; padding: 2px 6px; border-radius: 2px; }
.c-tag.crit { background: rgba(217, 54, 62, 0.1); color: #D9363E; border: 1px solid #D9363E; }
.c-tag.warn { background: rgba(245, 166, 35, 0.12); color: #B87300; border: 1px solid rgba(245, 166, 35, 0.5); }
.c-subject { font-size: 14px; font-weight: bold; color: #333; margin: 6px 0 4px; }
.c-type { font-size: 12px; color: #666; }

.outline-section { flex: 5; display: flex; flex-direction: column; padding: 15px 20px 20px; overflow: hidden; }
.outline-list { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 14px; }
.outline-item { display: flex; gap: 10px; align-items: flex-start; }
.icon { width: 20px; height: 20px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: bold; border: 1px solid currentColor; margin-top: 2px; }
.success .icon { color: #52C41A; background: rgba(82, 196, 26, 0.1); }
.generating .icon { color: #122E8A; background: rgba(18, 46, 138, 0.1); }
.pending .icon { color: #999; border-color: #999; }
.o-title { font-size: 13px; color: #333; font-weight: bold; margin-bottom: 4px; }
.o-desc { font-size: 12px; color: #666; }

.evidence-box { background: #F9F9F9; border: 1px solid rgba(18, 46, 138, 0.1); padding: 15px; margin-bottom: 15px; border-radius: 6px; }
.e-header { font-size: 12px; color: #666; margin-bottom: 8px; font-weight: bold; }
.e-val { font-size: 15px; font-weight: bold; }
.e-val.highlight { color: #122E8A; }
.e-val.alert { color: #D9363E; }
.e-meta { font-size: 11px; color: #666; margin-top: 6px; font-family: 'JetBrains Mono', monospace; }

.trace-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 10px; }
.trace-list li { background: #FFFFFF; padding: 10px; border-radius: 4px; border-left: 3px solid; border: 1px solid #EEE; border-left-width: 3px; }
.trace-list li p { margin: 5px 0 0; font-size: 12px; line-height: 1.6; color: #333; }
.trace-list li a { color: #122E8A; text-decoration: underline; margin-left: 5px; cursor: pointer; }
.tag { font-size: 10px; padding: 2px 6px; border-radius: 2px; border: 1px solid currentColor; font-weight: bold; }
.tag-blue { color: #122E8A; border-left-color: #122E8A; }
.tag-red { color: #D9363E; border-left-color: #D9363E; }
.tag-purple { color: #8B5CF6; border-left-color: #8B5CF6; }

.editor-panel {
  flex: 1; background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.2); border-radius: 6px;
  box-shadow: 0 4px 20px rgba(18, 46, 138, 0.08); display: flex; flex-direction: column; position: relative;
}
.editor-header {
  padding: 15px 24px; border-bottom: 1px solid rgba(18, 46, 138, 0.15); display: flex;
  justify-content: space-between; align-items: center; font-family: 'JetBrains Mono', monospace;
  font-size: 12px; color: #666; background: #F9F9F9; border-radius: 6px 6px 0 0;
}
.file-name { color: #122E8A; font-weight: bold; }

.paper-container { flex: 1; position: relative; padding: 30px 50px; overflow: hidden; background: #FFFFFF; }
.holographic-textarea {
  width: 100%; height: 100%; background: transparent; border: none; resize: none; outline: none;
  font-family: 'FangSong', '仿宋', 'PingFang SC', serif; font-size: 17px; line-height: 2.2;
  color: #333333; font-weight: 500;
}
.editor-footer { padding: 12px 24px; background: rgba(217, 54, 62, 0.05); border-top: 1px dashed rgba(217, 54, 62, 0.2); }
.warning-text { font-size: 12px; color: #D9363E; font-weight: bold; }

.panel-decorator::before {
  content: ''; position: absolute; top: -1px; left: -1px; width: 15px; height: 15px;
  border-top: 2px solid #122E8A; border-left: 2px solid #122E8A; z-index: 10;
}
</style>

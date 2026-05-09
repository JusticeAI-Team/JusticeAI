<template>
  <div class="agent-workspace-hud">
    <aside class="history-sidebar">
      <div class="sidebar-header">
        <el-button class="new-chat-btn" @click="handleNewChat">
          <i class="el-icon-plus"></i> 发起新研判任务
        </el-button>
      </div>
      <div class="history-list">
        <div class="history-group-label">历史研判线索</div>
        <div v-for="item in historyTasks" :key="item.id" class="history-item" @click="selectHistoryCase(item)">
          <div class="item-top">
            <span class="item-tag" :class="item.type">{{ item.typeText }}</span>
            <span class="item-time">{{ item.time }}</span>
          </div>
          <div class="item-title">{{ item.title }}</div>
        </div>
      </div>
      <div class="sidebar-footer">
      </div>
    </aside>

    <main class="chat-section">
      <header class="chat-header">
        <div class="header-info">
          <span class="status-dot pulse"></span>
          <span class="agent-name">数智检察 Agent 协同中枢 (OpenAI-Compatible)</span>
        </div>
        <div class="header-tools">
          <i class="el-icon-setting"></i>
        </div>
      </header>

      <div class="chat-display" ref="chatScrollRef">
        <div v-for="(msg, i) in messages" :key="i" :class="['message-row', msg.role]">
          <div class="avatar">{{ msg.role === 'user' ? '检察' : 'AI' }}</div>
          <div class="bubble">
            <div class="bubble-content" v-html="msg.content"></div>
            <div v-if="msg.structured" class="structured-card">
              <div class="card-line"><span class="label">识别主体:</span> {{ msg.structured.subject }}</div>
              <div class="card-line"><span class="label">风险权重:</span> <span class="risk-val">{{ msg.structured.risk }}</span></div>
              <div class="card-line"><span class="label">关联线索:</span> {{ msg.structured.clues }}</div>
            </div>
          </div>
        </div>
      </div>

      <footer class="input-area">
        <div class="input-wrapper">
          <input 
            v-model="userInput" 
            placeholder="输入指令进行风险穿透、文书生成或数据比对..." 
            @keyup.enter="handleSendMessage"
          />
          <button :disabled="isLoading" @click="handleSendMessage">{{ isLoading ? '研判中' : '发送指令' }}</button>
        </div>
        <div class="input-shortcuts">
          <span @click="applyShortcut('线索溯源')">线索溯源</span>
          <span @click="applyShortcut('生成建议')">生成建议</span>
          <span @click="applyShortcut('导出简报')">导出简报</span>
        </div>
      </footer>
    </main>

    <aside class="graph-section">
      <div class="panel-decorator"></div>
      <header class="graph-header">
        <div class="kicker">GRAPH TOPOLOGY</div>
        <h3 class="title">关联风险实体穿透图谱</h3>
        <div class="glow-line"></div>
      </header>

      <div class="graph-container">
        <div ref="graphRef" class="graph-dom"></div>
      </div>

      <footer class="graph-legend">
        <div class="legend-item"><span class="dot comp"></span> 企业</div>
        <div class="legend-item"><span class="dot person"></span> 自然人</div>
        <div class="legend-item"><span class="dot risk"></span> 风险点</div>
      </footer>
    </aside>
  </div>
</template>

<script setup>
import { ref, onMounted, nextTick } from 'vue'
import * as echarts from 'echarts'
import { ElMessage } from 'element-plus'
import { apiGet, apiPost } from '../api/platform'

const chatScrollRef = ref(null)
const graphRef = ref(null)
const userInput = ref('')
const isLoading = ref(false)

const historyTasks = ref([])

const messages = ref([
  { role: 'ai', content: '您好，我是数智检察助理。系统已接入导入案件、知识实体、图谱关系、预警和处置任务。请输入案件关键词，或从左侧历史研判线索中选择。' }
])

let graphIns = null

const riskType = (riskLevel) => {
  if (riskLevel === 'high') return 'risk'
  if (riskLevel === 'medium') return 'fund'
  return 'check'
}

const riskText = (riskLevel) => {
  if (riskLevel === 'high') return '高危'
  if (riskLevel === 'medium') return '中危'
  return '核查'
}

const formatDate = (value) => value ? new Date(value).toLocaleDateString('zh-CN').slice(5) : '--'

const loadHistoryCases = async () => {
  try {
    const result = await apiGet('/risk/cases?page_size=8')
    historyTasks.value = (result?.items || []).map((item) => ({
      id: item.id,
      title: item.title || item.case_code,
      time: formatDate(item.updated_at || item.created_at),
      type: riskType(item.risk_level),
      typeText: riskText(item.risk_level),
      caseCode: item.case_code
    }))
  } catch (error) {
    ElMessage.warning(`历史研判线索加载失败：${error.message}`)
  }
}

const selectHistoryCase = async (item) => {
  userInput.value = item.title
  await runCaseAnalysis({ caseId: item.id, queryText: item.title })
}

const renderMarkdown = (value) => String(value || '')
  .replace(/&/g, '&amp;')
  .replace(/</g, '&lt;')
  .replace(/>/g, '&gt;')
  .replace(/^### (.*)$/gm, '<h3>$1</h3>')
  .replace(/^#### (.*)$/gm, '<h4>$1</h4>')
  .replace(/^- (.*)$/gm, '<div class="md-line">• $1</div>')
  .replace(/^(\d+)\. (.*)$/gm, '<div class="md-line">$1. $2</div>')
  .replace(/\*\*(.*?)\*\*/g, '<b>$1</b>')
  .replace(/\n/g, '<br/>')

const runCaseAnalysis = async ({ caseId = null, queryText = '' } = {}) => {
  isLoading.value = true
  messages.value.push({ role: 'user', content: `研判线索：${queryText}` })
  try {
    const analysis = await apiPost('/agent/analyze', {
      case_id: caseId,
      query: queryText,
      intent: 'risk_judgement'
    })
    const detail = analysis?.case_detail || {}
    const caseInfo = detail?.case_info || {}
    const entities = detail?.entities || []
    const relations = detail?.relations || []
    const recommendations = detail?.recommendations || {}
    const advice = Array.isArray(recommendations.disposal_advice)
      ? recommendations.disposal_advice
      : (caseInfo.disposal_advice || [])

    messages.value.push({
      role: 'ai',
      content: renderMarkdown(analysis?.answer_markdown) || `已完成案件聚合研判：风险等级 <b>${caseInfo.risk_level || '--'}</b>，评分 <b>${caseInfo.risk_score ?? '--'}</b>。图谱同步状态：${caseInfo.graph_sync_status || '--'}，向量索引状态：${caseInfo.vector_sync_status || '--'}。<br/><br/>${recommendations.reason_summary || caseInfo.risk_reason_summary || '后端暂未返回风险原因摘要。'}<br/><br/><b>建议：</b>${advice.length ? advice.join('；') : '请承办检察官复核后补充处置意见。'}`,
      structured: {
        subject: caseInfo.title || queryText,
        risk: `${caseInfo.risk_level || '--'} (${caseInfo.risk_score ?? '--'})`,
        clues: `命中方式 ${analysis?.matched_by || '--'} / 实体 ${entities.length} 个 / 关系 ${relations.length} 条 / 预警 ${detail?.alerts?.length || 0} 条`
      }
    })
    renderGraphFromAnalysis(analysis)
  } catch (error) {
    messages.value.push({ role: 'ai', content: `研判失败：${error.message}` })
  } finally {
    isLoading.value = false
    nextTick(() => { chatScrollRef.value.scrollTop = chatScrollRef.value.scrollHeight })
  }
}

const renderGraphFromAnalysis = (analysis) => {
  const graph = analysis?.graph || {}
  const nodes = (graph.nodes || []).map((item) => ({
    id: item.id,
    name: item.id,
    label: { show: true, formatter: item.label || item.id },
    symbolSize: item.size || 32,
    itemStyle: { color: item.color || '#122E8A' }
  }))
  const links = (graph.edges || []).map((item) => ({
    source: item.source,
    target: item.target,
    label: { show: true, formatter: item.label || '关联' }
  }))
  graphIns?.setOption({ series: [{ data: nodes, links }] })
}

const initGraph = () => {
  if (!graphRef.value) return
  graphIns = echarts.init(graphRef.value)
  
  // 适配明亮主题的图谱颜色
  const data = [
    { name: '华丰建设', symbolSize: 55, itemStyle: { color: '#122E8A' } }, // 深海蓝
    { name: '王某某(法人)', symbolSize: 35, itemStyle: { color: '#8B5CF6' } },
    { name: '关联企业A', symbolSize: 25, itemStyle: { color: '#4A90E2' } },
    { name: '异常资金节点', symbolSize: 35, itemStyle: { color: '#D9363E' } },
    { name: '12345线索', symbolSize: 20, itemStyle: { color: '#F5A623' } },
  ]
  const links = [
    { source: '华丰建设', target: '王某某(法人)', label: { show: true, formatter: '任职' } },
    { source: '华丰建设', target: '关联企业A', label: { show: true, formatter: '控股' } },
    { source: '王某某(法人)', target: '异常资金节点', label: { show: true, formatter: '转账' } },
    { source: '华丰建设', target: '12345线索', label: { show: true, formatter: '指向' } },
  ]

  graphIns.setOption({
    backgroundColor: 'transparent',
    series: [{
      type: 'graph',
      layout: 'force',
      animation: true,
      data: data,
      links: links,
      force: { repulsion: 600, edgeLength: [100, 150] }, // 增大排斥力，让节点散得更开
      roam: true,
      // 节点旁边的文字强化
      label: { 
        show: true, 
        position: 'right', 
        color: '#122E8A', // 使用最深的蓝色
        fontSize: 13,
        fontWeight: '900',
        textBorderColor: '#FFFFFF', // 纯白描边，绝对清晰
        textBorderWidth: 3
      },
      // 连线上的文字强化
      edgeLabel: {
        show: true,
        fontSize: 11,
        fontWeight: 'bold',
        color: '#122E8A', // 字体深海蓝
        backgroundColor: '#FFFFFF', // 实心纯白背景，遮盖连线
        borderColor: '#122E8A',     // 加上深海蓝边框
        borderWidth: 1,
        padding: [3, 6],
        borderRadius: 4
      },
      // 连线本身强化
      lineStyle: { 
        color: '#A0AABF', // 偏蓝的深灰色，确保显眼但不喧宾夺主
        curveness: 0.2, 
        width: 2 
      },
      emphasis: { 
        focus: 'adjacency', 
        lineStyle: { width: 4, color: '#122E8A' } // 鼠标悬浮时连线变粗且变深蓝
      }
    }]
  })
}

const handleSendMessage = async () => {
  if (!userInput.value) return
  const query = userInput.value
  userInput.value = ''
  await runCaseAnalysis({ queryText: query })
}

const applyShortcut = (text) => {
  userInput.value = text
}

const handleNewChat = () => {
  messages.value = [
    { role: 'ai', content: '已新建研判会话。请输入案件编号、属地、来源类型或风险关键词，后端将通过统一 Agent 分析接口匹配案件并返回图谱/向量/风险聚合结果。' }
  ]
  userInput.value = ''
}

onMounted(() => {
  initGraph()
  loadHistoryCases()
  window.addEventListener('resize', () => graphIns?.resize())
})
</script>

<style scoped>
.agent-workspace-hud {
  display: flex;
  width: 100%;
  height: 93vh;
  background-color: #F5EFEA; /* 柔奶白全局背景 */
  color: #333333;
  overflow: hidden;
}

/* 1. 左侧历史栏 */
.history-sidebar {
  width: 240px;
  background: #FFFFFF;
  border-right: 1px solid rgba(18, 46, 138, 0.1);
  display: flex;
  flex-direction: column;
}
.sidebar-header { padding: 20px; }
.new-chat-btn {
  width: 100%; background: transparent; border: 1px dashed #122E8A;
  color: #122E8A; font-size: 13px; font-weight: bold; padding: 10px 0; border-radius: 6px; cursor: pointer; transition: 0.2s;
}
.new-chat-btn:hover { background: rgba(18, 46, 138, 0.05); }
.history-list { flex: 1; padding: 0 15px; overflow-y: auto; }
.history-group-label { font-size: 11px; color: #122E8A; margin-bottom: 15px; font-weight: bold; }
.history-item {
  padding: 12px; background: #F9F9F9; border-radius: 6px;
  margin-bottom: 10px; cursor: pointer; transition: 0.3s;
  border: 1px solid rgba(18, 46, 138, 0.05);
}
.history-item:hover { background: rgba(18, 46, 138, 0.05); border-color: rgba(18, 46, 138, 0.2); }
.item-top { display: flex; justify-content: space-between; font-size: 11px; margin-bottom: 6px; color: #666; font-weight: bold; }
.item-tag.risk { color: #D9363E; }
.item-tag.fund { color: #8B5CF6; }
.item-tag.check { color: #122E8A; }
.item-title { font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: #333; }

/* 2. 中间对话区 */
.chat-section { flex: 1; display: flex; flex-direction: column; background: #F5EFEA; }
.chat-header {
  height: 60px; border-bottom: 1px solid rgba(18, 46, 138, 0.1);
  display: flex; align-items: center; justify-content: space-between; padding: 0 30px;
  background: #FFFFFF; box-shadow: 0 2px 10px rgba(0,0,0,0.02); z-index: 10;
}
.agent-name { font-weight: 700; font-size: 15px; color: #122E8A; }
.status-dot { width: 8px; height: 8px; background: #52C41A; border-radius: 50%; margin-right: 10px; display: inline-block; }

.chat-display { flex: 1; padding: 30px; overflow-y: auto; }
.message-row { display: flex; gap: 15px; margin-bottom: 30px; }
.avatar { width: 38px; height: 38px; background: #FFFFFF; color: #122E8A; border: 2px solid rgba(18, 46, 138, 0.2); display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: 800; border-radius: 8px; box-shadow: 0 2px 5px rgba(0,0,0,0.05); }
.bubble { max-width: 80%; }
.bubble-content { background: #FFFFFF; padding: 15px 20px; border-radius: 0 12px 12px 12px; font-size: 14px; line-height: 1.6; border: 1px solid rgba(18, 46, 138, 0.1); box-shadow: 0 4px 12px rgba(18, 46, 138, 0.05); color: #333; }
.message-row.user { flex-direction: row-reverse; }
.message-row.user .bubble-content { background: #122E8A; color: #FFFFFF; border-radius: 12px 0 12px 12px; border: none; box-shadow: 0 4px 12px rgba(18, 46, 138, 0.2); }

.structured-card { 
  margin-top: 12px; background: #F9F9F9; border: 1px solid rgba(18, 46, 138, 0.15); 
  padding: 15px; border-radius: 8px; font-size: 13px; color: #333; line-height: 1.8;
}
.risk-val { color: #D9363E; font-weight: 800; }
.label { color: #122E8A; font-weight: 700; margin-right: 6px; }

.input-area { padding: 20px 30px; background: #FFFFFF; border-top: 1px solid rgba(18, 46, 138, 0.1); z-index: 10; }
.input-wrapper { display: flex; gap: 10px; background: #F5EFEA; padding: 8px; border-radius: 8px; border: 1px solid rgba(18, 46, 138, 0.2); }
.input-wrapper input { flex: 1; background: transparent; border: none; color: #333; outline: none; font-size: 14px; padding-left: 10px; font-weight: 500; }
.input-wrapper input::placeholder { color: #999; }
.input-wrapper button { background: #122E8A; color: #FFFFFF; border: none; padding: 8px 24px; border-radius: 6px; font-weight: bold; cursor: pointer; transition: 0.2s; font-size: 14px; }
.input-wrapper button:hover { background: #0D226A; box-shadow: 0 2px 8px rgba(18, 46, 138, 0.3); }
.input-wrapper button:disabled { opacity: 0.62; cursor: not-allowed; box-shadow: none; }
.input-shortcuts { display: flex; gap: 15px; margin-top: 12px; font-size: 12px; color: #666; font-weight: bold; }
.input-shortcuts span { cursor: pointer; padding: 4px 8px; border-radius: 4px; transition: 0.2s; }
.input-shortcuts span:hover { background: rgba(18, 46, 138, 0.05); color: #122E8A; }

/* 3. 右侧图谱区 */
.graph-section {
  width: 480px; background: #FFFFFF; border-left: 1px solid rgba(18, 46, 138, 0.1);
  padding: 24px; position: relative; display: flex; flex-direction: column; box-shadow: -4px 0 20px rgba(0,0,0,0.02);
}
.graph-container { flex: 1; position: relative; margin: 20px 0; background: #F9F9F9; border-radius: 8px; border: 1px solid rgba(18, 46, 138, 0.1); }
.graph-dom { width: 100%; height: 100%; }
.kicker { font-size: 11px; color: #666; letter-spacing: 1px; font-weight: bold; font-family: 'JetBrains Mono', sans-serif; }
.graph-header .title { font-size: 18px; margin: 5px 0; color: #122E8A; font-weight: 900; }
.glow-line { width: 50px; height: 4px; background: #122E8A; margin-top: 8px; border-radius: 2px; }

.graph-legend { display: flex; gap: 15px; font-size: 12px; color: #333; justify-content: center; font-weight: bold; }
.dot { width: 12px; height: 12px; border-radius: 50%; display: inline-block; margin-right: 6px; border: 1px solid rgba(0,0,0,0.1); }
.dot.comp { background: #122E8A; }
.dot.person { background: #8B5CF6; }
.dot.risk { background: #D9363E; }

/* 装饰角标修改为深海蓝实线 */
.panel-decorator::before {
  content: ''; position: absolute; top: -1px; right: -1px; width: 20px; height: 20px;
  border-top: 3px solid #122E8A; border-right: 3px solid #122E8A;
}

/* 滚动条优化 */
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: rgba(18, 46, 138, 0.2); border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: rgba(18, 46, 138, 0.4); }
</style>

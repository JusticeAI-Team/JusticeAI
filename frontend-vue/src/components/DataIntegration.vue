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
        <span class="status-text">系统全链路状态: 运转中</span>
      </div>
    </header>

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
          <div :class="['upload-dropzone', { 'is-processing': isProcessing }]" @click="triggerFileInput">
            <div class="scan-beam" v-if="isProcessing"></div>
            <i class="el-icon-upload cloud-icon"></i>
            <p class="u-main">{{ isProcessing ? '数据流注入中...' : '拖拽 12345/110 数据包至此' }}</p>
            <p class="u-sub">.xlsx / .csv / .json</p>
            <input type="file" ref="fileInput" class="hidden-input" @change="handleFileSelect" />
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
            <h3 class="step-title">GLM-5.1 智能抽取提炼</h3>
            <p class="step-sub">大模型将大段“白话文”转为结构化标签</p>
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
              GLM-5.1 正在阅读并提取下一条批次... <span class="blinking-cursor">_</span>
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
import { ref, onMounted, onBeforeUnmount, nextTick } from 'vue'
import * as echarts from 'echarts'

const isProcessing = ref(false)
const fileInput = ref(null)
const terminalRef = ref(null)
const graphRef = ref(null)

let myChart = null

const animatedNodes = ref(142589)
const animatedEdges = ref(384102)

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

const mockAiProcess = [
  {
    raw: "我是马驹桥的农民工，华丰的包工头欠了我们三十几人工资，人跑了！",
    json: `{<br/>  <span style="color:#122E8A">"企业"</span>: "华丰建设",<br/>  <span style="color:#8B5CF6">"嫌疑人"</span>: "王大拿",<br/>  <span style="color:#D9363E">"新增线索"</span>: "12345新增欠薪"<br/>}`,
    newNodes: [ { id: '9', name: '12345新增欠薪', category: 2, symbolSize: 26 } ],
    newLinks: [
      { source: '9', target: '0', name: '直接投诉' },
      { source: '9', target: '1', name: '责任人' }
    ]
  }
]

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

const triggerFileInput = () => { fileInput.value.click() }

const scrollToBottom = () => {
  nextTick(() => {
    if (terminalRef.value) terminalRef.value.scrollTop = terminalRef.value.scrollHeight
  })
}

const handleFileSelect = (e) => {
  if (e.target.files.length > 0) {
    const fileName = e.target.files[0].name
    startPipeline(fileName)
  }
}

const startPipeline = (fileName) => {
  if (isProcessing.value) return
  isProcessing.value = true
  fileList.value.unshift({ name: fileName, status: 'AI 提炼中...', color: '#F5A623' })
  
  let step = 0
  const timer = setInterval(() => {
    if (step < mockAiProcess.length) {
      const currentStep = mockAiProcess[step]
      
      logs.value.push(currentStep)
      scrollToBottom()
      
      graphNodes.value.push(...currentStep.newNodes)
      graphLinks.value.push(...currentStep.newLinks)
      updateGraph() 
      
      animatedNodes.value += 1
      animatedEdges.value += 3
      
      step++
    } else {
      clearInterval(timer)
      isProcessing.value = false
      fileList.value[0].status = '入库结网完毕'
      fileList.value[0].color = '#0F7E3B'
    }
  }, 2500)
}

onMounted(() => {
  initGraph()
  scrollToBottom()
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

.pipeline-board { flex: 1; display: flex; align-items: stretch; padding: 30px; gap: 15px; overflow: hidden; }

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
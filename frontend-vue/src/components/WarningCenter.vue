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
        <button class="hud-btn primary" @click="handleSearch">[ 执行检索 ]</button>
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
            <span class="status-tag pulse-warn">待人工核准</span>
          </div>

          <h2 class="target-name">{{ currentWarning.subject }}</h2>
          
          <div class="info-grid">
            <div class="info-item">
              <span class="i-label">法定代表人:</span>
              <span class="i-val">{{ currentWarning.legalPerson }}</span>
            </div>
            <div class="info-item">
              <span class="i-label">线索类型:</span>
              <span class="i-val highlight-red">{{ currentWarning.clueType }}</span>
            </div>
          </div>

          <div class="ai-report-box">
            <div class="box-title">> GLM-5.1 双域印证摘要</div>
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
            <span class="e-tag">跨区域资金回流</span>
            <span class="e-tag">夜间高频投诉</span>
            <span class="e-tag">法人关联历史案卷</span>
          </div>

          <div class="action-footer">
            <button class="hud-btn danger block">确认预警并推送监管</button>
            <button class="hud-btn ghost block">标记为低风险/忽略</button>
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
import { ref, computed } from 'vue'

const filters = ref({ level: '', subject: '' })

const warnings = ref([
  {
    id: 1, date: '2026-04-26 15:24:11', subject: '华丰建设有限公司', industry: '建筑工程',
    confidence: 93, level: 'P1', has12345: true, has110: true, creditCode: '91110112MA01XX8899',
    legalPerson: '张某某', clueType: '欠薪引发群体聚集',
    summary12345: '近 7 日接收欠薪投诉 47 人次，涉案金额预估 312 万元。',
    summary110: '昨日 19:30 接警，该工地门口发生劳资纠纷聚集，出警 2 次。',
    graphInfo: '发现该企业资金池与 3 家空壳公司存在频繁资金拆借。'
  },
  {
    id: 2, date: '2026-04-26 14:10:05', subject: '京运置业发展中心', industry: '金融理财',
    confidence: 88, level: 'P1', has12345: false, has110: true, creditCode: '91110112MA03ZZ3344',
    legalPerson: '王某某', clueType: '异常资金募集/非吸',
    summary12345: '暂未发现显著相关投诉。',
    summary110: '接获 8 名群众报警称理财产品无法兑付，涉案超 1200 万。',
    graphInfo: '法人名下另一家公司曾有非法吸收公众存款前科。'
  },
  {
    id: 3, date: '2026-04-26 11:38:22', subject: '通州城建劳务集团', industry: '建筑工程',
    confidence: 79, level: 'P2', has12345: true, has110: false, creditCode: '91110112MA02YY1122',
    legalPerson: '李某某', clueType: '工程分包履约异常',
    summary12345: '接收零星包工头投诉，涉及工程款结算纠纷。',
    summary110: '无相关警情记录。',
    graphInfo: '关联上下游 5 家分包商，暂未形成系统性风险网络。'
  },
  {
    id: 4, date: '2026-04-25 16:42:09', subject: '海川商贸服务中心', industry: '商贸流通',
    confidence: 84, level: 'P2', has12345: true, has110: true, creditCode: '91110112MA04AA5566',
    legalPerson: '赵某某', clueType: '虚假交易与合同诈骗',
    summary12345: '多名消费者投诉该商家预付卡跑路，关门停业。',
    summary110: '周边派出所接到 3 起商户寻衅滋事报警。',
    graphInfo: '穿透发现该主体已于一周前密集变更法人和股东。'
  },
  {
    id: 5, date: '2026-04-25 09:16:33', subject: '永顺社区便民服务站', industry: '民生服务',
    confidence: 65, level: 'P2', has12345: true, has110: false, creditCode: '91110112MA06CC9900',
    legalPerson: '陈某某', clueType: '常规舆情波动',
    summary12345: '集中反映社区周边夜间施工噪音扰民问题。',
    summary110: '无相关警情记录。',
    graphInfo: '孤立事件，未发现深层关联风险。'
  }
])

const currentWarning = ref(warnings.value[0])

const filteredWarnings = computed(() => {
  return warnings.value.filter((item) => {
    const hitLevel = !filters.value.level || item.level === filters.value.level
    const keyword = filters.value.subject.trim()
    const hitSubject = !keyword || item.subject.includes(keyword) || item.creditCode.includes(keyword)
    return hitLevel && hitSubject
  })
})

const selectRow = (row) => { currentWarning.value = row }

const handleSearch = () => {
  if(filteredWarnings.value.length > 0 && !filteredWarnings.value.includes(currentWarning.value)){
    currentWarning.value = filteredWarnings.value[0]
  } else if (filteredWarnings.value.length === 0) {
    currentWarning.value = null
  }
}
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

.dim { color: #666; font-size: 12px; }
.subject-cell { font-weight: 800; color: #122E8A; font-size: 14px; }
.industry-cell { font-size: 12px; color: #666; }

/* 风险级别徽章 */
.level-badge { padding: 3px 8px; border-radius: 4px; font-size: 11px; font-weight: 900; font-family: 'JetBrains Mono', sans-serif; }
.P1 { color: #D9363E; background: rgba(217, 54, 62, 0.1); border: 1px solid rgba(217, 54, 62, 0.3); }
.P2 { color: #F5A623; background: rgba(245, 166, 35, 0.1); border: 1px solid rgba(245, 166, 35, 0.3); }

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
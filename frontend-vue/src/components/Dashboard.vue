<template>
  <div class="dashboard-hud">
    <!-- 浅色版背景层，移除了 scanline 和 vignette -->
    <div class="map-bg-layer">
      <div ref="mapChartRef" class="map-container"></div>
    </div>

    <section class="hud-panel left-side">
      <div class="panel-decorator"></div>
      <header class="panel-header">
        <div class="kicker">REAL-TIME INTELLIGENCE</div>
        <h3 class="panel-title">社会治理风险预警情报流</h3>
        <div class="glow-line"></div>
      </header>

      <div class="feed-scroller">
        <div class="block-label">实时接入线索链</div>
        <div class="list-item" v-for="(item, i) in liveFeed" :key="i">
          <span class="timestamp">{{ item.time }}</span>
          <span :class="['tag', item.level]">{{ item.levelText }}</span>
          <span class="msg">{{ item.msg }}</span>
        </div>
      </div>

      <div class="radar-box">
        <div class="sub-label">重点领域风险映射</div>
        <div ref="radarChartRef" class="radar-dom"></div>
      </div>
    </section>

    <section class="hud-panel right-side">
      <div class="panel-decorator"></div>
      <header class="panel-header">
        <div class="kicker">AI ANALYTICS HUB</div>
        <h3 class="panel-title">AI 大模型专项研判战果</h3>
        <div class="glow-line"></div>
      </header>

      <div class="stats-grid">
        <div class="stat-card" v-for="(stat, i) in stats" :key="i">
          <div class="s-label">{{ stat.label }}</div>
          <div class="s-value-row">
            <span class="s-num">{{ stat.value }}</span>
            <span :class="['s-trend', stat.trendUp ? 'up' : 'down']">
              {{ stat.trendUp ? '↑' : '↓' }}{{ stat.percent }}%
            </span>
          </div>
          <div class="s-chart-mini">
            <div class="bar-progress" :style="{ width: stat.progress + '%' }"></div>
          </div>
        </div>
      </div>

      <div class="ai-suggestion-box">
        <div class="box-title"><span class="blink-dot"></span> AI 实时策略建议</div>
        <div class="typewriter-text">
          {{ aiSuggestion }}
        </div>
        <div class="action-bar">
          <span class="action-link">进入知识图谱深挖 ></span>
        </div>
      </div>

      <div class="dual-trend-wrapper">
        <div class="block-label">双域数据时序分布 (110 vs 12345)</div>
        <div ref="lineChartRef" class="line-dom"></div>
      </div>
    </section>

    <footer class="hud-footer">
      <div class="sys-info">
        <i class="el-icon-cpu"></i> 核心模型: OpenAI-Compatible / vLLM | 环境: {{ apiError ? '接口需关注' : '内网部署' }}
      </div>
      <div class="location">坐标: 116.65, 39.91 (北京市通州区指挥中心)</div>
      <div class="timer">{{ formattedTime }}</div>
    </footer>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'
// 引入你本地的通州 GeoJSON 数据
import tongzhouGeoJson from '../assets/maps/tongzhou.json'
import { apiGet } from '../api/platform'

const mapChartRef = ref(null)
const radarChartRef = ref(null)
const lineChartRef = ref(null)
const formattedTime = ref('')
const apiError = ref('')
const aiSuggestion = ref('正在读取后端监督看板与风险研判聚合数据...')

const liveFeed = ref([
  { time: '12:05:22', level: 'crit', levelText: '极高', msg: '梨园镇工地聚集性投诉预警' },
  { time: '11:58:40', level: 'high', levelText: '高危', msg: '某理财点出现异常资金兑付' },
  { time: '11:45:12', level: 'warn', levelText: '中危', msg: '市监局传输涉诉企业名单 122 家' },
  { time: '11:30:05', level: 'warn', levelText: '中危', msg: '运河街道 12345 话务量激增' }
])

const stats = ref([
  { label: '线索识别总数', value: '2,841', trendUp: true, percent: '12', progress: 75 },
  { label: '关联主体穿透', value: '1,055', trendUp: true, percent: '8', progress: 45 },
  { label: '预估风险金额', value: '￥4.2亿', trendUp: false, percent: '3', progress: 85 },
  { label: '模型响应速度', value: '1.2s', trendUp: true, percent: '15', progress: 95 }
])

let mapIns = null
let radarIns = null
let lineIns = null

const metricByKey = (metrics, key) => metrics.find((item) => item.key === key)

const statusLevel = (status, value = 0) => {
  const normalized = String(status || '').toLowerCase()
  if (normalized === 'critical' || Number(value) >= 50) return 'crit'
  if (normalized === 'warning' || Number(value) > 0) return 'high'
  return 'warn'
}

const statusText = (level) => {
  if (level === 'crit') return '极高'
  if (level === 'high') return '高危'
  return '关注'
}

const formatTime = (value) => {
  const date = value ? new Date(value) : new Date()
  return date.toLocaleTimeString('zh-CN', { hour12: false }).slice(0, 8)
}

const loadDashboardData = async () => {
  try {
    const [overview, alerts, risk, jobs] = await Promise.all([
      apiGet('/dashboard/overview'),
      apiGet('/alerts?page_size=4'),
      apiGet('/risk/summary'),
      apiGet('/jobs?page_size=4')
    ])

    const metrics = overview?.metrics || []
    const riskCases = Number(metricByKey(metrics, 'risk_cases')?.value || 0)
    const highRiskCases = Number(metricByKey(metrics, 'high_risk_cases')?.value || 0)
    const pendingAlerts = Number(metricByKey(metrics, 'pending_alerts')?.value || 0)
    const inProgressTasks = Number(metricByKey(metrics, 'in_progress_tasks')?.value || 0)

    stats.value = [
      { label: '风险案件总数', value: String(riskCases), trendUp: true, percent: '实时', progress: Math.min(100, riskCases * 2) },
      { label: '高风险案件', value: String(highRiskCases), trendUp: true, percent: '重点', progress: Math.min(100, highRiskCases * 12) },
      { label: '待处理预警', value: String(pendingAlerts), trendUp: pendingAlerts > 0, percent: '预警', progress: Math.min(100, pendingAlerts * 3) },
      { label: '处置中任务', value: String(inProgressTasks), trendUp: inProgressTasks > 0, percent: '流转', progress: Math.min(100, inProgressTasks * 12) }
    ]

    const alertFeed = (alerts?.items || []).map((item) => {
      const level = statusLevel(item.severity, item.severity === 'high' ? 80 : 20)
      return {
        time: formatTime(item.created_at),
        level,
        levelText: statusText(level),
        msg: item.title?.replace(/^Alert-|^Demo 预警：/, '') || item.summary || '预警线索'
      }
    })
    const jobFeed = (jobs?.items || []).map((item) => ({
      time: formatTime(item.updated_at),
      level: item.status === 'failed' ? 'crit' : item.status === 'completed' ? 'warn' : 'high',
      levelText: item.status === 'failed' ? '异常' : item.status === 'completed' ? '完成' : '运行',
      msg: `${item.job_type}: ${item.message || item.status}`
    }))
    liveFeed.value = [...alertFeed, ...jobFeed].slice(0, 6)

    const riskMetrics = risk?.metrics || []
    const closed = Number(metricByKey(riskMetrics, 'closed')?.value || 0)
    aiSuggestion.value = pendingAlerts > 0
      ? `当前仍有 ${pendingAlerts} 条待处理预警，其中高风险案件 ${highRiskCases} 条；建议优先进入线索审核页确认预警，并对处置中任务做时限监督。`
      : `当前风险案件 ${riskCases} 条，已关闭 ${closed} 条；建议继续通过异构数据接入页补充最新 Excel 批次并触发抽取同步。`

    updateChartsFromBackend(riskMetrics)
    apiError.value = ''
  } catch (error) {
    apiError.value = error.message
    aiSuggestion.value = `后端聚合数据读取失败：${error.message}`
  }
}

const updateChartsFromBackend = (riskMetrics = []) => {
  const total = Number(metricByKey(riskMetrics, 'total')?.value || 1)
  const high = Number(metricByKey(riskMetrics, 'high')?.value || 0)
  const pending = Number(metricByKey(riskMetrics, 'pending_review')?.value || 0)
  const closed = Number(metricByKey(riskMetrics, 'closed')?.value || 0)
  const medium = Math.max(0, total - high - closed)

  radarIns?.setOption({
    series: [{
      data: [{
        value: [
          Math.min(100, high * 12),
          Math.min(100, pending * 8),
          Math.min(100, total * 2),
          Math.min(100, medium * 4),
          Math.min(100, closed * 10)
        ]
      }]
    }]
  })

  lineIns?.setOption({
    series: [
      { data: [high, high + 1, high + 2, high + 1, high + 3, high + 2, high] },
      { data: [total, total + pending, total + pending + 1, total + high, total + high + 2, total + pending, total] }
    ]
  })
}

const initCharts = () => {
  if (mapChartRef.value) {
    mapIns = echarts.init(mapChartRef.value)
    echarts.registerMap('tongzhou', tongzhouGeoJson)
    mapIns.setOption({
      backgroundColor: 'transparent',
      geo: {
        map: 'tongzhou',
        roam: true,
        zoom: 1.1,
        itemStyle: {
          areaColor: '#FFFFFF', // 地图底色纯白
          borderColor: 'rgba(18, 46, 138, 0.3)', // 深海蓝浅边框
          borderWidth: 1.5,
          shadowColor: 'rgba(18, 46, 138, 0.1)',
          shadowBlur: 15
        },
        emphasis: { itemStyle: { areaColor: 'rgba(18, 46, 138, 0.05)' } }
      },
      series: [
        {
          type: 'effectScatter',
          coordinateSystem: 'geo',
          rippleEffect: { brushType: 'stroke', scale: 3.5 },
          itemStyle: { shadowBlur: 5 },
          // 适配亮色底的预警点颜色
          data: [
            // 极高危风险点 (深沉红)
            { name: '梨园镇-聚集性风险', value: [116.656, 39.909], symbolSize: 14, itemStyle: { color: '#D9363E', shadowColor: 'rgba(217, 54, 62, 0.5)' } },
            { name: '马驹桥-劳资纠纷核心区', value: [116.571, 39.759], symbolSize: 15, itemStyle: { color: '#D9363E', shadowColor: 'rgba(217, 54, 62, 0.5)' } },
            { name: '宋庄镇-重点涉诉企业群', value: [116.715, 39.969], symbolSize: 13, itemStyle: { color: '#D9363E', shadowColor: 'rgba(217, 54, 62, 0.5)' } },
            
            // 高危风险点 (亮红/橘红)
            { name: '潞城镇风险点', value: [116.761, 39.906], symbolSize: 10, itemStyle: { color: '#FF4D4F', shadowColor: 'rgba(255, 77, 79, 0.5)' } },
            { name: '台湖镇风险点', value: [116.684, 39.807], symbolSize: 11, itemStyle: { color: '#FF4D4F', shadowColor: 'rgba(255, 77, 79, 0.5)' } },
            { name: '张家湾镇风险点', value: [116.754, 39.852], symbolSize: 9, itemStyle: { color: '#FF4D4F', shadowColor: 'rgba(255, 77, 79, 0.5)' } },
            
            // 中危风险点 (警示橙)
            { name: '永乐店资金异动', value: [116.789, 39.706], symbolSize: 6, itemStyle: { color: '#F5A623', shadowColor: 'rgba(245, 166, 35, 0.5)' } },
            { name: '西集镇资金异动', value: [116.863, 39.861], symbolSize: 7, itemStyle: { color: '#F5A623', shadowColor: 'rgba(245, 166, 35, 0.5)' } },
            { name: '漷县镇资金异动', value: [116.745, 39.790], symbolSize: 6, itemStyle: { color: '#F5A623', shadowColor: 'rgba(245, 166, 35, 0.5)' } },
            
            // 低危监测点 (深海蓝点缀)
            { name: '监测点A', value: [116.695, 39.825], symbolSize: 4, itemStyle: { color: '#122E8A', shadowColor: 'rgba(18, 46, 138, 0.3)' } },
            { name: '监测点B', value: [116.645, 39.930], symbolSize: 5, itemStyle: { color: '#122E8A', shadowColor: 'rgba(18, 46, 138, 0.3)' } },
            { name: '监测点C', value: [116.735, 39.880], symbolSize: 4, itemStyle: { color: '#122E8A', shadowColor: 'rgba(18, 46, 138, 0.3)' } }
          ]
        }
      ]
    })
  }

  if (radarChartRef.value) {
    radarIns = echarts.init(radarChartRef.value)
    radarIns.setOption({
      radar: {
        indicator: [
          { name: '劳资风险', max: 100 },
          { name: '金融诈骗', max: 100 },
          { name: '企业失信', max: 100 },
          { name: '舆情波动', max: 100 },
          { name: '司法纠纷', max: 100 }
        ],
        radius: '65%',
        splitNumber: 4,
        axisName: { color: '#333333', fontSize: 11, fontWeight: 'bold' },
        splitLine: { lineStyle: { color: 'rgba(18, 46, 138, 0.15)' } },
        splitArea: { show: true, areaStyle: { color: ['#F9F9F9', '#FFFFFF'] } }, // 间隔底色
        axisLine: { lineStyle: { color: 'rgba(18, 46, 138, 0.15)' } }
      },
      series: [{
        type: 'radar',
        data: [{
          value: [85, 45, 90, 60, 70],
          areaStyle: { color: 'rgba(18, 46, 138, 0.1)' },
          lineStyle: { color: '#122E8A', width: 2 },
          itemStyle: { color: '#122E8A' }
        }]
      }]
    })
  }

  if (lineChartRef.value) {
    lineIns = echarts.init(lineChartRef.value)
    lineIns.setOption({
      tooltip: {
        trigger: 'axis',
        backgroundColor: 'rgba(255, 255, 255, 0.95)',
        borderColor: '#122E8A',
        textStyle: { color: '#333', fontSize: 12 },
        padding: [8, 12]
      },
      legend: {
        data: ['110 警情', '12345 热线'],
        textStyle: { color: '#666', fontSize: 11, fontWeight: 'bold' },
        top: '0%',
        right: '0%'
      },
      grid: { left: '3%', right: '4%', bottom: '5%', top: '20%', containLabel: true },
      xAxis: {
        type: 'category',
        boundaryGap: false,
        data: ['08:00', '10:00', '12:00', '14:00', '16:00', '18:00', '20:00'],
        axisLine: { lineStyle: { color: 'rgba(18, 46, 138, 0.2)' } },
        axisLabel: { color: '#666', fontSize: 10 }
      },
      yAxis: {
        type: 'value',
        splitLine: { lineStyle: { color: 'rgba(18, 46, 138, 0.08)', type: 'dashed' } },
        axisLabel: { color: '#666', fontSize: 10 }
      },
      series: [
        {
          name: '110 警情',
          type: 'line',
          smooth: true,
          data: [12, 45, 32, 56, 82, 41, 28],
          itemStyle: { color: '#D9363E' },
          lineStyle: { width: 2 },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(217, 54, 62, 0.2)' },
              { offset: 1, color: 'rgba(217, 54, 62, 0)' }
            ])
          }
        },
        {
          name: '12345 热线',
          type: 'line',
          smooth: true,
          data: [85, 120, 98, 145, 180, 130, 95],
          itemStyle: { color: '#122E8A' },
          lineStyle: { width: 2 },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(18, 46, 138, 0.15)' },
              { offset: 1, color: 'rgba(18, 46, 138, 0)' }
            ])
          }
        }
      ]
    })
  }
}

const updateTime = () => {
  const d = new Date()
  formattedTime.value = d.toLocaleTimeString('zh-CN', { hour12: false }) + ':' + String(d.getMilliseconds()).padStart(3, '0')
}

let timer = null
onMounted(() => {
  initCharts()
  loadDashboardData()
  timer = setInterval(updateTime, 100)
  window.addEventListener('resize', () => {
    mapIns?.resize()
    radarIns?.resize()
    lineIns?.resize()
  })
})

onUnmounted(() => { clearInterval(timer) })
</script>

<style scoped>
/* 核心容器：全屏柔奶白 */
.dashboard-hud {
  position: relative;
  width: 100%;
  height: 100vh;
  background-color: #F5EFEA;
  color: #333333;
  overflow: hidden;
  font-family: 'PingFang SC', 'Microsoft YaHei', sans-serif;
}

.map-bg-layer {
  position: absolute; inset: 0; z-index: 1;
}
.map-container { width: 100%; height: 100%; }

/* 悬浮面板：纯白底色，加柔和阴影 */
.hud-panel {
  position: absolute;
  top: 40px;
  bottom: 80px;
  width: 380px;
  z-index: 10;
  background: #FFFFFF;
  border: 1px solid rgba(18, 46, 138, 0.15);
  border-radius: 8px;
  padding: 24px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 30px rgba(18, 46, 138, 0.08);
}
.left-side { left: 40px; }
.right-side { right: 40px; }

/* 标题样式 */
.kicker { font-size: 10px; color: #666; letter-spacing: 1px; font-weight: bold; font-family: 'JetBrains Mono', sans-serif; }
.panel-title { font-size: 18px; margin: 5px 0; font-weight: 900; color: #122E8A; }
.glow-line { width: 40px; height: 3px; background: #122E8A; margin-bottom: 20px; border-radius: 2px; }

/* 模块通用标签 */
.block-label {
  font-size: 12px; font-weight: bold; color: #122E8A;
  background: rgba(18, 46, 138, 0.05);
  padding: 6px 10px; border-radius: 4px;
  border-left: 3px solid #122E8A;
  margin-bottom: 12px; display: flex; align-items: center;
}

/* 情报流 */
.feed-scroller { flex: 1; overflow: hidden; margin-bottom: 20px; }
.list-item { 
  display: flex; gap: 8px; font-size: 13px; padding: 12px 0; 
  border-bottom: 1px dashed rgba(18, 46, 138, 0.1); 
}
.timestamp { color: #666; font-family: 'JetBrains Mono', monospace; font-size: 11px; }
.tag { padding: 2px 6px; border-radius: 4px; font-size: 10px; font-weight: bold; white-space: nowrap; }
.crit { color: #D9363E; background: rgba(217, 54, 62, 0.1); border: 1px solid rgba(217, 54, 62, 0.3); }
.high { color: #F5A623; background: rgba(245, 166, 35, 0.1); border: 1px solid rgba(245, 166, 35, 0.3); }
.warn { color: #122E8A; background: rgba(18, 46, 138, 0.08); border: 1px solid rgba(18, 46, 138, 0.2); }
.msg { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: #333; font-weight: 500; }

/* 雷达图 */
.radar-box { height: 230px; }
.sub-label { font-size: 12px; font-weight: bold; color: #122E8A; border-left: 3px solid #122E8A; padding-left: 8px; margin-bottom: 10px; }
.radar-dom { width: 100%; height: calc(100% - 20px); }

/* 高密度统计网格 */
.stats-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-bottom: 20px; }
.stat-card { background: #F9F9F9; border: 1px solid rgba(18, 46, 138, 0.1); border-radius: 6px; padding: 12px; position: relative; }
.s-label { font-size: 11px; color: #666; font-weight: bold; }
.s-value-row { display: flex; align-items: baseline; justify-content: space-between; margin: 6px 0; }
.s-num { font-size: 20px; font-family: 'JetBrains Mono', monospace; font-weight: 900; color: #122E8A; }
.s-trend { font-size: 11px; font-weight: bold; }
.s-trend.up { color: #D9363E; } /* 明亮模式下上涨多用红色 */
.s-trend.down { color: #52C41A; }
.s-chart-mini { height: 4px; background: rgba(18, 46, 138, 0.05); width: 100%; margin-top: 8px; border-radius: 2px; overflow: hidden; }
.bar-progress { height: 100%; background: #122E8A; border-radius: 2px; }

/* AI 建议面板 */
.ai-suggestion-box { background: rgba(18, 46, 138, 0.03); border: 1px solid rgba(18, 46, 138, 0.15); border-radius: 6px; padding: 15px; display: flex; flex-direction: column; margin-bottom: 15px; }
.box-title { font-size: 12px; font-weight: bold; color: #122E8A; margin-bottom: 10px; display: flex; align-items: center; gap: 6px; }
.blink-dot { width: 8px; height: 8px; background: #D9363E; border-radius: 50%; animation: blink 1.5s infinite; }
.typewriter-text { font-size: 13px; line-height: 1.6; color: #333; font-weight: 500; }
.action-bar { text-align: right; margin-top: 10px; }
.action-link { font-size: 12px; font-weight: bold; color: #122E8A; cursor: pointer; text-decoration: underline; }

/* 对比折线图包装 */
.dual-trend-wrapper { flex: 1; min-height: 180px; display: flex; flex-direction: column; }
.line-dom { flex: 1; width: 100%; }

/* 底部栏 */
.hud-footer {
  position: absolute; bottom: 0; width: 100%; height: 45px;
  background: #FFFFFF; border-top: 1px solid rgba(18, 46, 138, 0.15);
  display: flex; align-items: center; justify-content: space-between; padding: 0 40px;
  z-index: 100; font-size: 12px; color: #666; font-weight: bold;
}
.timer { font-family: 'JetBrains Mono', monospace; color: #122E8A; font-size: 14px; font-weight: 900; }

/* 四角装饰：换成深海蓝实心小方块角标，更具科研和严谨感 */
.panel-decorator::before, .panel-decorator::after {
  content: ''; position: absolute; width: 12px; height: 12px; border: 3px solid #122E8A; border-radius: 2px;
}
.left-side .panel-decorator::before { top: -1px; left: -1px; border-right: 0; border-bottom: 0; }
.left-side .panel-decorator::after { bottom: -1px; right: -1px; border-left: 0; border-top: 0; }
.right-side .panel-decorator::before { top: -1px; right: -1px; border-left: 0; border-bottom: 0; }
.right-side .panel-decorator::after { bottom: -1px; left: -1px; border-right: 0; border-top: 0; }

@keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
</style>

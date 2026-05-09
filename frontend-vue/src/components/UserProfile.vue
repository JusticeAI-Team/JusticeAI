<template>
  <div class="profile-page">
    <header class="page-top">
      <div>
        <div class="kicker">PROSECUTOR TERMINAL</div>
        <h2>个人中心</h2>
      </div>
      <div class="terminal-chip">TEST-USER / 内网演示账号</div>
    </header>

    <main class="profile-grid">
      <section class="panel identity-panel">
        <div class="avatar-large">检</div>
        <div class="identity-main">
          <div class="name">测试用户</div>
          <div class="role">通州区检察院 · 社会治理风险研判员</div>
          <div class="tags">
            <span>风险研判</span>
            <span>线索审核</span>
            <span>报告生成</span>
          </div>
        </div>
      </section>

      <section class="panel">
        <div class="panel-header">
          <span class="bar"></span>
          <div>
            <div class="kicker">ACCOUNT PROFILE</div>
            <h3>账号信息</h3>
          </div>
        </div>
        <div class="info-list">
          <div><span>账号标识</span><strong>prosecutor.test</strong></div>
          <div><span>所属单位</span><strong>北京市通州区人民检察院</strong></div>
          <div><span>当前角色</span><strong>业务研判 / 系统调试</strong></div>
          <div><span>认证状态</span><strong class="green">本地演示终端</strong></div>
        </div>
      </section>

      <section class="panel">
        <div class="panel-header">
          <span class="bar red"></span>
          <div>
            <div class="kicker">PERMISSIONS</div>
            <h3>功能权限</h3>
          </div>
        </div>
        <div class="permission-grid">
          <div v-for="item in permissions" :key="item.name" class="permission-item">
            <span class="dot"></span>
            <div>
              <strong>{{ item.name }}</strong>
              <p>{{ item.desc }}</p>
            </div>
          </div>
        </div>
      </section>

      <section class="panel activity-panel">
        <div class="panel-header">
          <span class="bar orange"></span>
          <div>
            <div class="kicker">RECENT ACTIVITY</div>
            <h3>近期操作轨迹</h3>
          </div>
        </div>
        <div class="timeline">
          <div v-for="item in activities" :key="item.time" class="timeline-item">
            <span class="time">{{ item.time }}</span>
            <span class="event">{{ item.event }}</span>
            <span :class="['state', item.state]">{{ item.label }}</span>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup>
const permissions = [
  { name: '异构数据接入', desc: '上传 12345、110、信访、395 等多源表格并触发处理链路。' },
  { name: '风险线索审核', desc: '查看案件详情、确认预警、调整研判状态和处置建议。' },
  { name: '文书辅助生成', desc: '基于案件统计、处置反馈和 AI 摘要生成阶段报告。' },
  { name: '技术运维后台', desc: '查看 vLLM、Embedding、HugeGraph、Milvus 的联通状态。' }
]

const activities = [
  { time: '09:20', event: '查看全景风险指挥大屏', state: 'ok', label: '完成' },
  { time: '09:35', event: '进入异构数据接入流水线', state: 'ok', label: '完成' },
  { time: '10:05', event: '检查模型与图谱服务配置', state: 'warn', label: '待联调' },
  { time: '10:30', event: '准备导入通州 4 月 8 日样例数据', state: 'ok', label: '就绪' }
]
</script>

<style scoped>
.profile-page { height: 94vh; background: #F5EFEA; padding: 26px 30px; box-sizing: border-box; overflow: auto; color: #333; font-family: 'PingFang SC', 'Microsoft YaHei', sans-serif; }
.page-top { display: flex; align-items: center; justify-content: space-between; margin-bottom: 18px; }
.kicker { font-family: 'JetBrains Mono', Consolas, monospace; font-size: 11px; color: #8C98B0; font-weight: 900; letter-spacing: 1px; }
h2, h3 { margin: 4px 0 0; color: #122E8A; letter-spacing: 1px; }
.terminal-chip { border: 1px solid rgba(18, 46, 138, 0.2); background: #FFFFFF; border-radius: 999px; padding: 8px 14px; color: #122E8A; font-family: 'JetBrains Mono', Consolas, monospace; font-size: 12px; font-weight: 900; }
.profile-grid { display: grid; grid-template-columns: 1.1fr 1fr; gap: 16px; }
.panel { background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.14); border-radius: 8px; padding: 20px; box-shadow: 0 6px 18px rgba(18, 46, 138, 0.05); }
.identity-panel { grid-column: 1 / -1; display: flex; align-items: center; gap: 22px; background: linear-gradient(135deg, #FFFFFF 0%, #F7F9FF 100%); }
.avatar-large { width: 86px; height: 86px; border-radius: 12px; border: 2px solid #122E8A; background: rgba(18, 46, 138, 0.08); color: #122E8A; font-size: 34px; font-weight: 900; display: flex; align-items: center; justify-content: center; }
.name { font-size: 26px; color: #122E8A; font-weight: 900; }
.role { margin-top: 6px; color: #666; font-weight: bold; }
.tags { display: flex; gap: 8px; margin-top: 14px; }
.tags span { background: rgba(18, 46, 138, 0.08); color: #122E8A; border: 1px solid rgba(18, 46, 138, 0.14); border-radius: 999px; padding: 5px 10px; font-size: 12px; font-weight: bold; }
.panel-header { display: flex; gap: 10px; align-items: center; margin-bottom: 16px; }
.bar { width: 4px; height: 30px; background: #122E8A; border-radius: 2px; }
.bar.red { background: #D9363E; }
.bar.orange { background: #F5A623; }
.info-list { display: grid; gap: 12px; }
.info-list div { display: flex; justify-content: space-between; border-bottom: 1px solid rgba(18, 46, 138, 0.08); padding-bottom: 10px; font-size: 13px; }
.info-list span { color: #666; }
.info-list strong { color: #333; }
.green { color: #0F7E3B !important; }
.permission-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
.permission-item { display: flex; gap: 10px; padding: 12px; border: 1px solid rgba(18, 46, 138, 0.1); border-radius: 6px; background: #FAFAFA; }
.permission-item strong { color: #122E8A; }
.permission-item p { margin: 5px 0 0; color: #666; font-size: 12px; line-height: 1.6; }
.dot { width: 8px; height: 8px; border-radius: 50%; background: #52C41A; margin-top: 6px; flex-shrink: 0; }
.activity-panel { grid-column: 1 / -1; }
.timeline { display: grid; gap: 10px; }
.timeline-item { display: grid; grid-template-columns: 90px 1fr 80px; align-items: center; padding: 12px; background: #FAFAFA; border: 1px solid rgba(18, 46, 138, 0.08); border-radius: 6px; }
.time { color: #122E8A; font-family: 'JetBrains Mono', Consolas, monospace; font-weight: 900; }
.event { color: #333; font-weight: bold; }
.state { justify-self: end; font-size: 12px; font-weight: 900; }
.state.ok { color: #0F7E3B; }
.state.warn { color: #F5A623; }
@media (max-width: 1080px) { .profile-grid { grid-template-columns: 1fr; } .permission-grid { grid-template-columns: 1fr; } }
</style>

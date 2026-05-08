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
        <button class="hud-btn primary">
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
import { ref, onBeforeUnmount } from 'vue'

const caseQueue = ref([
  {
    id: 1, shortId: 'CA-2026-0426', subject: '华丰建设有限公司', code: '91110112MA01XX8899',
    level: 'crit', levelText: '极高危', type: '欠薪 / 非法集资双重风险',
    trace12345: '欠薪投诉 47 人次，涉案金额核算约 312 万元。', trace110: '非法集资及劳资纠纷聚集警情，关联 86 人。',
    traceGraph: '资金池穿透发现向法人名下关联账户高频转移资产。',
    baseDraft: '《检察建议书（草稿）》\n\n经查，华丰建设有限公司存在严重拖欠农民工工资行为，相关信访平台累计接收欠薪投诉 47 人次，涉及金额 312 万元。根据多源数据交叉比对结果，该主体在劳动关系管理、工程款支付链条及项目资金监管方面存在明显异常。\n\n特此建议。'
  }
])

const activeCase = ref(caseQueue.value[0])
const draftContent = ref(activeCase.value.baseDraft)
const isGenerating = ref(false)
let typingTimer = null

const selectCase = (c) => {
  if (isGenerating.value) return
  activeCase.value = c
  draftContent.value = c.baseDraft
}

const handleRegenerate = () => {
  if (isGenerating.value) return
  isGenerating.value = true
  draftContent.value = '正在根据深海蓝主题重新生成文书排版...'
  setTimeout(() => { isGenerating.value = false; draftContent.value = activeCase.value.baseDraft + '\n\n【已应用智能校对与最新法条】' }, 1500)
}
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
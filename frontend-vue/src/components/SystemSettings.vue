<template>
  <div class="settings-page">
    <header class="page-top">
      <div>
        <div class="kicker">PLATFORM SETTINGS</div>
        <h2>系统设置</h2>
      </div>
      <div class="actions">
        <button class="ghost-btn" @click="loadSettings" :disabled="loading">刷新配置</button>
        <button class="primary-btn" @click="saveAll" :disabled="saving">{{ saving ? '保存中...' : '保存配置' }}</button>
      </div>
    </header>

    <div v-if="message" :class="['notice', messageType]">{{ message }}</div>

    <main class="settings-grid">
      <section class="panel">
        <div class="panel-header">
          <span class="bar"></span>
          <div>
            <div class="kicker">BASIC</div>
            <h3>平台基础配置</h3>
          </div>
        </div>
        <div class="form-grid">
          <label>
            <span>平台名称</span>
            <input v-model="platformForm.platform_name" />
          </label>
          <label>
            <span>运行环境</span>
            <input v-model="platformForm.environment" />
          </label>
          <label>
            <span>上传目录</span>
            <input v-model="platformForm.upload_dir" />
          </label>
          <label>
            <span>报告目录</span>
            <input v-model="platformForm.report_dir" />
          </label>
          <label>
            <span>训练资料目录</span>
            <input v-model="platformForm.training_dir" />
          </label>
        </div>
      </section>

      <section class="panel">
        <div class="panel-header">
          <span class="bar red"></span>
          <div>
            <div class="kicker">MODEL</div>
            <h3>OpenAI-Compatible 模型配置</h3>
          </div>
        </div>
        <div class="form-grid">
          <label>
            <span>Chat Base URL</span>
            <input v-model="integrationForm.model_base_url" placeholder="http://host.docker.internal:8000/v1" />
          </label>
          <label>
            <span>Chat Model</span>
            <input v-model="integrationForm.model_name" placeholder="Qwen2.5-Coder-7B-Instruct" />
          </label>
          <label>
            <span>Chat Endpoint</span>
            <input v-model="integrationForm.model_chat_endpoint" placeholder="/chat/completions" />
          </label>
          <label>
            <span>API Key</span>
            <input v-model="integrationForm.openai_api_key" type="password" placeholder="本地 vLLM 可留空" />
          </label>
        </div>
      </section>

      <section class="panel">
        <div class="panel-header">
          <span class="bar orange"></span>
          <div>
            <div class="kicker">GRAPH / VECTOR</div>
            <h3>图谱与向量服务</h3>
          </div>
        </div>
        <div class="form-grid">
          <label>
            <span>HugeGraph Base URL</span>
            <input v-model="integrationForm.hugegraph_base_url" placeholder="http://host.docker.internal:8080" />
          </label>
          <label>
            <span>Milvus Address</span>
            <input v-model="integrationForm.milvus_address" placeholder="http://host.docker.internal:19530" />
          </label>
          <label>
            <span>Milvus Token</span>
            <input v-model="integrationForm.milvus_token" type="password" placeholder="root:Milvus" />
          </label>
          <label>
            <span>Milvus Collection</span>
            <input v-model="integrationForm.milvus_collection" placeholder="justiceai_cases" />
          </label>
        </div>
      </section>

      <section class="panel">
        <div class="panel-header">
          <span class="bar green"></span>
          <div>
            <div class="kicker">EMBEDDING</div>
            <h3>向量化模型服务</h3>
          </div>
        </div>
        <div class="form-grid">
          <label>
            <span>Embedding Base URL</span>
            <input v-model="integrationForm.embedding_base_url" placeholder="http://host.docker.internal:7997/v1" />
          </label>
          <label>
            <span>Embedding Endpoint</span>
            <input v-model="integrationForm.embedding_endpoint" placeholder="/embeddings" />
          </label>
          <label>
            <span>Embedding Model</span>
            <input v-model="integrationForm.embedding_model" placeholder="BAAI/bge-small-zh-v1.5" />
          </label>
          <label>
            <span>Embedding API Key</span>
            <input v-model="integrationForm.embedding_api_key" type="password" placeholder="可选" />
          </label>
        </div>
      </section>

      <section class="panel status-panel">
        <div class="panel-header">
          <span class="bar"></span>
          <div>
            <div class="kicker">CONNECTION TEST</div>
            <h3>当前连接状态</h3>
          </div>
        </div>
        <button class="primary-btn wide" @click="testIntegrations" :disabled="testing">
          {{ testing ? '测试中...' : '测试外部服务连接' }}
        </button>
        <div class="status-grid">
          <div v-for="item in statusItems" :key="item.key" class="status-item">
            <div>
              <strong>{{ item.label }}</strong>
              <p>{{ item.endpoint }}</p>
            </div>
            <span :class="['badge', item.className]">{{ item.text }}</span>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { apiGet, apiPost, statusClass, statusText } from '../api/platform'

const loading = ref(false)
const saving = ref(false)
const testing = ref(false)
const message = ref('')
const messageType = ref('ok')
const integrationStatus = ref(null)

const platformForm = ref({
  platform_name: '',
  environment: '',
  upload_dir: '',
  report_dir: '',
  training_dir: ''
})

const integrationForm = ref({
  hugegraph_base_url: '',
  hugegraph_gremlin_url: '',
  milvus_address: '',
  milvus_token: '',
  milvus_collection: 'justiceai_cases',
  model_base_url: '',
  model_name: '',
  model_request_style: 'openai_chat_completion_compatible',
  model_chat_endpoint: '/chat/completions',
  model_json_mode_supported: true,
  openai_api_key: '',
  embedding_base_url: '',
  embedding_model: '',
  embedding_api_key: '',
  embedding_endpoint: '/embeddings'
})

const showMessage = (text, type = 'ok') => {
  message.value = text
  messageType.value = type
}

const applyPlatform = (data) => {
  platformForm.value.platform_name = data?.platform?.app_name || '数智检察预警平台'
  platformForm.value.environment = data?.platform?.environment || 'development'
  platformForm.value.upload_dir = data?.storage?.upload_dir || ''
  platformForm.value.report_dir = data?.storage?.report_dir || ''
  platformForm.value.training_dir = data?.storage?.training_dir || ''
}

const applyIntegrations = (data) => {
  integrationStatus.value = data
  integrationForm.value.hugegraph_base_url = data?.hugegraph?.endpoint || integrationForm.value.hugegraph_base_url
  integrationForm.value.milvus_address = data?.milvus?.endpoint || integrationForm.value.milvus_address
  integrationForm.value.model_base_url = data?.model_service?.endpoint || integrationForm.value.model_base_url
  integrationForm.value.model_name = data?.model_service?.model || integrationForm.value.model_name
  integrationForm.value.model_chat_endpoint = data?.model_service?.chat_endpoint || integrationForm.value.model_chat_endpoint
  integrationForm.value.embedding_base_url = data?.embedding_service?.endpoint || integrationForm.value.embedding_base_url
  integrationForm.value.embedding_model = data?.embedding_service?.model || integrationForm.value.embedding_model
  integrationForm.value.embedding_endpoint = data?.embedding_service?.chat_endpoint || integrationForm.value.embedding_endpoint
}

const loadSettings = async () => {
  loading.value = true
  try {
    const [platform, integrations] = await Promise.all([
      apiGet('/settings/platform'),
      apiGet('/settings/integrations')
    ])
    applyPlatform(platform)
    applyIntegrations(integrations)
    showMessage('配置已从后端刷新。')
  } catch (error) {
    showMessage(error.message, 'bad')
  } finally {
    loading.value = false
  }
}

const saveAll = async () => {
  saving.value = true
  try {
    const [platform, integrations] = await Promise.all([
      apiPost('/settings/platform', platformForm.value),
      apiPost('/settings/integrations', integrationForm.value)
    ])
    applyPlatform(platform)
    applyIntegrations(integrations)
    showMessage('配置已保存。')
  } catch (error) {
    showMessage(error.message, 'bad')
  } finally {
    saving.value = false
  }
}

const testIntegrations = async () => {
  testing.value = true
  try {
    const result = await apiPost('/settings/integrations/test', integrationForm.value)
    integrationStatus.value = {
      ...integrationStatus.value,
      ...result,
      database: integrationStatus.value?.database
    }
    showMessage('连接测试已完成。')
  } catch (error) {
    showMessage(error.message, 'bad')
  } finally {
    testing.value = false
  }
}

const statusItems = computed(() => {
  const data = integrationStatus.value || {}
  return [
    { label: 'PostgreSQL', key: 'database', item: data.database },
    { label: 'HugeGraph', key: 'hugegraph', item: data.hugegraph },
    { label: 'Milvus', key: 'milvus', item: data.milvus },
    { label: 'vLLM ChatCompletion', key: 'model_service', item: data.model_service },
    { label: 'Embedding Service', key: 'embedding_service', item: data.embedding_service }
  ].map(({ label, key, item }) => ({
    key,
    label,
    endpoint: item?.endpoint || '--',
    text: statusText(item?.status),
    className: statusClass(item?.status)
  }))
})

onMounted(loadSettings)
</script>

<style scoped>
.settings-page { height: 94vh; background: #F5EFEA; padding: 26px 30px; box-sizing: border-box; overflow: auto; color: #333; font-family: 'PingFang SC', 'Microsoft YaHei', sans-serif; }
.page-top { display: flex; align-items: center; justify-content: space-between; margin-bottom: 14px; }
.kicker { font-family: 'JetBrains Mono', Consolas, monospace; font-size: 11px; color: #8C98B0; font-weight: 900; letter-spacing: 1px; }
h2, h3 { margin: 4px 0 0; color: #122E8A; letter-spacing: 1px; }
.actions { display: flex; gap: 10px; }
button { height: 36px; border-radius: 6px; padding: 0 14px; font-weight: 900; cursor: pointer; }
button:disabled { opacity: 0.65; cursor: not-allowed; }
.primary-btn { border: 1px solid #122E8A; background: #122E8A; color: #FFFFFF; }
.ghost-btn { border: 1px solid rgba(18, 46, 138, 0.22); background: #FFFFFF; color: #122E8A; }
.wide { width: 100%; margin-bottom: 14px; }
.notice { margin-bottom: 14px; padding: 10px 14px; border-radius: 6px; font-size: 12px; font-weight: bold; }
.notice.ok { background: rgba(82, 196, 26, 0.08); border: 1px solid rgba(82, 196, 26, 0.2); color: #0F7E3B; }
.notice.bad { background: rgba(217, 54, 62, 0.08); border: 1px solid rgba(217, 54, 62, 0.2); color: #D9363E; }
.settings-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
.panel { background: #FFFFFF; border: 1px solid rgba(18, 46, 138, 0.14); border-radius: 8px; padding: 18px; box-shadow: 0 6px 18px rgba(18, 46, 138, 0.05); }
.status-panel { grid-column: 1 / -1; }
.panel-header { display: flex; gap: 10px; align-items: center; margin-bottom: 16px; }
.bar { width: 4px; height: 30px; background: #122E8A; border-radius: 2px; }
.bar.red { background: #D9363E; }
.bar.orange { background: #F5A623; }
.bar.green { background: #52C41A; }
.form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
label { display: grid; gap: 6px; font-size: 12px; color: #666; font-weight: bold; }
input { height: 34px; border: 1px solid rgba(18, 46, 138, 0.16); border-radius: 6px; padding: 0 10px; outline: none; color: #333; background: #FAFAFA; font-family: 'JetBrains Mono', Consolas, monospace; }
input:focus { border-color: #122E8A; background: #FFFFFF; }
.status-grid { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px; }
.status-item { min-height: 88px; display: flex; flex-direction: column; justify-content: space-between; background: #FAFAFA; border: 1px solid rgba(18, 46, 138, 0.08); border-radius: 6px; padding: 12px; }
.status-item strong { color: #122E8A; }
.status-item p { margin: 6px 0 0; color: #666; font-size: 11px; word-break: break-all; font-family: 'JetBrains Mono', Consolas, monospace; }
.badge { align-self: flex-start; margin-top: 12px; border-radius: 999px; padding: 4px 9px; font-size: 11px; font-weight: 900; }
.badge.ok { background: rgba(82, 196, 26, 0.12); color: #0F7E3B; }
.badge.warn { background: rgba(245, 166, 35, 0.14); color: #B56B00; }
.badge.bad { background: rgba(217, 54, 62, 0.12); color: #D9363E; }
@media (max-width: 1180px) { .settings-grid, .form-grid { grid-template-columns: 1fr; } .status-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
</style>

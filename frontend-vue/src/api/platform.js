const API_BASE = import.meta.env.VITE_API_BASE_URL || '/api'

const buildUrl = (path) => `${API_BASE}${path.startsWith('/') ? path : `/${path}`}`

async function request(path, options = {}) {
  const isFormData = options.body instanceof FormData
  const response = await fetch(buildUrl(path), {
    ...options,
    headers: {
      ...(isFormData ? {} : { 'Content-Type': 'application/json' }),
      ...(options.headers || {})
    }
  })

  const payload = await response.json().catch(() => null)
  if (!response.ok || payload?.success === false) {
    const message = payload?.message || payload?.error || response.statusText || '接口请求失败'
    throw new Error(message)
  }

  return payload?.data ?? payload
}

export const apiGet = (path) => request(path)

export const apiPost = (path, body = {}) =>
  request(path, {
    method: 'POST',
    body: JSON.stringify(body)
  })

export const apiDownloadUrl = (path) => buildUrl(path)

export const apiUploadImport = (file) => {
  const formData = new FormData()
  formData.append('file', file)
  return request('/ingestion/upload', {
    method: 'POST',
    body: formData
  })
}

export const statusText = (status) => {
  const normalized = String(status || '').toLowerCase()
  if (['ok', 'up', 'healthy', 'processed', 'indexed', 'indexed_demo', 'synced', 'completed', 'completed_with_warnings'].includes(normalized)) {
    return '正常'
  }
  if (['queued', 'running', 'processing', 'configured', 'not_checked', 'uploaded', 'mapping_pending', 'ready_to_process', 'pending_extraction'].includes(normalized)) {
    return '待验证'
  }
  if (['degraded', 'warning', 'open', 'pending_review'].includes(normalized)) {
    return '关注'
  }
  if (['down', 'failed', 'critical', 'not_configured'].includes(normalized)) {
    return '异常'
  }
  return status || '未知'
}

export const statusClass = (status) => {
  const normalized = String(status || '').toLowerCase()
  if (['ok', 'up', 'healthy', 'processed', 'indexed', 'indexed_demo', 'synced', 'completed', 'completed_with_warnings'].includes(normalized)) {
    return 'ok'
  }
  if (['queued', 'running', 'processing', 'configured', 'not_checked', 'uploaded', 'mapping_pending', 'ready_to_process', 'pending_extraction'].includes(normalized)) {
    return 'warn'
  }
  if (['down', 'failed', 'critical', 'not_configured'].includes(normalized)) {
    return 'bad'
  }
  return 'warn'
}

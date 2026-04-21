const statusLabelMap: Record<string, string> = {
  healthy: '健康',
  ready: '就绪',
  completed: '已完成',
  configured: '已配置',
  running: '运行中',
  open: '开启',
  warning: '预警',
  degraded: '降级',
  draft: '草稿',
  todo: '待开始',
  in_progress: '进行中',
  acknowledged: '已确认',
  attention: '需关注',
  critical: '严重',
  uploaded: '已上传',
  success: '成功',
  pending: '待处理',
  failed: '失败',
  error: '异常',
  disconnected: '未接通',
}

export function formatDateTime(value?: string | null) {
  if (!value) {
    return '-'
  }

  const date = new Date(value)

  if (Number.isNaN(date.getTime())) {
    return value
  }

  return date.toLocaleString('zh-CN', {
    hour12: false,
  })
}

export function formatNumber(value?: number | null) {
  if (value == null || Number.isNaN(value)) {
    return '-'
  }

  return new Intl.NumberFormat('zh-CN').format(value)
}

export function formatMetricValue(value: string | number, unit?: string | null) {
  const rawValue = String(value)

  if (!unit) {
    return rawValue
  }

  if (['%', 'ms', 's', 'MB', 'GB', 'TB'].includes(unit)) {
    return `${rawValue}${unit}`
  }

  return `${rawValue} ${unit}`
}

export function formatStatusLabel(value?: string | null) {
  if (!value) {
    return '未知'
  }

  const normalized = value.trim().toLowerCase()

  if (statusLabelMap[normalized]) {
    return statusLabelMap[normalized]
  }

  return value.replace(/[_-]+/g, ' ')
}

export function resolveTone(value?: string | null) {
  const normalized = (value ?? '').trim().toLowerCase()
  const chineseValue = (value ?? '').trim()

  if (
    ['healthy', 'ready', 'completed', 'configured', 'success', 'open', 'up', 'generated'].includes(normalized) ||
    ['健康', '就绪', '已完成', '已配置', '成功', '开启', '已连接'].includes(chineseValue)
  ) {
    return 'good'
  }

  if (
    [
      'warning',
      'degraded',
      'draft',
      'todo',
      'attention',
      'pending',
      'queued',
      'running',
      'in_progress',
      'acknowledged',
      'uploaded',
    ].includes(normalized) ||
    ['预警', '降级', '草稿', '待开始', '需关注', '待处理', '运行中', '进行中', '已确认', '已上传'].includes(chineseValue)
  ) {
    return 'warning'
  }

  if (
    ['critical', 'failed', 'error', 'down', 'disconnected'].includes(normalized) ||
    ['严重', '失败', '异常', '未接通'].includes(chineseValue)
  ) {
    return 'danger'
  }

  return 'neutral'
}

export function formatConfidence(value?: number | null) {
  if (value == null || Number.isNaN(value)) {
    return '-'
  }

  const normalized = value <= 1 ? value * 100 : value
  return `${normalized.toFixed(1)}%`
}

export function formatFileSize(value?: number | null) {
  if (value == null || Number.isNaN(value)) {
    return '-'
  }

  if (value < 1024) {
    return `${value} B`
  }

  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`
  }

  return `${(value / 1024 / 1024).toFixed(2)} MB`
}

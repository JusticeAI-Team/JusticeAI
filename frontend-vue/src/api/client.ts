const DEFAULT_API_BASE_URL = '/api'

const rawBaseUrl = (import.meta.env.VITE_API_BASE_URL ?? '').trim()
const normalizedBaseUrl = rawBaseUrl || DEFAULT_API_BASE_URL

const BASE_URL = normalizedBaseUrl.endsWith('/')
  ? normalizedBaseUrl.slice(0, -1)
  : normalizedBaseUrl

interface ApiResponse<T> {
  success: boolean
  code: string
  message: string
  data: T
}

interface ApiErrorResponse {
  success?: boolean
  code?: string
  message?: string
  details?: string
}

function resolveErrorMessage(payload: ApiErrorResponse | null, status: number) {
  const message = payload?.details ?? payload?.message

  if (payload?.code && message) {
    return `${payload.code}: ${message}`
  }

  return message ?? `HTTP ${status}`
}

function resolveNetworkErrorMessage(error: unknown) {
  if (error instanceof Error && error.name === 'AbortError') {
    return '请求已取消'
  }

  return '网络请求失败，请检查前端代理或后端服务'
}

export async function requestJson<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers ?? {})
  const isFormData = init.body instanceof FormData

  if (!isFormData && init.body != null && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json')
  }

  let response: Response

  try {
    response = await fetch(`${BASE_URL}${path}`, {
      ...init,
      headers,
    })
  } catch (error) {
    throw new Error(resolveNetworkErrorMessage(error))
  }

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }

  return response.json() as Promise<T>
}

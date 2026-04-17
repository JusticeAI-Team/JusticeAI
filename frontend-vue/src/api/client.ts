const DEFAULT_API_BASE_URL = '/api'

const rawBaseUrl = (import.meta.env.VITE_API_BASE_URL ?? '').trim()
const normalizedBaseUrl = rawBaseUrl || DEFAULT_API_BASE_URL

const BASE_URL = normalizedBaseUrl.endsWith('/')
  ? normalizedBaseUrl.slice(0, -1)
  : normalizedBaseUrl

export async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${BASE_URL}${path}`, {
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    ...init,
  })

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }

  return response.json() as Promise<T>
}

import { requestJson } from './client'

export interface ImportListParams {
  page?: number
  pageSize?: number
  status?: string
}

export interface ImportListItem {
  id: string
  source_type: string
  status: string
  created_at: string
  updated_at: string
}

export interface ImportListResponse {
  items: ImportListItem[]
  page: number
  page_size: number
  total: number
}

export async function fetchImportList(params: ImportListParams = {}) {
  const searchParams = new URLSearchParams({
    page: String(params.page ?? 1),
    page_size: String(params.pageSize ?? 20),
  })

  if (params.status) {
    searchParams.set('status', params.status)
  }

  return requestJson<ImportListResponse>(`/imports?${searchParams.toString()}`)
}

export async function fetchImportDetail(importId: string) {
  return requestJson(`/api/imports/${importId}`)
}

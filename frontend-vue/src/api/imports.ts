import { requestJson } from './client'

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

export async function fetchImportList() {
  return requestJson<ImportListResponse>('/api/imports?page=1&page_size=20')
}

export async function fetchImportDetail(importId: string) {
  return requestJson(`/api/imports/${importId}`)
}

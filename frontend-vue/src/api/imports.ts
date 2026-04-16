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

export interface UploadedImportFile {
  id: string
  original_filename: string
  stored_filename: string
  stored_path: string
  file_size: number
  mime_type: string | null
}

export interface ImportDetailFileItem extends UploadedImportFile {
  created_at: string
}

export interface ImportListResponse {
  items: ImportListItem[]
  page: number
  page_size: number
  total: number
}

export interface ImportDetailResponse {
  id: string
  source_type: string
  status: string
  created_at: string
  updated_at: string
  files: ImportDetailFileItem[]
}

export interface UploadImportResponse {
  import_id: string
  status: string
  file: UploadedImportFile
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
  return requestJson<ImportDetailResponse>(`/imports/${importId}`)
}

export async function uploadImportFile(file: File) {
  const body = new FormData()
  body.append('file', file)

  return requestJson<UploadImportResponse>('/import/upload', {
    method: 'POST',
    body,
  })
}

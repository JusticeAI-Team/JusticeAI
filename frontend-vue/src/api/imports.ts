import { requestJson } from './client'
import type {
  DashboardOverviewResponse,
  IngestionBatchListResponse,
  IngestionSummaryResponse,
  MappingCurrentResponse,
  ExtractionSummaryResponse,
  GraphOverviewResponse,
  DispatchTaskListResponse,
  EvaluationSummaryResponse,
} from '../types/workspace'

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

export async function fetchIngestionSummary() {
  return requestJson<IngestionSummaryResponse>('/ingestion/summary')
}

export async function fetchIngestionBatches() {
  return requestJson<IngestionBatchListResponse>('/ingestion/batches')
}

export async function fetchMappingCurrent() {
  return requestJson<MappingCurrentResponse>('/mapping/current')
}

export async function fetchExtractionSummary() {
  return requestJson<ExtractionSummaryResponse>('/extraction/summary')
}

export async function fetchGraphOverview() {
  return requestJson<GraphOverviewResponse>('/graph/overview')
}

export async function fetchDispatchTasks() {
  return requestJson<DispatchTaskListResponse>('/dispatch/tasks')
}

export async function fetchEvaluationSummary() {
  return requestJson<EvaluationSummaryResponse>('/evaluation/summary')
}

export async function fetchHomeOverview() {
  return requestJson<DashboardOverviewResponse>('/dashboard/overview')
}

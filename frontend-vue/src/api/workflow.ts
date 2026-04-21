import { requestJson } from './client'
import type {
  DispatchTaskListResponse,
  EvaluationSummaryResponse,
  ExtractionSummaryResponse,
  GraphOverviewResponse,
  IngestionBatchListResponse,
  IngestionSummaryResponse,
  MappingCurrentResponse,
} from '../types/workspace'

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

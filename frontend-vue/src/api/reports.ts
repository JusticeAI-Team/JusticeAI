import { requestJson } from './client'
import type { ReportListResponse } from '../types/workspace'

export async function fetchReports() {
  return requestJson<ReportListResponse>('/reports')
}

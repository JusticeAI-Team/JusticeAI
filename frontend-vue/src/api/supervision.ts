import { requestJson } from './client'
import type { SupervisionOverviewResponse } from '../types/workspace'

export async function fetchSupervisionOverview() {
  return requestJson<SupervisionOverviewResponse>('/supervision/overview')
}

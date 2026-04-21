import { requestJson } from './client'
import type { RiskOverviewResponse } from '../types/workspace'

export async function fetchRiskOverview() {
  return requestJson<RiskOverviewResponse>('/risk/overview')
}

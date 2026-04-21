import { requestJson } from './client'
import type { DashboardOverviewResponse } from '../types/workspace'

export async function fetchDashboardOverview() {
  return requestJson<DashboardOverviewResponse>('/dashboard/overview')
}

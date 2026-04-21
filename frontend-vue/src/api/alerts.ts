import { requestJson } from './client'
import type { AlertsSummaryResponse } from '../types/workspace'

export async function fetchAlertsSummary() {
  return requestJson<AlertsSummaryResponse>('/alerts/summary')
}

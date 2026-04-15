import { requestJson } from './client'

export async function fetchImportList() {
  return requestJson('/api/imports?page=1&page_size=20')
}

export async function fetchImportDetail(importId: string) {
  return requestJson(`/api/imports/${importId}`)
}

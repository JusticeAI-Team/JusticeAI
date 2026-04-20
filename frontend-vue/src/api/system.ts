import { requestJson } from './client'

export interface HealthDependencyStatuses {
  postgres: string
  hugegraph: string
  vllm: string
  milvus: string
}

export interface HealthResponse {
  status: string
  service: string
  app_env: string
  timestamp: string
  dependencies: HealthDependencyStatuses
  notes: string[]
}

export interface SystemAppInfo {
  name: string
  env: string
  host: string
  port: number
}

export interface SystemDatabaseInfo {
  max_connections: number
  acquire_timeout_secs: number
}

export interface SystemHugeGraphInfo {
  base_url: string
  gremlin_url: string
}

export interface SystemMilvusInfo {
  address: string
}

export interface SystemVllmInfo {
  base_url: string
  model_name: string
}

export interface SystemStorageInfo {
  upload_dir: string
  report_dir: string
  training_dir: string
}

export interface SystemRuntimeInfo {
  version: string
}

export interface SystemInfoResponse {
  app: SystemAppInfo
  database: SystemDatabaseInfo
  hugegraph: SystemHugeGraphInfo
  milvus: SystemMilvusInfo
  vllm: SystemVllmInfo
  storage: SystemStorageInfo
  runtime: SystemRuntimeInfo
}

export async function fetchHealth() {
  return requestJson<HealthResponse>('/health')
}

export async function fetchSystemInfo() {
  return requestJson<SystemInfoResponse>('/system/info')
}

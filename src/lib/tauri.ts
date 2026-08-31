import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { BatchEvent, RepoStatus, Settings } from '../api'

export async function scanRepos(): Promise<string[]> {
  return invoke<string[]>('scan_repos')
}

export async function readStatus(paths?: string[]): Promise<RepoStatus[]> {
  return invoke<RepoStatus[]>('read_status', { paths: paths ?? null })
}

export async function addRepo(path: string): Promise<RepoStatus> {
  return invoke<RepoStatus>('add_repo', { path })
}

export async function batchFetch(): Promise<number> {
  return invoke<number>('batch_fetch')
}

export async function batchPull(): Promise<number> {
  return invoke<number>('batch_pull')
}

/** 对指定路径子集执行 fetch/pull，返回逐仓结果 */
export interface BatchOutcome {
  path: string
  action: string
  ok: boolean
  skipped: boolean
  exit_code: number | null
  stdout: string
  stderr: string
}

export async function fetchRepos(paths: string[]): Promise<BatchOutcome[]> {
  return invoke<BatchOutcome[]>('fetch_repos', { paths })
}

export async function pullRepos(paths: string[]): Promise<BatchOutcome[]> {
  return invoke<BatchOutcome[]>('pull_repos', { paths })
}

export async function getSettings(): Promise<Settings> {
  return invoke<Settings>('get_settings')
}

export async function saveSettings(s: Settings): Promise<void> {
  await invoke('save_settings', { settings: s })
}

export async function listenBatch(
  handler: (ev: BatchEvent) => void,
): Promise<UnlistenFn> {
  return listen<BatchEvent>('batch://progress', (e) => handler(e.payload))
}

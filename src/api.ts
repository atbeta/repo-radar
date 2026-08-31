// 与 Rust 侧 RepoStatus / BatchEvent / Settings 对应的类型

export interface RepoStatus {
  path: string
  name: string
  branch: string | null
  remote_url: string | null
  is_clean: boolean
  dirty_count: number
  staged: number
  unstaged: number
  untracked: number
  ahead: number
  behind: number
  last_commit_ts: number | null
  fetch_head_ts: number | null
  is_worktree: boolean
  missing: boolean
  error: string | null
}

export interface BatchEvent {
  path: string
  phase: 'started' | 'done'
  ok: boolean
}

export interface Settings {
  roots: string[]
  max_depth: number
  concurrency: number
  exclude: string[]
}

export type SortKey =
  | 'name'
  | 'path'
  | 'branch'
  | 'dirty'
  | 'behind'
  | 'ahead'
  | 'last_commit'
  | 'fetch'

export interface SortSpec {
  key: SortKey
  dir: 'asc' | 'desc'
}

export type FilterState = 'all' | 'dirty' | 'stale'

/** 落后阈值（behind 超过该值视为“显著落后”，UI 高亮） */
export const STALE_BEHIND = 1

export function isStale(r: RepoStatus): boolean {
  return r.behind >= STALE_BEHIND
}

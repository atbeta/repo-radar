import { computed, reactive, ref } from 'vue'
import type { BatchEvent, FilterState, RepoStatus, Settings, SortKey, SortSpec } from '../api'
import { isStale } from '../api'
import * as api from '../lib/tauri'

export function useRadar() {
  // ---- 状态 ----
  const repos = ref<RepoStatus[]>([])
  const settings = ref<Settings>({ roots: [], max_depth: 4, concurrency: 8, exclude: [] })
  const selected = reactive(new Set<string>())
  const busy = ref<'none' | 'scan' | 'refresh' | 'fetch' | 'pull'>('none')
  const message = ref('')
  const showSettings = ref(false)
  const pendingPullPaths = ref<string[] | null>(null) // 非 null 时显示确认弹窗
  const filter = ref<FilterState>('all')
  const search = ref('')
  const sort = ref<SortSpec>({ key: 'name', dir: 'asc' })
  const running = reactive(new Set<string>()) // 批量操作进行中的仓库

  // ---- 派生 ----
  const counts = computed(() => {
    let dirty = 0
    let stale = 0
    let err = 0
    for (const r of repos.value) {
      if (r.error || r.missing) err++
      else {
        if (!r.is_clean) dirty++
        if (isStale(r)) stale++
      }
    }
    return { total: repos.value.length, dirty, stale, err }
  })

  const selectedPaths = computed(() =>
    repos.value.filter((r) => selected.has(r.path)).map((r) => r.path),
  )

  const viewRepos = computed(() => {
    let list = repos.value
    const q = search.value.trim().toLowerCase()
    if (q) {
      list = list.filter(
        (r) =>
          r.name.toLowerCase().includes(q) ||
          (r.branch ?? '').toLowerCase().includes(q) ||
          r.path.toLowerCase().includes(q),
      )
    }
    if (filter.value === 'dirty') list = list.filter((r) => !r.is_clean && !r.error && !r.missing)
    if (filter.value === 'stale') list = list.filter((r) => isStale(r))
    const { key, dir } = sort.value
    const mul = dir === 'asc' ? 1 : -1
    const val = (r: RepoStatus): number | string => {
      switch (key) {
        case 'path':
          return r.path
        case 'branch':
          return r.branch ?? ''
        case 'dirty':
          return r.dirty_count
        case 'behind':
          return r.behind
        case 'ahead':
          return r.ahead
        case 'last_commit':
          return r.last_commit_ts ?? 0
        case 'fetch':
          return r.fetch_head_ts ?? 0
        default:
          return r.name.toLowerCase()
      }
    }
    return [...list].sort((a, b) => {
      const va = val(a)
      const vb = val(b)
      if (va < vb) return -1 * mul
      if (va > vb) return 1 * mul
      return 0
    })
  })

  // ---- 动作 ----
  function flash(msg: string) {
    message.value = msg
    window.setTimeout(() => {
      if (message.value === msg) message.value = ''
    }, 6000)
  }

  async function loadSettings() {
    try {
      settings.value = await api.getSettings()
    } catch (e) {
      flash(`读取设置失败: ${e}`)
    }
  }

  async function persistSettings() {
    try {
      await api.saveSettings(settings.value)
      flash('设置已保存')
      showSettings.value = false
    } catch (e) {
      flash(`保存设置失败: ${e}`)
    }
  }

  async function scan() {
    if (busy.value !== 'none') return
    busy.value = 'scan'
    try {
      const paths = await api.scanRepos()
      const before = new Set(repos.value.map((r) => r.path))
      selected.clear()
      repos.value = paths.map((p) => ({
        path: p,
        name: p.split('/').filter(Boolean).pop() ?? p,
        branch: null,
        remote_url: null,
        is_clean: false,
        dirty_count: 0,
        staged: 0,
        unstaged: 0,
        untracked: 0,
        ahead: 0,
        behind: 0,
        last_commit_ts: null,
        fetch_head_ts: null,
        is_worktree: false,
        missing: false,
        error: before.has(p) ? null : '加载中…',
      }))
      await refresh()
      flash(`扫描完成：${paths.length} 个仓库`)
    } catch (e) {
      flash(`扫描失败: ${e}`)
    } finally {
      busy.value = 'none'
    }
  }

  async function refresh() {
    if (busy.value !== 'none') return
    busy.value = 'refresh'
    try {
      const list = await api.readStatus()
      repos.value = list
    } catch (e) {
      flash(`读取状态失败: ${e}`)
    } finally {
      busy.value = 'none'
    }
  }

  async function addRepo(path: string) {
    try {
      const st = await api.addRepo(path)
      if (!repos.value.some((r) => r.path === st.path)) repos.value.push(st)
      repos.value.sort((a, b) => a.path.localeCompare(b.path))
      flash(`已添加 ${st.name}`)
    } catch (e) {
      flash(`添加失败: ${e}`)
    }
  }

  function toggleSelect(path: string) {
    if (selected.has(path)) selected.delete(path)
    else selected.add(path)
  }

  function selectAllInView() {
    for (const r of viewRepos.value) selected.add(r.path)
  }

  function clearSelection() {
    selected.clear()
  }

  function setSort(key: SortKey) {
    if (sort.value.key === key) {
      sort.value = { key, dir: sort.value.dir === 'asc' ? 'desc' : 'asc' }
    } else {
      sort.value = { key, dir: 'asc' }
    }
  }

  // ---- 批量操作 ----
  function wireProgress() {
    api.listenBatch((ev: BatchEvent) => {
      if (ev.phase === 'started') running.add(ev.path)
      else {
        running.delete(ev.path)
        // 单仓完成后立即刷新该仓状态
        api
          .readStatus([ev.path])
          .then((list) => {
            if (list[0]) {
              const i = repos.value.findIndex((r) => r.path === list[0].path)
              if (i >= 0) repos.value[i] = list[0]
            }
          })
          .catch(() => {})
      }
    })
  }

  async function fetchPaths(paths: string[]) {
    if (!paths.length || busy.value !== 'none') return
    busy.value = 'fetch'
    try {
      const outcomes = await api.fetchRepos(paths)
      await refresh()
      const okN = outcomes.filter((o) => o.ok).length
      const fail = outcomes.filter((o) => !o.ok)
      flash(
        fail.length
          ? `fetch 完成：${okN} 成功 / ${fail.length} 失败（见结果面板）`
          : `fetch 完成：${okN} 个仓库`,
      )
      showResults(outcomes)
    } catch (e) {
      flash(`fetch 失败: ${e}`)
    } finally {
      busy.value = 'none'
      running.clear()
    }
  }

  async function pullPaths(paths: string[]) {
    if (!paths.length || busy.value !== 'none') return
    busy.value = 'pull'
    try {
      const outcomes = await api.pullRepos(paths)
      await refresh()
      const okN = outcomes.filter((o) => o.ok).length
      const skipped = outcomes.filter((o) => o.skipped).length
      const fail = outcomes.filter((o) => !o.ok && !o.skipped)
      let msg = `pull 完成：${okN} 成功`
      if (skipped) msg += ` / ${skipped} 跳过(脏)`
      if (fail.length) msg += ` / ${fail.length} 失败`
      flash(msg)
      showResults(outcomes)
    } catch (e) {
      flash(`pull 失败: ${e}`)
    } finally {
      busy.value = 'none'
      running.clear()
    }
  }

  function requestPull(paths: string[]) {
    pendingPullPaths.value = paths
  }

  function confirmPull() {
    const paths = pendingPullPaths.value ?? []
    pendingPullPaths.value = null
    pullPaths(paths)
  }

  function cancelPull() {
    pendingPullPaths.value = null
  }

  // 结果面板
  const lastOutcomes = ref<api.BatchOutcome[] | null>(null)
  function showResults(outcomes: api.BatchOutcome[]) {
    lastOutcomes.value = outcomes
  }
  function closeResults() {
    lastOutcomes.value = null
  }

  return {
    repos,
    settings,
    selected,
    selectedPaths,
    busy,
    running,
    message,
    showSettings,
    pendingPullPaths,
    filter,
    search,
    sort,
    counts,
    viewRepos,
    lastOutcomes,
    loadSettings,
    persistSettings,
    scan,
    refresh,
    addRepo,
    toggleSelect,
    selectAllInView,
    clearSelection,
    setSort,
    wireProgress,
    fetchPaths,
    pullPaths,
    requestPull,
    confirmPull,
    cancelPull,
    flash,
    closeResults,
  }
}

export type RadarStore = ReturnType<typeof useRadar>

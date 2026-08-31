<script setup lang="ts">
import type { FilterState, SortKey } from '../api'
import type { RepoStatus } from '../api'
import RepoRow from './RepoRow.vue'

defineProps<{
  repos: RepoStatus[]
  selected: Set<string>
  running: Set<string>
  filter: FilterState
  sortKey: SortKey
  sortDir: 'asc' | 'desc'
  loading: boolean
}>()

const emit = defineEmits<{
  (e: 'update:filter', v: FilterState): void
  (e: 'sort', k: SortKey): void
  (e: 'toggle', path: string): void
  (e: 'fetch', path: string): void
  (e: 'pull', path: string): void
}>()

const columns: Array<{ key: SortKey | null; label: string; class?: string }> = [
  { key: null, label: '', class: 'sel' },
  { key: 'name', label: '仓库 / 分支', class: 'name' },
  { key: 'dirty', label: '工作区', class: 'clean' },
  { key: 'behind', label: '同步', class: 'sync' },
  { key: null, label: '远程', class: 'remote' },
  { key: 'last_commit', label: '最近提交', class: 'time' },
  { key: 'fetch', label: '上次 fetch', class: 'time' },
  { key: null, label: '', class: 'actions' },
]

function onHeader(k: SortKey | null) {
  if (k) emit('sort', k)
}
</script>

<template>
  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th
            v-for="c in columns"
            :key="c.label + c.class"
            :class="[c.class, { sortable: !!c.key, active: c.key === sortKey }]"
            @click="onHeader(c.key)"
          >
            {{ c.label }}
            <span v-if="c.key && c.key === sortKey" class="arrow">
              {{ sortDir === 'asc' ? '▲' : '▼' }}
            </span>
          </th>
        </tr>
      </thead>
      <tbody>
        <RepoRow
          v-for="r in repos"
          :key="r.path"
          :r="r"
          :selected="selected.has(r.path)"
          :running="running.has(r.path)"
          :loading="r.error === '加载中…'"
          @toggle="emit('toggle', r.path)"
          @fetch="emit('fetch', r.path)"
          @pull="emit('pull', r.path)"
        />
        <tr v-if="!repos.length && !loading">
          <td colspan="8" class="empty">没有仓库 — 点「扫描」或打开设置配置根目录</td>
        </tr>
        <tr v-if="!repos.length && loading">
          <td colspan="8" class="empty">扫描中…</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.table-wrap {
  flex: 1;
  overflow: auto;
}
table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-sm);
}
thead th {
  position: sticky;
  top: 0;
  z-index: 10;
  background: rgb(var(--background));
  color: rgb(var(--muted-foreground));
  text-align: left;
  font-weight: 500;
  font-size: var(--text-xs);
  padding: 8px 10px;
  border-bottom: 1px solid rgb(var(--border));
  white-space: nowrap;
  user-select: none;
}
th.sortable { cursor: pointer; }
th.sortable:hover { color: rgb(var(--foreground)); }
th.active { color: rgb(var(--primary)); }
.arrow { font-size: 9px; margin-left: 2px; }
tbody tr { border-bottom: 1px solid rgb(var(--border)); }
tbody tr:hover { background: rgb(var(--muted) / 0.5); }
td { padding: 7px 10px; vertical-align: middle; }
.empty {
  text-align: center;
  color: rgb(var(--muted-foreground));
  padding: 40px 0 !important;
}
</style>

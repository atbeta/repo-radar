<script setup lang="ts">
import type { RepoStatus } from '../api'
import { remoteShort, fmtAgo } from '../lib/format'

defineProps<{
  r: RepoStatus
  selected: boolean
  running: boolean
}>()

defineEmits<{
  (e: 'toggle'): void
  (e: 'fetch'): void
  (e: 'pull'): void
}>()
</script>

<template>
  <tr
    :class="{
      dirty: !r.is_clean && !r.error,
      stale: r.behind > 0,
      running,
      selected,
      err: !!r.error,
    }"
    :title="r.path"
  >
    <td class="sel">
      <input
        type="checkbox"
        :checked="selected"
        :disabled="!!r.error"
        @change="$emit('toggle')"
      />
    </td>
    <td class="name">
      <span class="repo-name">{{ r.name }}</span>
      <span v-if="r.is_worktree" class="tag wt" title="git worktree">wt</span>
      <span v-if="r.branch" class="branch">{{ r.branch }}</span>
      <span v-else-if="!r.error" class="branch none">detached</span>
    </td>
    <td class="clean">
      <span v-if="r.error" class="tag err-tag" :title="r.error">错误</span>
      <span v-else-if="r.is_clean" class="ok">✓</span>
      <span v-else class="dirty-badge">
        {{ r.dirty_count }} 项
        <em v-if="r.staged">+{{ r.staged }}s</em>
        <em v-if="r.untracked"> ?{{ r.untracked }}</em>
      </span>
    </td>
    <td class="sync">
      <template v-if="!r.error">
        <span v-if="r.behind" class="behind">↓{{ r.behind }}</span>
        <span v-if="r.ahead" class="ahead">↑{{ r.ahead }}</span>
        <span v-if="!r.behind && !r.ahead" class="insync">—</span>
      </template>
      <span v-else>—</span>
    </td>
    <td class="remote" :title="r.remote_url ?? ''">
      {{ remoteShort(r.remote_url) }}
    </td>
    <td class="time" :title="r.last_commit_ts ? new Date(r.last_commit_ts * 1000).toLocaleString() : ''">
      {{ fmtAgo(r.last_commit_ts) }}
    </td>
    <td class="time">{{ fmtAgo(r.fetch_head_ts) }}</td>
    <td class="actions">
      <button class="mini" :disabled="running" title="fetch（只更新远程信息）" @click="$emit('fetch')">⟳</button>
      <button
        class="mini"
        :disabled="running || !r.is_clean"
        :title="r.is_clean ? 'pull --ff-only' : '有未提交改动，跳过 pull'"
        @click="$emit('pull')"
      >⬇</button>
    </td>
  </tr>
</template>

<style scoped>
tr.selected { background: rgb(var(--primary-soft)); }
tr.running { background: rgb(var(--warning-soft)); }
tr.err { opacity: 0.7; }
.sel { width: 28px; text-align: center; }
.repo-name { font-weight: 600; }
.branch {
  margin-left: 8px;
  font-size: var(--text-2xs);
  color: rgb(var(--primary));
  background: rgb(var(--primary-soft));
  padding: 1px 6px;
  border-radius: 8px;
}
.branch.none { color: rgb(var(--muted-foreground)); background: rgb(var(--muted)); }
.tag.wt {
  margin-left: 6px;
  font-size: var(--text-2xs);
  color: rgb(var(--muted-foreground));
  border: 1px solid rgb(var(--border-strong));
  padding: 0 4px;
  border-radius: 6px;
}
.ok { color: rgb(var(--success)); }
.dirty-badge { color: rgb(var(--warning)); font-weight: 600; }
.dirty-badge em { font-style: normal; font-size: var(--text-2xs); color: rgb(var(--muted-foreground)); }
.tag.err-tag { color: rgb(var(--destructive)); cursor: help; }
.behind { color: rgb(var(--primary)); font-weight: 700; }
.ahead { color: rgb(var(--muted-foreground)); font-weight: 600; margin-left: 6px; }
.insync { opacity: 0.4; }
.remote {
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: rgb(var(--muted-foreground));
  font-size: var(--text-xs);
}
.time { color: rgb(var(--muted-foreground)); font-size: var(--text-xs); white-space: nowrap; }
.actions { white-space: nowrap; }
.mini {
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: var(--text-sm);
  padding: 2px 5px;
  border-radius: var(--radius);
  color: rgb(var(--muted-foreground));
  transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
}
.mini:hover:not(:disabled) { background: rgb(var(--muted)); color: rgb(var(--foreground)); }
.mini:disabled { opacity: 0.25; cursor: default; }
</style>

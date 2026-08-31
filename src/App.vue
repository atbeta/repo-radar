<script setup lang="ts">
import { onMounted } from 'vue'
import type { FilterState, SortKey } from './api'
import RepoTable from './components/RepoTable.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import ResultPanel from './components/ResultPanel.vue'
import ConfirmPull from './components/ConfirmPull.vue'
import { useRadar } from './composables/useRadar'
import { useTheme } from './lib/theme'

const s = useRadar()
const theme = useTheme()

onMounted(() => {
  s.loadSettings()
  s.wireProgress()
})

const busyLabel: Record<string, string> = {
  scan: '扫描中…',
  refresh: '读取状态…',
  fetch: 'fetch 中…',
  pull: 'pull 中…',
}

const filters: Array<{ v: FilterState; label: string }> = [
  { v: 'all', label: '全部' },
  { v: 'dirty', label: '有改动' },
  { v: 'stale', label: '落后' },
]
</script>

<template>
  <div class="app">
    <header class="toolbar">
      <div class="brand">Repo Radar</div>
      <button class="btn primary" :disabled="s.busy.value !== 'none'" @click="s.scan()">
        扫描
      </button>
      <button class="btn" :disabled="s.busy.value !== 'none'" @click="s.refresh()">刷新状态</button>
      <button
        class="btn"
        :disabled="s.busy.value !== 'none' || !s.repos.value.length"
        title="只更新远程信息，不碰工作区"
        @click="s.fetchPaths(s.repos.value.map((r) => r.path))"
      >
        全部 Fetch
      </button>
      <button
        class="btn"
        :disabled="s.busy.value !== 'none' || !s.repos.value.length"
        @click="s.requestPull(s.repos.value.map((r) => r.path))"
      >
        全部 Pull
      </button>

      <span class="spacer" />

      <span v-if="s.selectedPaths.value.length" class="selinfo">
        已选 {{ s.selectedPaths.value.length }}
      </span>
      <button
        v-if="s.selectedPaths.value.length"
        class="btn"
        :disabled="s.busy.value !== 'none'"
        @click="s.fetchPaths(s.selectedPaths.value)"
      >Fetch 选中</button>
      <button
        v-if="s.selectedPaths.value.length"
        class="btn"
        :disabled="s.busy.value !== 'none'"
        @click="s.requestPull(s.selectedPaths.value)"
      >Pull 选中</button>
      <button class="btn ghost" @click="s.clearSelection()">取消选择</button>

      <input
        v-model="s.search.value"
        class="search"
        placeholder="搜索名称 / 分支 / 路径"
      />
      <button class="btn ghost" :title="`主题：${theme.pref.value}`" @click="theme.cycle()">◐</button>
      <button class="btn ghost" title="设置" @click="s.showSettings.value = true">⚙</button>
    </header>

    <div class="subbar">
      <div class="filter">
        <button
          v-for="f in filters"
          :key="f.v"
          :class="['chip', { on: s.filter.value === f.v }]"
          @click="s.filter.value = f.v"
        >
          {{ f.label }}
          <b v-if="f.v === 'all'">{{ s.counts.value.total }}</b>
          <b v-else-if="f.v === 'dirty'" class="warn">{{ s.counts.value.dirty }}</b>
          <b v-else class="bad">{{ s.counts.value.stale }}</b>
        </button>
        <span v-if="s.counts.value.err" class="errcnt" :title="`${s.counts.value.err} 个仓库状态异常`">
          ⚠ {{ s.counts.value.err }}
        </span>
      </div>
      <span class="spacer" />
      <span v-if="s.busy.value !== 'none'" class="busy">{{ busyLabel[s.busy.value] }}</span>
      <span v-if="s.message.value" class="msg">{{ s.message.value }}</span>
    </div>

    <RepoTable
      :repos="s.viewRepos.value"
      :selected="s.selected"
      :running="s.running"
      :filter="s.filter.value"
      :sort-key="s.sort.value.key"
      :sort-dir="s.sort.value.dir"
      :loading="s.busy.value === 'scan'"
      @update:filter="(v) => (s.filter.value = v)"
      @sort="(k: SortKey) => s.setSort(k)"
      @toggle="s.toggleSelect"
      @fetch="(p) => s.fetchPaths([p])"
      @pull="(p) => s.requestPull([p])"
    />

    <footer class="statusbar">
      <span>{{ s.repos.value.length }} 个仓库</span>
      <span class="sep">·</span>
      <span>脏 {{ s.counts.value.dirty }}</span>
      <span class="sep">·</span>
      <span>落后 {{ s.counts.value.stale }}</span>
      <span class="spacer" />
      <span class="dim">pull 只做 --ff-only · 脏仓库自动跳过</span>
    </footer>

    <SettingsPanel
      v-if="s.showSettings.value"
      :settings="s.settings.value"
      @close="s.showSettings.value = false"
      @save="s.persistSettings(); s.scan()"
    />
    <ResultPanel
      v-if="s.lastOutcomes.value"
      :outcomes="s.lastOutcomes.value"
      @close="s.closeResults"
    />
    <ConfirmPull
      v-if="s.pendingPullPaths.value"
      :paths="s.pendingPullPaths.value"
      @confirm="s.confirmPull"
      @cancel="s.cancelPull"
    />
  </div>
</template>

<style>
/* ── 全局基础（token 驱动，跟随 NoteFast 设计语言）── */
* { box-sizing: border-box; }
html, body, #app { height: 100%; margin: 0; }
body {
  background: rgb(var(--background));
  color: rgb(var(--foreground));
  font-family: var(--font-sans);
  font-size: var(--text-base);
  overflow: hidden;
}
.app { display: flex; flex-direction: column; height: 100vh; }

/* ── 工具栏（表面即背景，仅一条 hairline 分隔）── */
.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-bottom: 1px solid rgb(var(--border));
}
.brand { font-weight: 700; margin-right: 8px; font-size: var(--text-md); }

/* ── 按钮体系：ghost / 默认(边框) / primary(墨色) ── */
.btn {
  padding: 6px 14px;
  border-radius: var(--radius);
  border: 1px solid rgb(var(--border));
  background: rgb(var(--card));
  color: rgb(var(--foreground));
  cursor: pointer;
  font-size: var(--text-sm);
  font-family: inherit;
  transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease);
}
.btn:hover:not(:disabled) { background: rgb(var(--muted)); }
.btn:disabled { opacity: 0.4; cursor: default; }
.btn.primary {
  background: rgb(var(--ink));
  border-color: rgb(var(--ink));
  color: rgb(var(--ink-foreground));
}
.btn.primary:hover:not(:disabled) { background: rgb(var(--ink-hover)); border-color: rgb(var(--ink-hover)); }
.btn.ghost { background: transparent; border-color: transparent; color: rgb(var(--muted-foreground)); }
.btn.ghost:hover:not(:disabled) { background: rgb(var(--muted)); color: rgb(var(--foreground)); }

.search {
  width: 200px;
  background: transparent;
  color: rgb(var(--foreground));
  border: 1px solid rgb(var(--border));
  border-radius: var(--radius);
  padding: 6px 10px;
  font-size: var(--text-sm);
  font-family: inherit;
  transition: border-color var(--dur) var(--ease);
}
.search:focus { outline: none; border-color: rgb(var(--ring)); }
.search::placeholder { color: rgb(var(--muted-foreground)); }
.selinfo { color: rgb(var(--primary)); font-size: var(--text-sm); }

/* ── 筛选条 ── */
.subbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 14px;
  border-bottom: 1px solid rgb(var(--border));
}
.filter { display: flex; gap: 6px; align-items: center; }
.chip {
  padding: 4px 10px;
  border-radius: 20px;
  font-size: var(--text-xs);
  border: 1px solid transparent;
  background: transparent;
  color: rgb(var(--muted-foreground));
  cursor: pointer;
  font-family: inherit;
  transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
}
.chip:hover { background: rgb(var(--muted)); }
.chip.on {
  background: rgb(var(--primary) / 0.08);
  color: rgb(var(--primary));
  border-color: rgb(var(--primary) / 0.25);
}
.chip b { margin-left: 4px; font-weight: 600; color: inherit; opacity: 0.75; }
.chip b.warn { color: rgb(var(--warning)); opacity: 1; }
.chip b.bad { color: rgb(var(--primary)); opacity: 1; }
.errcnt { color: rgb(var(--destructive)); font-size: var(--text-xs); margin-left: 6px; }
.busy { color: rgb(var(--warning)); font-size: var(--text-xs); }
.msg {
  color: rgb(var(--muted-foreground));
  font-size: var(--text-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ── 状态栏 ── */
.statusbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 14px;
  border-top: 1px solid rgb(var(--border));
  font-size: var(--text-xs);
  color: rgb(var(--muted-foreground));
}
.statusbar .sep { opacity: 0.5; }
.statusbar .dim { opacity: 0.65; }
</style>

<script setup lang="ts">
import type { BatchOutcome } from '../lib/tauri'

defineProps<{ outcomes: BatchOutcome[] }>()
const emit = defineEmits<{ (e: 'close'): void }>()

function summary(o: BatchOutcome): string {
  if (o.skipped) return '跳过（脏）'
  if (o.ok) return '成功'
  return `失败 (exit ${o.exit_code ?? '?'})`
}
</script>

<template>
  <div class="mask" @click.self="emit('close')">
    <div class="panel">
      <h2>批量操作结果</h2>
      <div class="list">
        <details v-for="o in outcomes" :key="o.path" :class="{ fail: !o.ok && !o.skipped }">
          <summary>
            <span class="s">{{ summary(o) }}</span>
            <span class="p">{{ o.path }}</span>
          </summary>
          <pre v-if="o.stderr.trim()">{{ o.stderr }}</pre>
          <pre v-if="o.stdout.trim()">{{ o.stdout }}</pre>
        </details>
      </div>
      <div class="btns">
        <button class="btn primary" @click="emit('close')">关闭</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 90;
}
.panel {
  width: 560px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  background: rgb(var(--card));
  color: rgb(var(--foreground));
  border-radius: var(--radius-card);
  box-shadow: var(--shadow-floating);
  padding: 18px 20px;
}
h2 { margin: 0 0 12px; font-size: var(--text-md); }
.list { flex: 1; overflow: auto; }
details { margin-bottom: 6px; border: 1px solid rgb(var(--border)); border-radius: var(--radius); }
summary {
  cursor: pointer;
  padding: 6px 10px;
  font-size: var(--text-xs);
  display: flex;
  gap: 10px;
  align-items: baseline;
}
details.fail { border-color: rgb(var(--destructive) / 0.35); }
details.fail summary .s { color: rgb(var(--destructive)); }
.s { white-space: nowrap; font-weight: 600; }
details:not(.fail) summary .s { color: rgb(var(--muted-foreground)); }
.p { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: rgb(var(--muted-foreground)); }
pre {
  margin: 0;
  padding: 8px 12px;
  background: rgb(var(--muted));
  color: rgb(var(--foreground) / 0.8);
  font-size: var(--text-2xs);
  font-family: var(--font-mono);
  overflow-x: auto;
  border-top: 1px solid rgb(var(--border));
  max-height: 150px;
  overflow-y: auto;
}
.btns { display: flex; justify-content: flex-end; margin-top: 14px; }
</style>

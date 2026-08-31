<script setup lang="ts">
defineProps<{ paths: string[] }>()
const emit = defineEmits<{ (e: 'confirm'): void; (e: 'cancel'): void }>()
</script>

<template>
  <div class="mask" @click.self="emit('cancel')">
    <div class="panel">
      <h2>确认 Pull</h2>
      <p>
        将对 <b>{{ paths.length }}</b> 个仓库执行 <code>git pull --ff-only</code>。
        工作区不干净的仓库会自动跳过，不会 stash、不会丢弃任何改动。
      </p>
      <ul>
        <li v-for="p in paths.slice(0, 8)" :key="p">{{ p }}</li>
        <li v-if="paths.length > 8" class="more">…等 {{ paths.length }} 个仓库</li>
      </ul>
      <div class="btns">
        <button class="btn" @click="emit('cancel')">取消</button>
        <button class="btn primary" @click="emit('confirm')">执行 Pull</button>
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
  width: 420px;
  background: rgb(var(--card));
  color: rgb(var(--foreground));
  border-radius: var(--radius-card);
  box-shadow: var(--shadow-floating);
  padding: 20px 22px;
}
h2 { margin: 0 0 10px; font-size: var(--text-md); }
p { font-size: var(--text-sm); color: rgb(var(--foreground) / 0.85); line-height: 1.6; }
code {
  background: rgb(var(--muted));
  padding: 1px 6px;
  border-radius: 4px;
  font-size: var(--text-xs);
  font-family: var(--font-mono);
}
ul { margin: 10px 0; padding-left: 18px; font-size: var(--text-xs); color: rgb(var(--muted-foreground)); }
li { margin-bottom: 3px; word-break: break-all; font-family: var(--font-mono); }
.more { color: rgb(var(--muted-foreground)); list-style: none; margin-left: -18px; font-family: inherit; }
.btns { display: flex; justify-content: flex-end; gap: 10px; margin-top: 16px; }
</style>

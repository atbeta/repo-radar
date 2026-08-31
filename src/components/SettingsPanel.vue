<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Settings } from '../api'
import { probeGit } from '../lib/tauri'

const props = defineProps<{ settings: Settings }>()
const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save'): void
}>()

const probing = ref(false)
const probeResult = ref('')
const probeError = ref('')

async function doProbe() {
  probing.value = true
  probeResult.value = ''
  probeError.value = ''
  try {
    probeResult.value = await probeGit(props.settings.git_path ?? '')
  } catch (e) {
    probeError.value = String(e)
  } finally {
    probing.value = false
  }
}

const rootsText = computed({
  get: () => props.settings.roots.join('\n'),
  set: (v: string) => {
    props.settings.roots = v
      .split('\n')
      .map((s) => s.trim())
      .filter(Boolean)
  },
})
const excludeText = computed({
  get: () => props.settings.exclude.join('\n'),
  set: (v: string) => {
    props.settings.exclude = v
      .split('\n')
      .map((s) => s.trim())
      .filter(Boolean)
  },
})
</script>

<template>
  <div class="mask" @click.self="emit('close')">
    <div class="panel">
      <h2>设置</h2>

      <label class="field">
        <span class="label">扫描根目录（每行一个，如 D:\code）</span>
        <textarea v-model="rootsText" rows="4" spellcheck="false"></textarea>
      </label>

      <label class="field">
        <span class="label">排除目录（每行一个绝对路径，整棵跳过）</span>
        <textarea v-model="excludeText" rows="3" spellcheck="false"></textarea>
      </label>

      <div class="row2">
        <label class="field">
          <span class="label">扫描深度</span>
          <input v-model.number="settings.max_depth" type="number" min="1" max="12" />
        </label>
        <label class="field">
          <span class="label">并发数</span>
          <input v-model.number="settings.concurrency" type="number" min="1" max="32" />
        </label>
      </div>

      <label class="field">
        <span class="label">Git 可执行文件路径（留空 = 使用 PATH 中的 git）</span>
        <div class="gitrow">
          <input
            v-model="settings.git_path"
            type="text"
            placeholder="如 D:\PortableGit\bin\git.exe"
            spellcheck="false"
          />
          <button class="btn probe" :disabled="probing" @click="doProbe">
            {{ probing ? '探测中…' : '探测' }}
          </button>
        </div>
        <span v-if="probeResult" class="probe-ok">✓ {{ probeResult }}</span>
        <span v-else-if="probeError" class="probe-bad">✕ {{ probeError }}</span>
      </label>

      <p class="hint">
        深度 = 从根目录往下最多探测几层目录找 .git。改动只影响之后的「扫描」，
        已添加的仓库不受影响。
      </p>

      <div class="btns">
        <button class="btn" @click="emit('close')">取消</button>
        <button class="btn primary" @click="emit('save')">保存</button>
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
  width: 480px;
  max-height: 85vh;
  overflow: auto;
  background: rgb(var(--card));
  color: rgb(var(--foreground));
  border-radius: var(--radius-card);
  box-shadow: var(--shadow-floating);
  padding: 20px 22px;
}
h2 { margin: 0 0 16px; font-size: var(--text-md); }
.field { display: block; margin-bottom: 14px; }
.label { display: block; font-size: var(--text-xs); color: rgb(var(--muted-foreground)); margin-bottom: 5px; }
textarea, input {
  width: 100%;
  box-sizing: border-box;
  background: rgb(var(--background));
  color: rgb(var(--foreground));
  border: 1px solid rgb(var(--border));
  border-radius: var(--radius);
  padding: 7px 9px;
  font-size: var(--text-sm);
  font-family: inherit;
  transition: border-color var(--dur) var(--ease);
}
textarea { resize: vertical; font-family: var(--font-mono); font-size: var(--text-xs); }
textarea:focus, input:focus { outline: none; border-color: rgb(var(--ring)); }
.row2 { display: flex; gap: 12px; }
.row2 .field { flex: 1; }
.gitrow { display: flex; gap: 8px; }
.gitrow input { flex: 1; }
.btn {
  padding: 7px 14px;
  border-radius: var(--radius);
  border: 1px solid rgb(var(--border));
  background: rgb(var(--muted));
  color: rgb(var(--foreground));
  cursor: pointer;
  font-size: var(--text-xs);
  white-space: nowrap;
}
.btn:disabled { opacity: 0.5; cursor: default; }
.probe-ok { display: block; margin-top: 5px; font-size: var(--text-2xs); color: rgb(var(--success)); }
.probe-bad { display: block; margin-top: 5px; font-size: var(--text-2xs); color: rgb(var(--destructive)); }
.hint { font-size: var(--text-2xs); color: rgb(var(--muted-foreground)); line-height: 1.5; }
.btns { display: flex; justify-content: flex-end; gap: 10px; margin-top: 16px; }
</style>

// 主题管理：light / dark / 跟随系统，持久化 localStorage，同 NoteFast 方案
import { ref, watchEffect } from 'vue'

export type ThemePref = 'light' | 'dark' | 'system'

const KEY = 'repo-radar.theme'
const media = window.matchMedia('(prefers-color-scheme: dark)')

const stored = localStorage.getItem(KEY) as ThemePref | null
const pref = ref<ThemePref>(stored ?? 'system')

function apply() {
  const dark = pref.value === 'dark' || (pref.value === 'system' && media.matches)
  document.documentElement.dataset.theme = dark ? 'dark' : 'light'
}

// 系统偏好变化时实时翻转（仅 system 档受影响，但统一重算无害）
media.addEventListener('change', apply)
watchEffect(apply)

export function useTheme() {
  function set(p: ThemePref) {
    pref.value = p
    localStorage.setItem(KEY, p)
  }
  function cycle() {
    // 三档循环：system → light → dark → system
    const next: ThemePref = pref.value === 'system' ? 'light' : pref.value === 'light' ? 'dark' : 'system'
    set(next)
  }
  return { pref, set, cycle }
}

/** index.html 防闪烁内联脚本用的同款逻辑（保持同步） */
export const THEME_BOOTSTRAP_SNIPPET = `(function(){try{var p=localStorage.getItem('repo-radar.theme')||'system';var d=p==='dark'||(p==='system'&&matchMedia('(prefers-color-scheme: dark)').matches);document.documentElement.dataset.theme=d?'dark':'light';}catch(e){}})();`

// 时间/文本格式化助手

export function fmtTime(ts: number | null): string {
  if (!ts) return '—'
  const d = new Date(ts * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

/** 相对时间的粗略表达，用于快速感知新旧 */
export function fmtAgo(ts: number | null): string {
  if (!ts) return 'never'
  const diff = Date.now() / 1000 - ts
  if (diff < 0) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  return `${Math.floor(diff / 86400)} 天前`
}

/** remote url -> 展示名（去掉协议与 .git 后缀） */
export function remoteShort(url: string | null): string {
  if (!url) return '—'
  let s = url.replace(/\.git$/, '')
  s = s.replace(/^git@([^:]+):/, '$1/')
  s = s.replace(/^https?:\/\//, '')
  s = s.replace(/^(ssh|git):\/\//, '')
  return s
}

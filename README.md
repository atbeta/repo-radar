# Repo Radar

本地多 Git 仓库的只读驾驶舱 — 一屏看清所有仓库的脏净、分支、落后多少，批量 fetch / pull。

## 它解决什么问题

本地囤了大量「只看不写」的仓库时：最新代码要一个个 `git pull`、哪个目录被改过完全不可见。Repo Radar 把这些收进一张表：

- **扫描**：配置根目录（如 `D:\code`），自动发现其中所有 git 仓库（含 worktree），可排除目录、控制深度
- **状态一览**：分支、远程地址、干净/脏（改动+暂存+未跟踪计数）、ahead/behind、最近提交时间、上次 fetch 时间
- **批量操作**：全部 Fetch（无风险，只更新远程信息）/ 全部 Pull / 勾选子集操作
- **安全边界**：pull 只做 `git pull --ff-only`；**工作区不干净的仓库自动跳过**，不 stash、不丢弃任何改动

## 平台

Windows 优先（NSIS 安装包 + 便携 exe，见 Actions 流水线）。核心引擎 `radar-core` 是纯 Rust 无头库，理论上可移植到任意桌面平台。

## 开发

```bash
npm install
npm run tauri dev      # 调试
npm run tauri build    # 本地构建 NSIS 安装包
```

前置：Node 18+、Rust（rustup）、WebView2（Win11 自带）。

### 结构

```
core/          # radar-core：扫描/状态/批量操作的无头引擎（cargo test 覆盖）
src/           # Vue 3 前端驾驶舱（tokens 对齐 NoteFast 设计语言，明暗双主题）
src-tauri/     # Tauri 2 壳层（命令编排 + 设置持久化 + 进度事件）
```

## 构建

推送 `v*` tag（或手动 dispatch）触发 GitHub Actions，产出：

- `RepoRadar_x.y.z_x64-setup.exe` — NSIS 安装包
- `RepoRadar-x.y.z-portable-windows.zip` — 便携单 exe

tag 推送会自动创建 GitHub Release 并附上两个产物。

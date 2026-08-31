//! radar-core: Repo Radar 的无头引擎。
//!
//! 职责：发现本地 git 仓库、并发读取仓库状态、批量 fetch / pull。
//! 设计原则：只读驾驶舱——除 `fetch` / `pull --ff-only` 外不触碰任何工作区；
//! 脏仓库一律跳过 pull，绝不自动 stash。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use serde::Serialize;
use tokio::sync::Semaphore;

/// 默认扫描深度（目录层级）
pub const DEFAULT_MAX_DEPTH: usize = 4;
/// 默认并发数（fetch/pull 同时跑的仓库数）
pub const DEFAULT_CONCURRENCY: usize = 8;
/// 状态读取类命令超时（status / config / log）
const STATUS_TIMEOUT: Duration = Duration::from_secs(30);
/// 批量 fetch/pull 单仓超时（留足网络与凭据交互时间）
const OP_TIMEOUT: Duration = Duration::from_secs(300);

// ---------------------------------------------------------------------------
// 数据模型
// ---------------------------------------------------------------------------

/// 单个仓库的只读状态快照
#[derive(Debug, Clone, Serialize)]
pub struct RepoStatus {
    /// 仓库绝对路径（统一为正斜杠分隔，便于前端展示与跨端一致）
    pub path: String,
    /// 目录名
    pub name: String,
    /// 当前分支名；detached HEAD 时为 None
    pub branch: Option<String>,
    /// origin 远程地址；无 remote 时为 None
    pub remote_url: Option<String>,
    /// 工作区是否干净
    pub is_clean: bool,
    /// 未提交条目总数（改动 + 暂存 + 未跟踪）
    pub dirty_count: u32,
    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    /// 本地领先上游的提交数
    pub ahead: u32,
    /// 本地落后上游的提交数（需先 fetch 才反映远端最新）
    pub behind: u32,
    /// 最近一次提交的 Unix 时间戳（秒）
    pub last_commit_ts: Option<i64>,
    /// .git/FETCH_HEAD 修改时间（上次 fetch 时间，Unix 秒）
    pub fetch_head_ts: Option<i64>,
    /// 是否为 git worktree（.git 是文件而非目录）
    pub is_worktree: bool,
    /// 目录存在但没有 .git（扫描时可能混入）
    pub missing: bool,
    /// 读取状态时的错误信息
    pub error: Option<String>,
}

/// 批量操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchOp {
    Fetch,
    Pull,
}

impl BatchOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            BatchOp::Fetch => "fetch",
            BatchOp::Pull => "pull",
        }
    }
}

/// 批量操作中单个仓库的结果
#[derive(Debug, Clone, Serialize)]
pub struct BatchOutcome {
    pub path: String,
    pub action: &'static str,
    /// git 命令执行成功
    pub ok: bool,
    /// 因脏工作区被跳过（仅 pull）
    pub skipped: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// 批量操作汇总报告
#[derive(Debug, Clone, Serialize)]
pub struct BatchReport {
    pub op: &'static str,
    pub outcomes: Vec<BatchOutcome>,
}

/// 批量操作进度事件（用于 UI 实时刷新）
#[derive(Debug, Clone, Serialize)]
pub struct BatchEvent {
    pub path: String,
    /// started | done
    pub phase: &'static str,
    pub ok: bool,
}

// ---------------------------------------------------------------------------
// 仓库发现
// ---------------------------------------------------------------------------

fn normalize(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// 递归扫描 root 下的 git 仓库（含 worktree）。
///
/// - root 自身是仓库时同样收录，且继续向下扫描嵌套仓库；
/// - 找到 `.git`（目录或文件）即认定仓库，不再向其内部下钻；
/// - `exclude` 中的目录整体跳过（不进入、不记录）。
pub fn discover_roots(root: &Path, max_depth: usize, exclude: &[PathBuf]) -> anyhow::Result<Vec<PathBuf>> {
    let exclude_norm: Vec<String> = exclude.iter().map(|p| normalize(p)).collect();
    let mut found: Vec<PathBuf> = Vec::new();

    if root.join(".git").exists() {
        found.push(root.to_path_buf());
    }

    let mut walker = walkdir::WalkDir::new(root)
        .max_depth(max_depth.saturating_add(1)) // .git 在仓库目录下一层
        .follow_links(false)
        .into_iter();

    loop {
        let entry = match walker.next() {
            Some(Ok(e)) => e,
            Some(Err(_)) => continue,
            None => break,
        };
        if !entry.file_type().is_dir() {
            continue;
        }
        if entry.file_name() == ".git" {
            if let Some(dir) = entry.path().parent() {
                let dir = dir.to_path_buf();
                if !found.contains(&dir) {
                    found.push(dir);
                }
            }
            walker.skip_current_dir(); // 不进入 .git 内部
            continue;
        }
        // 被排除的目录：整棵剪掉
        let p = normalize(entry.path());
        if exclude_norm.iter().any(|x| p == *x || p.starts_with(&format!("{x}/"))) {
            walker.skip_current_dir();
        }
    }

    found.sort();
    Ok(found)
}

// ---------------------------------------------------------------------------
// 状态读取
// ---------------------------------------------------------------------------

fn run_git_sync(git: &str, cwd: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    std::process::Command::new(git)
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
}

/// 解析 `status --porcelain=v2 --branch` 输出
/// 返回 (branch, is_clean, dirty, staged, unstaged, untracked, ahead, behind)
fn parse_status_v2(out: &str) -> (Option<String>, bool, u32, u32, u32, u32, u32, u32) {
    let mut branch = None;
    let mut staged = 0u32;
    let mut unstaged = 0u32;
    let mut untracked = 0u32;
    let mut ahead = 0u32;
    let mut behind = 0u32;

    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            let b = rest.trim();
            if b != "(detached)" {
                branch = Some(b.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for tok in rest.split_whitespace() {
                if let Some(n) = tok.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = tok.strip_prefix('-') {
                    behind = n.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("? ") {
            untracked += 1;
        } else if line.starts_with("1 ") || line.starts_with("2 ") || line.starts_with("u ") {
            // xy 字段：第 0 位是 index 状态（staged），第 1 位是 worktree 状态
            let fields = line.split_whitespace().collect::<Vec<_>>();
            let xy = fields.get(1).copied().unwrap_or("..");
            let bytes = xy.as_bytes();
            if bytes.len() >= 2 {
                if bytes[0] != b'.' {
                    staged += 1;
                }
                if bytes[1] != b'.' {
                    unstaged += 1;
                }
            }
        }
    }

    let dirty = staged + unstaged + untracked;
    (branch, dirty == 0, dirty, staged, unstaged, untracked, ahead, behind)
}

/// 读取单个仓库状态（status / remote url / last commit 并发执行）
pub async fn repo_status(path: &Path, git: &str) -> RepoStatus {
    let git = git.to_string();
    let path_buf = path.to_path_buf();
    let display = normalize(path);
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| display.clone());

    let dot_git = path.join(".git");
    if !dot_git.exists() {
        return RepoStatus {
            path: display,
            name,
            branch: None,
            remote_url: None,
            is_clean: false,
            dirty_count: 0,
            staged: 0,
            unstaged: 0,
            untracked: 0,
            ahead: 0,
            behind: 0,
            last_commit_ts: None,
            fetch_head_ts: None,
            is_worktree: false,
            missing: true,
            error: Some("目录不存在或不是 git 仓库".into()),
        };
    }
    let is_worktree = dot_git.is_file();

    let st = {
        let git = git.clone();
        let p = path_buf.clone();
        tokio::time::timeout(
            STATUS_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                run_git_sync(&git, &p, &["status", "--porcelain=v2", "--branch"])
            }),
        )
        .await
    };
    let rm = {
        let git = git.clone();
        let p = path_buf.clone();
        tokio::time::timeout(
            STATUS_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                run_git_sync(&git, &p, &["config", "--get", "remote.origin.url"])
            }),
        )
        .await
    };
    let lc = {
        let git = git.clone();
        let p = path_buf.clone();
        tokio::time::timeout(
            STATUS_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                run_git_sync(&git, &p, &["log", "-1", "--format=%ct"])
            }),
        )
        .await
    };

    let mut status = RepoStatus {
        path: display,
        name,
        branch: None,
        remote_url: None,
        is_clean: false,
        dirty_count: 0,
        staged: 0,
        unstaged: 0,
        untracked: 0,
        ahead: 0,
        behind: 0,
        last_commit_ts: None,
        fetch_head_ts: None,
        is_worktree,
        missing: false,
        error: None,
    };

    // FETCH_HEAD mtime：本地文件读，不依赖 git
    status.fetch_head_ts = std::fs::metadata(path_buf.join(".git").join("FETCH_HEAD"))
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    match st {
        Ok(Ok(Ok(out))) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            let (branch, is_clean, dirty, staged, unstaged, untracked, ahead, behind) =
                parse_status_v2(&text);
            status.branch = branch;
            status.is_clean = is_clean;
            status.dirty_count = dirty;
            status.staged = staged;
            status.unstaged = unstaged;
            status.untracked = untracked;
            status.ahead = ahead;
            status.behind = behind;
        }
        Ok(Ok(Ok(out))) => {
            status.error = Some(format!(
                "git status 失败 (exit {:?}): {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(Ok(Err(e))) => status.error = Some(format!("无法执行 git: {e}")),
        Ok(Err(e)) => status.error = Some(format!("任务异常: {e}")),
        Err(_) => status.error = Some("读取状态超时（可能有进程卡在凭据输入）".into()),
    }

    if let Ok(Ok(Ok(out))) = rm {
        if out.status.success() {
            let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !url.is_empty() {
                status.remote_url = Some(url);
            }
        }
    }

    if let Ok(Ok(Ok(out))) = lc {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Ok(ts) = s.parse::<i64>() {
                status.last_commit_ts = Some(ts);
            }
        }
    }

    status
}

/// 并发读取多个仓库状态（保持入参顺序）
pub async fn read_statuses(paths: Vec<PathBuf>, git: &str, concurrency: usize) -> Vec<RepoStatus> {
    let sem = Arc::new(Semaphore::new(concurrency.max(1)));
    let git = git.to_string();
    let mut handles = Vec::with_capacity(paths.len());

    for p in paths {
        let sem = sem.clone();
        let git = git.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire_owned().await;
            repo_status(&p, &git).await
        }));
    }

    let mut out = Vec::with_capacity(handles.len());
    for h in handles {
        if let Ok(s) = h.await {
            out.push(s);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 批量 fetch / pull
// ---------------------------------------------------------------------------

fn has_uncommitted(path: &Path, git: &str) -> bool {
    matches!(
        run_git_sync(git, path, &["status", "--porcelain=v2"]),
        Ok(out) if !String::from_utf8_lossy(&out.stdout).trim().is_empty()
    )
}

/// 对一批仓库执行 fetch 或 pull。
///
/// - pull 仅对干净仓库执行 `pull --ff-only`；脏仓库跳过（skipped=true）。
/// - `on_event` 为 None 时不推送进度。
pub async fn batch_op(
    paths: Vec<PathBuf>,
    git: &str,
    op: BatchOp,
    concurrency: usize,
    on_event: Option<Arc<dyn Fn(BatchEvent) + Send + Sync>>,
) -> anyhow::Result<BatchReport> {
    let sem = Arc::new(Semaphore::new(concurrency.max(1)));
    let git = git.to_string();
    let mut handles = Vec::with_capacity(paths.len());

    for p in paths {
        let sem = sem.clone();
        let git = git.clone();
        let ev = on_event.clone();
        handles.push(tokio::spawn(async move {
            let path_str = normalize(&p);
            let _permit = sem.acquire_owned().await;

            if let Some(ev) = &ev {
                ev(BatchEvent { path: path_str.clone(), phase: "started", ok: true });
            }

            let path_for_task = path_str.clone();
            let outcome = tokio::time::timeout(OP_TIMEOUT, tokio::task::spawn_blocking(move || {
                // 脏仓库：pull 跳过，fetch 照常（fetch 不碰工作区）
                if op == BatchOp::Pull && has_uncommitted(&p, &git) {
                    return BatchOutcome {
                        path: path_for_task.clone(),
                        action: op.as_str(),
                        ok: false,
                        skipped: true,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: "跳过：工作区有未提交改动".into(),
                    };
                }

                let args: &[&str] = match op {
                    BatchOp::Fetch => &["fetch", "--all", "--prune"],
                    BatchOp::Pull => &["pull", "--ff-only"],
                };
                match run_git_sync(&git, &p, args) {
                    Ok(out) => BatchOutcome {
                        path: path_for_task,
                        action: op.as_str(),
                        ok: out.status.success(),
                        skipped: false,
                        exit_code: out.status.code(),
                        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                    },
                    Err(e) => BatchOutcome {
                        path: path_for_task,
                        action: op.as_str(),
                        ok: false,
                        skipped: false,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: format!("无法执行 git: {e}"),
                    },
                }
            }))
            .await;
            let outcome = match outcome {
                Ok(Ok(o)) => o,
                Ok(Err(e)) => BatchOutcome {
                    path: path_str.clone(),
                    action: op.as_str(),
                    ok: false,
                    skipped: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: format!("任务异常: {e}"),
                },
                Err(_) => BatchOutcome {
                    path: path_str.clone(),
                    action: op.as_str(),
                    ok: false,
                    skipped: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "操作超时（可能卡在凭据输入或网络）".into(),
                },
            };

            if let Some(ev) = &ev {
                ev(BatchEvent { path: outcome.path.clone(), phase: "done", ok: outcome.ok || outcome.skipped });
            }
            outcome
        }));
    }

    let mut outcomes = Vec::with_capacity(handles.len());
    for h in handles {
        if let Ok(o) = h.await {
            outcomes.push(o);
        }
    }
    outcomes.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(BatchReport { op: op.as_str(), outcomes })
}


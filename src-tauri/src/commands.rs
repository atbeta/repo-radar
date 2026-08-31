//! Tauri 壳层：把 radar-core 暴露给前端。
//! 业务逻辑全部在 radar-core，这里只做参数编排、状态缓存与事件推送。

use std::path::PathBuf;
use std::sync::Mutex;

use radar_core::{BatchEvent, BatchOp, RepoStatus};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

const GIT_BIN: &str = "git";
/// 事件名：批量操作进度
const EVT_BATCH: &str = "batch://progress";

// ---------------------------------------------------------------------------
// 应用状态
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// 扫描根目录列表（Windows 下如 D:/code）
    pub roots: Vec<String>,
    /// 扫描深度
    pub max_depth: usize,
    /// 批量操作并发数
    pub concurrency: usize,
    /// 排除目录
    pub exclude: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            max_depth: radar_core::DEFAULT_MAX_DEPTH,
            concurrency: radar_core::DEFAULT_CONCURRENCY,
            exclude: Vec::new(),
        }
    }
}

/// 设置文件路径：<config>/settings.json
fn settings_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("settings.json"))
}

/// 读取设置；文件不存在或损坏时返回默认值
pub fn load_settings(app: &AppHandle) -> Settings {
    let Some(f) = settings_file(app) else {
        return Settings::default();
    };
    std::fs::read_to_string(f)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// 写入设置文件（尽力而为，失败不阻断）
fn persist_settings(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let Some(f) = settings_file(app) else {
        return Err("无法定位配置目录".into());
    };
    if let Some(parent) = f.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| format!("序列化失败: {e}"))?;
    std::fs::write(f, json).map_err(|e| format!("写入设置失败: {e}"))
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    /// 最近一次扫描/添加得到的仓库路径列表
    pub last_repos: Mutex<Vec<PathBuf>>,
}

// ---------------------------------------------------------------------------
// 命令：设置
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    if settings.roots.is_empty() {
        return Err("至少需要一个扫描根目录".into());
    }
    if settings.max_depth == 0 || settings.max_depth > 12 {
        return Err("扫描深度需在 1-12 之间".into());
    }
    if settings.concurrency == 0 || settings.concurrency > 32 {
        return Err("并发数需在 1-32 之间".into());
    }
    persist_settings(&app, &settings)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

// ---------------------------------------------------------------------------
// 命令：扫描 + 状态
// ---------------------------------------------------------------------------

/// 扫描所有根目录，返回发现的仓库路径列表
#[tauri::command]
async fn scan_repos(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let (roots, max_depth, exclude) = {
        let s = state.settings.lock().unwrap();
        (s.roots.clone(), s.max_depth, s.exclude.clone())
    };
    if roots.is_empty() {
        return Err("尚未配置扫描根目录".into());
    }

    let mut repos: Vec<PathBuf> = Vec::new();
    for r in roots {
        let root = PathBuf::from(r.trim());
        if !root.is_dir() {
            return Err(format!("扫描根目录不存在或不是目录: {r}"));
        }
        let excl: Vec<PathBuf> = exclude.iter().map(PathBuf::from).collect();
        let found = tokio::task::spawn_blocking(move || {
            radar_core::discover_roots(&root, max_depth, &excl)
        })
        .await
        .map_err(|e| format!("扫描任务失败: {e}"))?
        .map_err(|e| format!("扫描失败: {e}"))?;
        repos.extend(found);
    }

    repos.sort();
    repos.dedup();

    *state.last_repos.lock().unwrap() = repos.clone();
    Ok(repos
        .into_iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect())
}

/// 读取指定仓库的状态；paths 为空时读取最近一次扫描结果
#[tauri::command]
async fn read_status(
    state: State<'_, AppState>,
    paths: Option<Vec<String>>,
) -> Result<Vec<RepoStatus>, String> {
    let resolved: Vec<PathBuf> = match paths {
        Some(list) if !list.is_empty() => list.into_iter().map(PathBuf::from).collect(),
        _ => state.last_repos.lock().unwrap().clone(),
    };
    if resolved.is_empty() {
        return Err("没有可读取的仓库（先扫描或手动指定路径）".into());
    }
    let concurrency = state.settings.lock().unwrap().concurrency;
    Ok(radar_core::read_statuses(resolved, GIT_BIN, concurrency).await)
}

/// 手动添加单个仓库（不经过扫描）
#[tauri::command]
async fn add_repo(state: State<'_, AppState>, path: String) -> Result<RepoStatus, String> {
    let p = PathBuf::from(path.trim());
    if !p.join(".git").exists() {
        return Err(format!("不是 git 仓库: {}", p.display()));
    }
    {
        let mut last = state.last_repos.lock().unwrap();
        if !last.contains(&p) {
            last.push(p.clone());
            last.sort();
        }
    }
    Ok(radar_core::repo_status(&p, GIT_BIN).await)
}

// ---------------------------------------------------------------------------
// 命令：批量操作（带进度事件）
// ---------------------------------------------------------------------------

async fn run_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    op: BatchOp,
) -> Result<usize, String> {
    let paths: Vec<PathBuf> = {
        let last = state.last_repos.lock().unwrap();
        if last.is_empty() {
            return Err("没有仓库可操作（先扫描）".into());
        }
        last.clone()
    };
    let concurrency = state.settings.lock().unwrap().concurrency;

    let on_event: std::sync::Arc<dyn Fn(BatchEvent) + Send + Sync> =
        std::sync::Arc::new(move |ev: BatchEvent| {
            let _ = app.emit(EVT_BATCH, &ev);
        });

    let report = radar_core::batch_op(paths, GIT_BIN, op, concurrency, Some(on_event))
        .await
        .map_err(|e| format!("批量操作失败: {e}"))?;

    Ok(report.outcomes.iter().filter(|o| o.ok && !o.skipped).count())
}

#[tauri::command]
async fn batch_fetch(app: AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    run_batch(app, state, BatchOp::Fetch).await
}

#[tauri::command]
async fn batch_pull(app: AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    run_batch(app, state, BatchOp::Pull).await
}

/// 对指定路径子集执行批量操作（返回逐仓结果，用于结果面板）
async fn run_batch_subset(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
    op: BatchOp,
) -> Result<Vec<radar_core::BatchOutcome>, String> {
    if paths.is_empty() {
        return Err("未选择仓库".into());
    }
    let concurrency = state.settings.lock().unwrap().concurrency;
    let resolved: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    let on_event: std::sync::Arc<dyn Fn(BatchEvent) + Send + Sync> =
        std::sync::Arc::new(move |ev: BatchEvent| {
            let _ = app.emit(EVT_BATCH, &ev);
        });
    let report = radar_core::batch_op(resolved, GIT_BIN, op, concurrency, Some(on_event))
        .await
        .map_err(|e| format!("批量操作失败: {e}"))?;
    Ok(report.outcomes)
}

#[tauri::command]
async fn fetch_repos(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<radar_core::BatchOutcome>, String> {
    run_batch_subset(app, state, paths, BatchOp::Fetch).await
}

#[tauri::command]
async fn pull_repos(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<radar_core::BatchOutcome>, String> {
    run_batch_subset(app, state, paths, BatchOp::Pull).await
}

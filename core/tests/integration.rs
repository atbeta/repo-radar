//! 集成测试：所有用例基于真实 git 命令构建的临时仓库。

use std::path::{Path, PathBuf};
use std::process::Command;

use radar_core::{batch_op, discover_roots, read_statuses, repo_status, BatchOp};

// ---------------------------------------------------------------------------
// 测试脚手架
// ---------------------------------------------------------------------------

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git 可执行");
    assert!(
        out.status.success(),
        "git {args:?} 失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// 免环境依赖的 init（不依赖全局 user.name/email）
fn git_init(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let out = Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
        .arg(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "init 失败: {}", String::from_utf8_lossy(&out.stderr));
}

fn commit_file(dir: &Path, name: &str, content: &str, msg: &str) {
    let f = dir.join(name);
    if let Some(parent) = f.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&f, content).unwrap();
    git(dir, &["add", name]);
    git(dir, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-m", msg]);
}

/// src 作为上游仓 origin 的克隆
fn clone(src: &Path, dst: &Path) {
    let out = Command::new("git")
        .arg("clone")
        .arg(src)
        .arg(dst)
        .output()
        .unwrap();
    assert!(out.status.success(), "clone 失败: {}", String::from_utf8_lossy(&out.stderr));
}

fn tmp_root(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("repo-radar-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    base
}

// ---------------------------------------------------------------------------
// 发现扫描
// ---------------------------------------------------------------------------

#[test]
fn test_discover_nested_and_root_itself() {
    let root = tmp_root("discover");

    // root 本身是仓库
    git_init(&root);
    commit_file(&root, "a.txt", "1", "init");

    // 嵌套仓库
    let nested = root.join("team/project-a");
    git_init(&nested);
    commit_file(&nested, "x.txt", "x", "init");

    let found = discover_roots(&root, 4, &[]).unwrap();
    assert_eq!(found.len(), 2, "应发现 root 与 project-a: {found:?}");
    assert_eq!(found[0], root);

    // 排除目录
    let excluded = root.join("team");
    let found2 = discover_roots(&root, 4, &[excluded]).unwrap();
    assert_eq!(found2, vec![root.clone()]);

    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}

#[test]
fn test_discover_respects_depth() {
    let root = tmp_root("depth");
    let deep = root.join("l1/l2/l3/l4/repo");
    git_init(&deep);
    commit_file(&deep, "f", "1", "i");

    let shallow = discover_roots(&root, 2, &[]).unwrap();
    assert!(shallow.is_empty(), "深度 2 不应到达深层仓库: {shallow:?}");

    let deep_res = discover_roots(&root, 6, &[]).unwrap();
    assert_eq!(deep_res.len(), 1);

    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}

// ---------------------------------------------------------------------------
// 状态读取
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_status_clean_and_dirty() {
    let root = tmp_root("status");
    let repo = root.join("r1");
    git_init(&repo);
    commit_file(&repo, "f.txt", "v1", "init");

    let st = repo_status(&repo, "git").await;
    assert_eq!(st.branch.as_deref(), Some("main"));
    assert!(st.is_clean);
    assert_eq!(st.dirty_count, 0);
    assert!(st.remote_url.is_none());
    assert!(st.last_commit_ts.is_some());
    assert!(st.fetch_head_ts.is_none(), "没 fetch 过");

    // 弄脏：改动 + 未跟踪
    std::fs::write(repo.join("f.txt"), "v2").unwrap();
    std::fs::write(repo.join("new.txt"), "n").unwrap();
    let st2 = repo_status(&repo, "git").await;
    assert!(!st2.is_clean);
    assert_eq!(st2.unstaged, 1);
    assert_eq!(st2.untracked, 1);
    assert_eq!(st2.dirty_count, 2);

    // staged
    git(&repo, &["add", "f.txt"]);
    let st3 = repo_status(&repo, "git").await;
    assert_eq!(st3.staged, 1);
    assert_eq!(st3.dirty_count, 2, "staged+untracked");

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_status_ahead_behind_and_remote() {
    let root = tmp_root("ahead");
    let origin = root.join("origin");
    let work = root.join("work");

    git_init(&origin);
    commit_file(&origin, "f.txt", "v1", "init");
    clone(&origin, &work);

    // 落后：上游推进 2 个提交
    commit_file(&origin, "f.txt", "v2", "c2");
    commit_file(&origin, "g.txt", "v3", "c3");
    git(&work, &["fetch"]);
    let st = repo_status(&work, "git").await;
    assert_eq!(st.behind, 2);
    assert_eq!(st.ahead, 0);
    assert!(st.is_clean);
    assert!(st.fetch_head_ts.is_some(), "fetch 后应有 FETCH_HEAD");
    assert!(st.remote_url.is_some(), "克隆仓应有 origin url");

    // 领先：本地推进 1 个提交
    commit_file(&work, "local.txt", "l", "local");
    let st2 = repo_status(&work, "git").await;
    assert_eq!(st2.ahead, 1);
    assert_eq!(st2.behind, 2);

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_status_detached_and_worktree() {
    let root = tmp_root("detached");
    let repo = root.join("r");
    git_init(&repo);
    commit_file(&repo, "f.txt", "v1", "init");
    git(&repo, &["checkout", "--detach", "HEAD"]);

    let st = repo_status(&repo, "git").await;
    assert_eq!(st.branch, None, "detached 应无分支名");
    assert!(st.is_clean);

    // worktree：.git 是文件
    let wt = root.join("wt");
    git(&repo, &["worktree", "add", wt.to_str().unwrap(), "-b", "feature"]);
    assert!(wt.join(".git").is_file());
    let st2 = repo_status(&wt, "git").await;
    assert!(st2.is_worktree);
    assert_eq!(st2.branch.as_deref(), Some("feature"));

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_status_missing_dir() {
    let st = repo_status(Path::new("/nonexistent/definitely-not-here"), "git").await;
    assert!(st.missing);
    assert!(st.error.is_some());
}

#[tokio::test]
async fn test_read_statuses_keeps_order() {
    let root = tmp_root("order");
    let names = ["zoo", "alpha", "mid"];
    for n in names {
        let d = root.join(n);
        git_init(&d);
        commit_file(&d, "f", "1", "i");
    }
    let paths: Vec<PathBuf> = names.iter().map(|n| root.join(n)).collect();
    let out = read_statuses(paths, "git", 2).await;
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].name, "zoo");
    assert_eq!(out[1].name, "alpha");
    assert_eq!(out[2].name, "mid");

    let _ = std::fs::remove_dir_all(root);
}

// ---------------------------------------------------------------------------
// 批量操作
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_batch_pull_skips_dirty_and_ffs_clean() {
    let root = tmp_root("batchpull");
    let origin = root.join("origin");
    git_init(&origin);
    commit_file(&origin, "f.txt", "v1", "init");

    let clean = root.join("clean-repo");
    let dirty = root.join("dirty-repo");
    clone(&origin, &clean);
    clone(&origin, &dirty);

    // 上游新提交
    commit_file(&origin, "f.txt", "v2", "update");
    git(&clean, &["fetch"]);
    git(&dirty, &["fetch"]);

    // clean 干净 / dirty 有未提交改动
    std::fs::write(dirty.join("f.txt"), "local edit").unwrap();

    let report = batch_op(vec![clean.clone(), dirty.clone()], "git", BatchOp::Pull, 4, None)
        .await
        .unwrap();

    assert_eq!(report.op, "pull");
    assert_eq!(report.outcomes.len(), 2);

    let clean_out = report.outcomes.iter().find(|o| o.path.ends_with("clean-repo")).unwrap();
    assert!(clean_out.ok, "clean 应 pull 成功: {:?}", clean_out.stderr);

    let dirty_out = report.outcomes.iter().find(|o| o.path.ends_with("dirty-repo")).unwrap();
    assert!(!dirty_out.ok);
    assert!(dirty_out.skipped, "脏仓库应被跳过");

    // clean 的内容确实更新了
    assert_eq!(std::fs::read_to_string(clean.join("f.txt")).unwrap(), "v2");
    // dirty 的本地改动原样保留
    assert_eq!(std::fs::read_to_string(dirty.join("f.txt")).unwrap(), "local edit");

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_batch_fetch_updates_behind() {
    let root = tmp_root("batchfetch");
    let origin = root.join("origin");
    git_init(&origin);
    commit_file(&origin, "f.txt", "v1", "init");

    let work = root.join("work");
    clone(&origin, &work);
    commit_file(&origin, "f.txt", "v2", "update");
    commit_file(&origin, "f.txt", "v3", "update2");

    // fetch 前 behind=0（本地不知道远端有新提交）
    let before = repo_status(&work, "git").await;
    assert_eq!(before.behind, 0);

    let report = batch_op(vec![work.clone()], "git", BatchOp::Fetch, 4, None)
        .await
        .unwrap();
    assert!(report.outcomes[0].ok, "fetch 应成功: {:?}", report.outcomes[0].stderr);

    // fetch 后 behind=2
    let after = repo_status(&work, "git").await;
    assert_eq!(after.behind, 2);
    // 工作区仍未被触碰（干净且文件内容不变）
    assert!(after.is_clean);
    assert_eq!(std::fs::read_to_string(work.join("f.txt")).unwrap(), "v1");

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_batch_op_reports_failure_and_events() {
    let root = tmp_root("events");
    let ok_repo = root.join("ok");
    git_init(&ok_repo);
    commit_file(&ok_repo, "f", "1", "i");
    // fetch 指向不存在的 remote 应失败
    git(&ok_repo, &["remote", "add", "ghost", "/nonexistent/ghost.git"]);

    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let ev_clone = events.clone();
    let report = batch_op(
        vec![ok_repo.clone()],
        "git",
        BatchOp::Fetch,
        2,
        Some(std::sync::Arc::new(move |e| {
            ev_clone.lock().unwrap().push(format!("{}:{}", e.path, e.phase));
        })),
    )
    .await
    .unwrap();

    let o = &report.outcomes[0];
    assert!(!o.ok, "fetch 不存在的 remote 应失败");
    assert!(!o.skipped);
    let ev = events.lock().unwrap();
    assert!(ev.iter().any(|e| e.ends_with(":started")));
    assert!(ev.iter().any(|e| e.ends_with(":done")));

    let _ = std::fs::remove_dir_all(root);
}

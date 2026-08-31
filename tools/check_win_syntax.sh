#!/usr/bin/env bash
# 在 Linux 上对 Windows GNU target 做纯 rustc 语法检查（跳过 tauri-build 的
# tauri-winres（需要 mingw windres），用 --check 直接解析 lib.rs/commands.rs）。
source "$HOME/.cargo/env"
cd /opt/data/repo-radar/src-tauri
# 只让依赖编译到能出元数据即可；build script 失败的包我们绕不过，
# 所以改用对单文件做语法级检查：rustc --edition 2021 --emit=metadata 单文件会缺依赖。
# 务实方案：让 tauri-build 跳过 winres —— TAURI_SKIP_DEVSERVER_CHECK 无关；
# tauri-winres 只在 build.rs 的 tauri_build::build() 里跑。这里临时用环境变量开关。
export REPO_RADAR_SKIP_WINRES=1
# tauri-winres 不读这个变量 —— 换个思路：注释掉 build.rs 不可行（会提交）。
# 直接用一个临时 build.rs 覆盖：
cp build.rs /tmp/build.rs.bak
printf 'fn main() {}\n' > build.rs
cargo check --target x86_64-pc-windows-gnu 2>&1 | grep -E "^(error|warning: unused)" -A 8 | head -80
cp /tmp/build.rs.bak build.rs

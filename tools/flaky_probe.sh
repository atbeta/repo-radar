#!/usr/bin/env bash
# 并行全量跑 4 轮，记录失败详情与 /tmp 残留
source "$HOME/.cargo/env"
cd /opt/data/repo-radar
for i in 1 2 3 4; do
  echo "=== run $i ==="
  cargo test -p radar-core 2>&1 | grep -E "^test |panicked|assertion|left:|right:|fatal:" | head -30
done
echo "=== leftovers ==="
ls -d /tmp/repo-radar-test-* 2>/dev/null || echo "none"

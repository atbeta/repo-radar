#!/usr/bin/env python3
"""验证生成的图标文件结构（PNG 魔数 / ICO 目录）。"""
import pathlib
import struct

base = pathlib.Path("/opt/data/repo-radar/src-tauri/icons")
PNG_MAGIC = b"\x89PNG\r\n\x1a\n"
ICO_MAGIC = b"\x00\x00\x01\x00"

for name in ("32x32.png", "128x128.png", "128x128@2x.png", "icon.png", "icon.ico"):
    d = (base / name).read_bytes()
    kind = "png" if d[:8] == PNG_MAGIC else ("ico" if d[:4] == ICO_MAGIC else "UNKNOWN")
    print(f"{name:20s} {len(d):8d} bytes  {kind}")

ic = (base / "icon.ico").read_bytes()
_, typ, cnt = struct.unpack("<HHH", ic[:6])
print(f"ico header: type={typ} images={cnt}")

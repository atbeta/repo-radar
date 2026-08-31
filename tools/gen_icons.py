#!/usr/bin/env python3
"""生成 Repo Radar 应用图标（雷达主题），纯 stdlib 实现。"""
import math
import struct
import zlib
from pathlib import Path

ROOT = Path("/opt/data/repo-radar/src-tauri/icons")

def clamp(v):
    return max(0, min(255, int(v)))

def render(size: int) -> bytes:
    """渲染 size x size RGBA 像素，返回 PNG 字节。"""
    px = bytearray(size * size * 4)
    c = size / 2.0
    R = size * 0.46          # 外圈半径
    bg = (18, 18, 26)        # 深底
    ring = (74, 158, 255)    # 环
    sweep = (64, 220, 160)   # 扫描扇区
    blip = (255, 120, 120)   # 目标点

    for y in range(size):
        for x in range(size):
            dx, dy = x - c, y - c
            r = math.hypot(dx, dy)
            i = (y * size + x) * 4
            # 圆形深色底（外圆角）
            if r > R + size * 0.02:
                continue
            col = bg
            # 三圈同心环
            for k, rr in enumerate((0.33, 0.62, 1.0)):
                if abs(r - R * rr) < max(1.0, size * 0.012):
                    col = ring
                    break
            # 扫描扇区（-90°±28°，右上），随角度渐隐
            if col == bg and r <= R * 0.98:
                ang = math.degrees(math.atan2(dy, dx))  # -180..180
                if -118 <= ang <= -62:
                    fade = 1 - abs(ang + 90) / 28
                    t = clamp(40 + 150 * fade * fade)
                    col = (18 + (sweep[0]-18)*fade//3, 18 + (sweep[1]-18)*fade*2//3, 26 + (sweep[2]-26)*fade*2//3)
                    col = (clamp(18 + (sweep[0] - 18) * fade * 0.55),
                           clamp(18 + (sweep[1] - 18) * fade * 0.85),
                           clamp(26 + (sweep[2] - 26) * fade * 0.75))
            # 十字准线（细）
            if col == bg and (abs(dx) < size*0.004 or abs(dy) < size*0.004) and r <= R*0.98:
                col = (40, 52, 74)
            # 三个目标点
            for bx, by in ((0.42, -0.30), (-0.34, 0.12), (0.10, 0.44)):
                if math.hypot(dx - R*bx, dy - R*by) < size * 0.035:
                    col = blip
                    break
            a = 255 if r <= R else clamp((R + size*0.02 - r) / (size*0.02) * 255)
            px[i], px[i+1], px[i+2], px[i+3] = col[0], col[1], col[2], a

    return write_png(size, px)

def write_png(size: int, px: bytearray) -> bytes:
    def chunk(tag: bytes, data: bytes) -> bytes:
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    raw = b"".join(b"\x00" + bytes(px[y*size*4:(y+1)*size*4]) for y in range(size))
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", ihdr)
            + chunk(b"IDAT", zlib.compress(raw, 9))
            + chunk(b"IEND", b""))

def resize_bilinear(png_size: int, target: int) -> bytes:
    """简单最近邻缩放：从大图重新渲染更干净，这里直接重渲染。"""
    return render(target)

def make_ico(sizes, render_fn) -> bytes:
    """ICO 容器，内嵌 PNG（Vista+ 支持）。"""
    imgs = [(s, render_fn(s)) for s in sizes]
    header = struct.pack("<HHH", 0, 1, len(imgs))
    offset = 6 + 16 * len(imgs)
    entries = b""
    body = b""
    for s, data in imgs:
        entries += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(data), offset)
        body += data
        offset += len(data)
    return header + entries + body

if __name__ == "__main__":
    ROOT.mkdir(parents=True, exist_ok=True)
    for name, sz in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256),
                     ("256x256.png", 256), ("512x512.png", 512), ("icon.png", 512),
                     ("Square30x30Logo.png", 30), ("Square44x44Logo.png", 44),
                     ("Square71x71Logo.png", 71), ("Square89x89Logo.png", 89),
                     ("Square107x107Logo.png", 107), ("Square142x142Logo.png", 142),
                     ("Square150x150Logo.png", 150), ("Square284x284Logo.png", 284),
                     ("Square310x310Logo.png", 310), ("StoreLogo.png", 50)):
        (ROOT / name).write_bytes(render(sz))
        print("ok", name)
    (ROOT / "icon.ico").write_bytes(make_ico([16, 24, 32, 48, 64, 128, 256], render))
    print("ok icon.ico")

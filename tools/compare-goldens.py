#!/usr/bin/env python3
"""Compare reference/<screen>.png (Swift) with rust/<screen>.png; write diff/, index.html, summary.txt.

Usage: compare-goldens.py [OUTDIR]   (default /tmp/localwave-goldens); compare-goldens.py --crop FILE TOP [HEIGHT]. Stdlib only.
Metrics: mean absolute RGB difference (0-255) and percent of pixels whose max channel differs by > THRESH.
Pixel-exact equality is not expected (CoreText antialiasing is deferred decision D); review the HTML.
"""
import html, os, struct, sys, zlib

THRESH = 24


def read_png(path):
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    pos, idat, w, h, depth, ctype, interlace = 8, [], 0, 0, 0, 0, 0
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        pos += 12 + n
        if kind == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat.append(body)
    if depth != 8 or ctype not in (2, 6) or interlace:
        raise ValueError(f"{path}: only 8-bit non-interlaced RGB/RGBA PNG is supported")
    bpp = 3 if ctype == 2 else 4
    raw, stride = zlib.decompress(b"".join(idat)), w * bpp
    out, prev = bytearray(), bytearray(stride)
    for y in range(h):
        f, row = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        if f == 1:
            for i in range(bpp, stride):
                row[i] = (row[i] + row[i - bpp]) & 255
        elif f == 2:
            for i in range(stride):
                row[i] = (row[i] + prev[i]) & 255
        elif f == 3:
            for i in range(stride):
                row[i] = (row[i] + (((row[i - bpp] if i >= bpp else 0) + prev[i]) >> 1)) & 255
        elif f == 4:
            for i in range(stride):
                a, b, c = (row[i - bpp] if i >= bpp else 0), prev[i], (prev[i - bpp] if i >= bpp else 0)
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                row[i] = (row[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        out += row
        prev = row
    return w, h, bpp, bytes(out)


def write_png(path, w, h, rgb):
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))
    raw = b"".join(b"\0" + rgb[y * w * 3:(y + 1) * w * 3] for y in range(h))
    open(path, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                           + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def compare(ref, rust, diff_path):
    (w, h, rb, a), (w2, h2, sb, b) = read_png(ref), read_png(rust)
    if (w, h) != (w2, h2):
        return None, f"size {w}x{h} vs {w2}x{h2}"
    total, over, heat = 0, 0, bytearray(w * h * 3)
    for p in range(w * h):
        i, j = p * rb, p * sb
        d = max(abs(a[i] - b[j]), abs(a[i + 1] - b[j + 1]), abs(a[i + 2] - b[j + 2]))
        total += abs(a[i] - b[j]) + abs(a[i + 1] - b[j + 1]) + abs(a[i + 2] - b[j + 2])
        over += d > THRESH
        k = p * 3
        if d > THRESH:
            heat[k:k + 3] = bytes((255, max(0, 255 - d), 0))  # red = large difference
        else:  # dimmed reference for context
            heat[k:k + 3] = bytes((a[i] // 4 + 190, a[i + 1] // 4 + 190, a[i + 2] // 4 + 190))
    write_png(diff_path, w, h, bytes(heat))
    return (total / (w * h * 3), 100.0 * over / (w * h)), None


def crop(path, top, height=872):
    """Drop `top` rows (titlebar inset) in place so the capture is the content area (sips mishandles an offset of exactly the remainder)."""
    w, h, bpp, data = read_png(path)
    assert h >= top + height, f"{path}: {h} rows cannot hold {height} below {top}"
    rgb = b"".join(data[y * w * bpp + x * bpp:y * w * bpp + x * bpp + 3] for y in range(top, top + height) for x in range(w))
    write_png(path, w, height, rgb)


def main():
    if len(sys.argv) in (4, 5) and sys.argv[1] == "--crop":
        return crop(sys.argv[2], int(sys.argv[3]), *map(int, sys.argv[4:]))
    out = sys.argv[1] if len(sys.argv) > 1 else "/tmp/localwave-goldens"
    os.makedirs(f"{out}/diff", exist_ok=True)
    names = sorted(n[:-4] for n in os.listdir(f"{out}/rust") if n.endswith(".png") and os.path.exists(f"{out}/reference/{n}"))
    rows = []
    for name in names:
        try:
            metrics, error = compare(f"{out}/reference/{name}.png", f"{out}/rust/{name}.png", f"{out}/diff/{name}.png")
        except (ValueError, AssertionError, zlib.error) as e:
            metrics, error = None, str(e)
        rows.append((name, metrics, error))
    lines = [f"{'screen':<14}{'mean abs diff':>15}{'% px > ' + str(THRESH):>12}"]
    page = ["<!doctype html><meta charset=utf-8><title>LocalWave goldens</title><style>body{font:14px system-ui;margin:16px}"
            "img{width:480px;border:1px solid #888}table{border-collapse:collapse}td,th{padding:4px 10px;text-align:right}"
            ".row{display:flex;gap:8px;margin-bottom:24px}</style><h1>LocalWave goldens</h1>"
            "<p>reference (Swift) | rust | diff (red: channel diff &gt; %d)</p><table><tr><th>screen<th>mean abs diff<th>%% px &gt; %d</tr>" % (THRESH, THRESH)]
    for name, metrics, error in rows:
        if metrics:
            lines.append(f"{name:<14}{metrics[0]:>15.2f}{metrics[1]:>11.1f}%")
            page.append(f"<tr><td><a href='#{name}'>{name}</a><td>{metrics[0]:.2f}<td>{metrics[1]:.1f}%</tr>")
        else:
            lines.append(f"{name:<14}{'ERROR: ' + str(error):>27}")
            page.append(f"<tr><td>{name}<td colspan=2>{html.escape(str(error))}</tr>")
    page.append("</table>")
    for name, metrics, error in rows:
        page.append(f"<h2 id='{name}'>{name}</h2><div class=row><img src='reference/{name}.png'><img src='rust/{name}.png'>"
                    + (f"<img src='diff/{name}.png'>" if metrics else "") + "</div>")
    open(f"{out}/index.html", "w").write("".join(page))
    open(f"{out}/summary.txt", "w").write("\n".join(lines) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    main()

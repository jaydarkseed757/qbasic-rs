#!/usr/bin/env python3
"""Convert a binary P6 PPM (written by the runtime's QBC_DUMP) to a PNG.

Scales the native framebuffer to a target size with nearest-neighbor — the same
thing the runtime window does — so screenshots look exactly like the running
program. Pure standard library (zlib only); no Pillow/ImageMagick needed.

Usage: ppm2png.py in.ppm out.png [width height]   (default 960x600)
"""
import sys, zlib, struct


def read_ppm(path):
    data = open(path, "rb").read()
    if data[:2] != b"P6":
        raise ValueError("not a P6 PPM")
    idx, vals = 2, []
    while len(vals) < 3:
        while idx < len(data) and data[idx] in b" \t\n\r":
            idx += 1
        if data[idx:idx + 1] == b"#":            # comment line
            while data[idx] not in b"\n":
                idx += 1
            continue
        start = idx
        while idx < len(data) and data[idx] not in b" \t\n\r":
            idx += 1
        vals.append(int(data[start:idx]))
    w, h, _maxv = vals
    idx += 1                                       # one whitespace after maxval
    return w, h, data[idx:idx + w * h * 3]


def scale_nn(w, h, pix, tw, th):
    out = bytearray(tw * th * 3)
    xmap = [ox * w // tw for ox in range(tw)]      # precompute source columns
    for oy in range(th):
        srow = (oy * h // th) * w
        di = oy * tw * 3
        for sx in xmap:
            si = (srow + sx) * 3
            out[di:di + 3] = pix[si:si + 3]
            di += 3
    return bytes(out)


def write_png(path, w, h, rgb):
    def chunk(typ, data):
        return (struct.pack(">I", len(data)) + typ + data
                + struct.pack(">I", zlib.crc32(typ + data) & 0xffffffff))
    raw = bytearray()
    for y in range(h):
        raw.append(0)                              # filter: none
        raw += rgb[y * w * 3:(y + 1) * w * 3]
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)   # 8-bit RGB
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", ihdr))
        f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
        f.write(chunk(b"IEND", b""))


def main():
    if len(sys.argv) < 3:
        print(__doc__); sys.exit(1)
    src, dst = sys.argv[1], sys.argv[2]
    tw = int(sys.argv[3]) if len(sys.argv) > 3 else 960
    th = int(sys.argv[4]) if len(sys.argv) > 4 else 600
    w, h, pix = read_ppm(src)
    write_png(dst, tw, th, scale_nn(w, h, pix, tw, th))
    print(f"{src} ({w}x{h}) -> {dst} ({tw}x{th})")


if __name__ == "__main__":
    main()

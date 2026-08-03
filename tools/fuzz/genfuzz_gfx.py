#!/usr/bin/env python3
"""Random SCREEN 13 graphics-program generator for DETERMINISM fuzzing.

Unlike genfuzz.py/qbref.py (which diff qbc's output against an independent
text-mode reference interpreter), there is no independent renderer to check
graphics output AGAINST — so this generator isn't paired with an oracle.
Instead it targets a narrower but real property: the SAME program run
headless TWICE, with the SAME seed, must produce a BIT-IDENTICAL framebuffer
checksum. That property is exactly what the wall-clock/opt-level flakes
(see CLAUDE.md's "Simulated headless clock" entry) violated, and it's only
checkable now that headless time is fully virtual — before that fix, this
generator would have failed almost every seed.

Generated programs are entirely literal-driven (no RND at the .bas level —
randomness lives in the PYTHON generator, seeded by `seed`), so there is no
QB-RNG-fidelity question to get tangled up in; this is purely an engine-level
regression net. Coordinates are bounded to the SCREEN 13 canvas (0..319 /
0..199) and colors to 0..255, so no generated program should ever error or
hang. A `WAIT &H3DA, 8[, 8]` pair (the vsync double-wait idiom) and a `SLEEP`
are included with real weight, since the virtual clock's frame-pacing and
vsync paths are exactly what a wall-clock regression would land in.

Usage: genfuzz_gfx.py SEED > prog.bas
"""
import random
import sys

W, H = 319, 199


class GfxGen:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.lines = ["SCREEN 13"]
        self.boxes = []  # (x1,y1,x2,y2) of B-outlined boxes drawn so far, for PAINT

    def pt(self):
        return self.r.randint(0, W), self.r.randint(0, H)

    def color(self):
        return self.r.randint(0, 255)

    def emit(self, line):
        self.lines.append(line)

    def gen_pset(self):
        x, y = self.pt()
        self.emit(f"PSET ({x},{y}), {self.color()}")

    def gen_line(self):
        x1, y1 = self.pt()
        x2, y2 = self.pt()
        form = self.r.choice(["", "B", "BF"])
        c = self.color()
        if form:
            self.emit(f"LINE ({x1},{y1})-({x2},{y2}), {c}, {form}")
            if form == "B":
                self.boxes.append((min(x1, x2), min(y1, y2), max(x1, x2), max(y1, y2)))
        else:
            self.emit(f"LINE ({x1},{y1})-({x2},{y2}), {c}")

    def gen_circle(self):
        x, y = self.pt()
        r = self.r.randint(1, 60)
        self.emit(f"CIRCLE ({x},{y}), {r}, {self.color()}")

    def gen_color(self):
        self.emit(f"COLOR {self.r.randint(1, 255)}")

    def gen_paint(self):
        # Flood fill bounded inside an already-drawn B (outline-only) box, so
        # the fill region is guaranteed closed and small — never the whole
        # screen. Falls back to a PSET if no box exists yet.
        if not self.boxes:
            self.gen_pset()
            return
        x1, y1, x2, y2 = self.r.choice(self.boxes)
        if x2 - x1 < 2 or y2 - y1 < 2:
            self.gen_pset()
            return
        mx, my = (x1 + x2) // 2, (y1 + y2) // 2
        border = self.r.randint(0, 255)
        fill = self.r.randint(0, 255)
        # Redraw the border in a known color right before painting, so the
        # fill/border relationship is exact regardless of what overdrew it.
        self.emit(f"LINE ({x1},{y1})-({x2},{y2}), {border}, B")
        self.emit(f"PAINT ({mx},{my}), {fill}, {border}")

    def gen_sprite_roundtrip(self):
        x1, y1 = self.r.randint(0, W - 20), self.r.randint(0, H - 20)
        x2, y2 = x1 + self.r.randint(4, 20), y1 + self.r.randint(4, 20)
        dx, dy = self.r.randint(0, W - 20), self.r.randint(0, H - 20)
        verb = self.r.choice(["PSET", "XOR", "OR", "AND"])
        name = f"SPR{len(self.lines)}"
        # DIM size is cosmetic — GET/PUT resize the Vec dynamically — but
        # `AS INTEGER` matches the convention every bundled sprite program
        # uses (screen13-sprite.bas).
        self.emit(f"DIM {name}(400) AS INTEGER")
        self.emit(f"GET ({x1},{y1})-({x2},{y2}), {name}")
        self.emit(f"PUT ({dx},{dy}), {name}, {verb}")

    def gen_vsync(self):
        self.emit("WAIT &H3DA, 8, 8")
        self.emit("WAIT &H3DA, 8")

    def gen_sleep(self):
        self.emit(f"SLEEP {self.r.choice([0, 1])}")

    def program(self):
        n = self.r.randint(15, 35)
        weighted = (
            [self.gen_pset] * 4 + [self.gen_line] * 4 + [self.gen_circle] * 3
            + [self.gen_color] * 2 + [self.gen_paint] * 3
            + [self.gen_sprite_roundtrip] * 2 + [self.gen_vsync] * 2
            + [self.gen_sleep] * 1
        )
        for _ in range(n):
            self.r.choice(weighted)()
        self.emit("END")
        return "\n".join(self.lines) + "\n"


if __name__ == "__main__":
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    sys.stdout.write(GfxGen(seed).program())

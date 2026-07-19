#!/usr/bin/env python3
"""Random QBasic program generator for differential fuzzing (qbc vs qbref.py).

Generates a seeded, deterministic program over a QB subset chosen so that the
transpiled-native run and the reference interpreter must agree EXACTLY:

- All numeric values stay integral and within f64-exact / i64-safe range:
  every numeric assignment is tamed with `MOD 32749`, `*` only joins atoms,
  `^` exponents are literals 0..3, `/` appears only as `INT(expr / lit)`.
- All loops provably terminate (FOR with literal bounds; WHILE/DO guarded by
  dedicated `L<n>` counters the body never touches).
- No RND/TIMER/INKEY$/graphics — pure deterministic text computation.
- String growth is tamed with `LEFT$(expr, 40)` at assignment.

Usage: genfuzz.py SEED > prog.bas
"""
import random
import sys

NUMVARS = ["A", "B", "C", "D", "E"]
STRVARS = ["S$", "T$", "U$"]
ARRAYS  = [("AR", 20), ("BR", 12)]          # numeric 1-D arrays, DIM name(upper)
STRLITS = ["AB", "xyz", "Hello", "Q", "no", "FUZZ", "bAsIc", ""]

class Gen:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.lines = []
        self.loop_id = 0
        self.for_depth = 0
        self.stmt_budget = 0

    # ── expressions ─────────────────────────────────────────────────────────
    def atom(self):
        c = self.r.random()
        if c < 0.35:
            return str(self.r.randint(0, 999))
        if c < 0.65:
            return self.r.choice(NUMVARS)
        if c < 0.75:
            # loop counters are readable anywhere (undefined reads are 0 on
            # both sides)
            return self.r.choice(["F1", "F2", "L1", "L2", "L3"])
        if c < 0.9:
            name, upper = self.r.choice(ARRAYS)
            return f"{name}(ABS({self.nexpr(0)}) MOD {upper + 1})"
        return f"-{self.r.randint(1, 99)}"

    def nexpr(self, depth):
        """Numeric expression. All operands stay i64/f64-exact."""
        if depth >= 3 or self.r.random() < 0.3:
            return self.atom()
        c = self.r.random()
        a = self.nexpr(depth + 1)
        b = self.nexpr(depth + 1)
        if c < 0.30:
            op = self.r.choice(["+", "-"])
            return f"({a} {op} {b})"
        if c < 0.42:
            # * only joins atoms → bounded product
            return f"({self.atom()} * {self.atom()})"
        if c < 0.52:
            return f"({a} \\ {self.r.randint(1, 7)})"
        if c < 0.62:
            return f"({a} MOD {self.r.randint(2, 97)})"
        if c < 0.68:
            return f"({self.atom()} ^ {self.r.randint(0, 3)})"
        if c < 0.74:
            return f"INT({a} / {self.r.randint(1, 7)})"
        if c < 0.80:
            op = self.r.choice(["=", "<>", "<", ">", "<=", ">="])
            return f"({a} {op} {b})"
        if c < 0.86:
            op = self.r.choice(["AND", "OR", "XOR"])
            return f"({a} {op} {b})"
        if c < 0.90:
            return f"(NOT {a})"
        if c < 0.94:
            return f"ABS({a})"
        if c < 0.97:
            return f"SGN({a})"
        return f"LEN({self.sexpr(depth + 1)})"

    def cond(self, depth=1):
        a = self.nexpr(depth)
        b = self.nexpr(depth)
        op = self.r.choice(["=", "<>", "<", ">", "<=", ">="])
        return f"{a} {op} {b}"

    def sexpr(self, depth):
        """String expression."""
        if depth >= 3 or self.r.random() < 0.4:
            c = self.r.random()
            if c < 0.5:
                return '"' + self.r.choice(STRLITS) + '"'
            return self.r.choice(STRVARS)
        c = self.r.random()
        a = self.sexpr(depth + 1)
        if c < 0.30:
            return f"({a} + {self.sexpr(depth + 1)})"
        if c < 0.42:
            return f"LEFT$({a}, ABS({self.atom()}) MOD 10)"
        if c < 0.54:
            return f"RIGHT$({a}, ABS({self.atom()}) MOD 10)"
        if c < 0.68:
            return f"MID$({a}, ABS({self.atom()}) MOD 20 + 1, ABS({self.atom()}) MOD 10)"
        if c < 0.78:
            return f"UCASE$({a})"
        if c < 0.86:
            return f"LCASE$({a})"
        if c < 0.94:
            return f"STR$({self.nexpr(depth + 1)})"
        return f"CHR$(ABS({self.nexpr(depth + 1)}) MOD 95 + 32)"

    # ── statements ──────────────────────────────────────────────────────────
    def emit(self, line, indent):
        self.lines.append("    " * indent + line)

    def gen_assign(self, indent):
        c = self.r.random()
        if c < 0.55:
            v = self.r.choice(NUMVARS)
            self.emit(f"{v} = ({self.nexpr(0)}) MOD 32749", indent)
        elif c < 0.75:
            name, upper = self.r.choice(ARRAYS)
            idx = f"ABS({self.nexpr(1)}) MOD {upper + 1}"
            self.emit(f"{name}({idx}) = ({self.nexpr(0)}) MOD 32749", indent)
        else:
            v = self.r.choice(STRVARS)
            self.emit(f"{v} = LEFT$({self.sexpr(0)}, 40)", indent)

    def gen_print(self, indent):
        n = self.r.randint(1, 4)
        parts = []
        for _ in range(n):
            if self.r.random() < 0.6:
                parts.append(self.nexpr(1))
            else:
                parts.append(self.sexpr(1))
        sep = ", " if self.r.random() < 0.25 else "; "
        trail = ";" if self.r.random() < 0.2 else ""
        self.emit("PRINT " + sep.join(parts) + trail, indent)

    def gen_if(self, indent, depth):
        self.emit(f"IF {self.cond()} THEN", indent)
        self.gen_block(indent + 1, depth + 1, self.r.randint(1, 3))
        if self.r.random() < 0.4:
            self.emit(f"ELSEIF {self.cond()} THEN", indent)
            self.gen_block(indent + 1, depth + 1, self.r.randint(1, 2))
        if self.r.random() < 0.5:
            self.emit("ELSE", indent)
            self.gen_block(indent + 1, depth + 1, self.r.randint(1, 2))
        self.emit("END IF", indent)

    def gen_for(self, indent, depth):
        self.for_depth += 1
        v = f"F{self.for_depth}"
        a = self.r.randint(0, 5)
        b = self.r.randint(0, 8)
        step = self.r.choice(["", " STEP 2", " STEP -1", " STEP -2"])
        if step.startswith(" STEP -"):
            a, b = max(a, b), min(a, b)
        self.emit(f"FOR {v} = {a} TO {b}{step}", indent)
        self.gen_block(indent + 1, depth + 1, self.r.randint(1, 3))
        self.emit(f"NEXT {v}", indent)
        self.for_depth -= 1

    def gen_while(self, indent, depth):
        self.loop_id += 1
        lv = f"L{self.loop_id}"
        limit = self.r.randint(2, 6)
        self.emit(f"{lv} = 0", indent)
        self.emit(f"WHILE {lv} < {limit}", indent)
        self.gen_block(indent + 1, depth + 1, self.r.randint(1, 3))
        self.emit(f"{lv} = {lv} + 1", indent + 1)
        self.emit("WEND", indent)

    def gen_do(self, indent, depth):
        self.loop_id += 1
        lv = f"L{self.loop_id}"
        limit = self.r.randint(2, 6)
        self.emit(f"{lv} = 0", indent)
        form = self.r.randrange(4)
        if form == 0:
            self.emit(f"DO WHILE {lv} < {limit}", indent)
        elif form == 1:
            self.emit(f"DO UNTIL {lv} >= {limit}", indent)
        else:
            self.emit("DO", indent)
        self.gen_block(indent + 1, depth + 1, self.r.randint(1, 3))
        self.emit(f"{lv} = {lv} + 1", indent + 1)
        if form == 2:
            self.emit(f"LOOP WHILE {lv} < {limit}", indent)
        elif form == 3:
            self.emit(f"LOOP UNTIL {lv} >= {limit}", indent)
        else:
            self.emit("LOOP", indent)

    def gen_select(self, indent, depth):
        self.emit(f"SELECT CASE ABS({self.nexpr(1)}) MOD 10", indent)
        used = 0
        for _ in range(self.r.randint(1, 3)):
            c = self.r.random()
            if c < 0.4:
                vals = ", ".join(str(self.r.randint(0, 9))
                                 for _ in range(self.r.randint(1, 2)))
                self.emit(f"CASE {vals}", indent)
            elif c < 0.7:
                a = self.r.randint(0, 6)
                self.emit(f"CASE {a} TO {a + self.r.randint(0, 3)}", indent)
            else:
                op = self.r.choice(["<", ">", "<=", ">="])
                self.emit(f"CASE IS {op} {self.r.randint(0, 9)}", indent)
            self.gen_block(indent + 1, depth + 1, self.r.randint(1, 2))
            used += 1
        if self.r.random() < 0.6:
            self.emit("CASE ELSE", indent)
            self.gen_block(indent + 1, depth + 1, 1)
        self.emit("END SELECT", indent)

    def gen_stmt(self, indent, depth):
        self.stmt_budget -= 1
        c = self.r.random()
        if depth >= 3 or c < 0.45 or self.stmt_budget < 0:
            self.gen_assign(indent)
        elif c < 0.60:
            self.gen_print(indent)
        elif c < 0.72:
            self.gen_if(indent, depth)
        elif c < 0.82 and self.for_depth < 2:
            self.gen_for(indent, depth)
        elif c < 0.88:
            self.gen_while(indent, depth)
        elif c < 0.94:
            self.gen_do(indent, depth)
        else:
            self.gen_select(indent, depth)

    def gen_block(self, indent, depth, n):
        for _ in range(n):
            self.gen_stmt(indent, depth)

    def program(self):
        self.emit("' fuzz-generated program (genfuzz.py)", 0)
        for name, upper in ARRAYS:
            self.emit(f"DIM {name}({upper})", 0)
        self.stmt_budget = 60
        self.gen_block(0, 0, self.r.randint(12, 22))
        # Final state dump — maximizes diff sensitivity.
        self.emit('PRINT "-- dump --"', 0)
        self.emit("PRINT " + "; ".join(NUMVARS), 0)
        self.emit("PRINT " + '; "|"; '.join(STRVARS), 0)
        for name, upper in ARRAYS:
            self.emit(f"FOR F9 = 0 TO {upper}", 0)
            self.emit(f"PRINT {name}(F9);", 1)
            self.emit("NEXT F9", 0)
            self.emit("PRINT", 0)
        return "\n".join(self.lines) + "\n"

if __name__ == "__main__":
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    sys.stdout.write(Gen(seed).program())

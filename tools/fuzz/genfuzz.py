#!/usr/bin/env python3
"""Random QBasic program generator for differential fuzzing (qbc vs qbref.py).

Generates a seeded, deterministic program over a QB subset chosen so that the
transpiled-native run and the reference interpreter must agree EXACTLY:

- All numeric values stay integral and within f64-exact / i64-safe range:
  every numeric assignment is tamed with `MOD 32749`, `*` only joins atoms,
  `^` exponents are literals 0..3, `/` appears only as `INT(expr / lit)`.
- All loops provably terminate (FOR with literal bounds; WHILE/DO guarded by
  dedicated `L<n>` counters the body never touches); GOSUB nesting is
  strictly downward (a sub may only GOSUB a higher-numbered sub).
- No RND/TIMER/INKEY$/graphics — pure deterministic text computation.
- String growth is tamed with `LEFT$(expr, 40)` at assignment.

Two program styles:
- Mode A (structured): IF/FOR/WHILE/DO/SELECT nesting, GOSUB subroutines
  after END, SWAP, 2-D arrays, string comparisons, PRINT USING.
- Mode B (seed % 3 == 0; flat line-numbered): every line numbered, forward
  GOTO / IF…THEN GOTO / numeric GOSUB — specifically targets the __pc
  state-machine emitter and numeric-GOSUB extraction.

Usage: genfuzz.py SEED > prog.bas
"""
import random
import sys

NUMVARS = ["A", "B", "C", "D", "E"]
STRVARS = ["S$", "T$", "U$"]
ARRAYS  = [("AR", 20), ("BR", 12)]          # numeric 1-D arrays, DIM name(upper)
ARRAYS2 = [("G2", 6, 4)]                    # numeric 2-D arrays, DIM name(u1, u2)
STRLITS = ["AB", "xyz", "Hello", "Q", "no", "FUZZ", "bAsIc", ""]


class Gen:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.lines = []
        self.loop_id = 0
        self.for_depth = 0
        self.stmt_budget = 0
        self.n_subs = 0
        self.sub_level = 0     # 0 = main; sub k may only GOSUB j > k
        self.use_2d = True     # mode B doesn't DIM the 2-D array

    # ── expressions ─────────────────────────────────────────────────────────
    def arr1_ref(self):
        name, upper = self.r.choice(ARRAYS)
        return f"{name}(ABS({self.nexpr(2)}) MOD {upper + 1})"

    def arr2_ref(self):
        name, u1, u2 = self.r.choice(ARRAYS2)
        return (f"{name}(ABS({self.nexpr(2)}) MOD {u1 + 1}, "
                f"ABS({self.nexpr(2)}) MOD {u2 + 1})")

    def atom(self):
        c = self.r.random()
        if c < 0.32:
            return str(self.r.randint(0, 999))
        if c < 0.60:
            return self.r.choice(NUMVARS)
        if c < 0.70:
            return self.r.choice(["F1", "F2", "L1", "L2", "L3"])
        if c < 0.82:
            return self.arr1_ref()
        if c < 0.90 and self.use_2d:
            return self.arr2_ref()
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
        if c < 0.67:
            return f"({self.atom()} ^ {self.r.randint(0, 3)})"
        if c < 0.72:
            return f"INT({a} / {self.r.randint(1, 7)})"
        if c < 0.78:
            op = self.r.choice(["=", "<>", "<", ">", "<=", ">="])
            return f"({a} {op} {b})"
        if c < 0.82:
            # string comparison as a numeric (-1/0) value
            op = self.r.choice(["=", "<>", "<", ">", "<=", ">="])
            return f"({self.sexpr(depth + 1)} {op} {self.sexpr(depth + 1)})"
        if c < 0.87:
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
        if self.r.random() < 0.25:
            a = self.sexpr(depth)
            b = self.sexpr(depth)
        else:
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

    def num_target(self):
        c = self.r.random()
        if c < 0.55:
            return self.r.choice(NUMVARS)
        if c < 0.85:
            return self.arr1_ref()
        return self.arr2_ref()

    def gen_assign(self, indent):
        c = self.r.random()
        if c < 0.75:
            self.emit(f"{self.num_target()} = ({self.nexpr(0)}) MOD 32749", indent)
        else:
            v = self.r.choice(STRVARS)
            self.emit(f"{v} = LEFT$({self.sexpr(0)}, 40)", indent)

    def gen_swap(self, indent):
        c = self.r.random()
        if c < 0.4:
            a, b = self.r.sample(NUMVARS, 2)
        elif c < 0.6:
            a, b = self.r.sample(STRVARS, 2)
        elif c < 0.8:
            a, b = self.num_target(), self.num_target()
        else:
            a, b = self.r.choice(NUMVARS), self.arr1_ref()
        self.emit(f"SWAP {a}, {b}", indent)

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

    def gen_print_using(self, indent):
        nfields = self.r.randint(1, 2)
        fmt = ""
        args = []
        for k in range(nfields):
            lit = self.r.choice(["v:", " |", "x=", "", ">"])
            fmt += lit.replace("#", "") + "#" * self.r.randint(4, 7)
            args.append(f"({self.nexpr(1)}) MOD 32749")
        trail = ";" if self.r.random() < 0.2 else ""
        self.emit(f'PRINT USING "{fmt}"; ' + "; ".join(args) + trail, indent)

    def gen_gosub(self, indent):
        lo = self.sub_level + 1
        if lo > self.n_subs:
            self.gen_assign(indent)
            return
        self.emit(f"GOSUB SUB{self.r.randint(lo, self.n_subs)}", indent)

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
        if self.r.random() < 0.6:
            self.emit("CASE ELSE", indent)
            self.gen_block(indent + 1, depth + 1, 1)
        self.emit("END SELECT", indent)

    def gen_stmt(self, indent, depth):
        self.stmt_budget -= 1
        c = self.r.random()
        if depth >= 3 or c < 0.40 or self.stmt_budget < 0:
            self.gen_assign(indent)
        elif c < 0.50:
            self.gen_print(indent)
        elif c < 0.55:
            self.gen_print_using(indent)
        elif c < 0.60:
            self.gen_swap(indent)
        elif c < 0.65:
            self.gen_gosub(indent)
        elif c < 0.74:
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

    def emit_dims(self):
        for name, upper in ARRAYS:
            self.emit(f"DIM {name}({upper})", 0)
        for name, u1, u2 in ARRAYS2:
            self.emit(f"DIM {name}({u1}, {u2})", 0)

    def emit_dump(self):
        self.emit('PRINT "-- dump --"', 0)
        self.emit("PRINT " + "; ".join(NUMVARS), 0)
        self.emit("PRINT " + '; "|"; '.join(STRVARS), 0)
        for name, upper in ARRAYS:
            self.emit(f"FOR F9 = 0 TO {upper}", 0)
            self.emit(f"PRINT {name}(F9);", 1)
            self.emit("NEXT F9", 0)
            self.emit("PRINT", 0)
        for name, u1, u2 in ARRAYS2:
            self.emit(f"FOR F9 = 0 TO {u1}", 0)
            self.emit(f"FOR F8 = 0 TO {u2}", 1)
            self.emit(f"PRINT {name}(F9, F8);", 2)
            self.emit("NEXT F8", 1)
            self.emit("NEXT F9", 0)
            self.emit("PRINT", 0)

    def program(self):
        self.emit("' fuzz-generated program (genfuzz.py, mode A)", 0)
        self.emit_dims()
        self.n_subs = self.r.randint(0, 3)
        self.stmt_budget = 60
        self.gen_block(0, 0, self.r.randint(12, 22))
        self.emit_dump()
        self.emit("END", 0)
        # GOSUB subroutines: strictly-downward calls guarantee termination.
        for k in range(1, self.n_subs + 1):
            self.emit(f"SUB{k}:", 0)
            self.sub_level = k
            saved = self.stmt_budget
            self.stmt_budget = 6
            for _ in range(self.r.randint(1, 4)):
                c = self.r.random()
                if c < 0.55:
                    self.gen_assign(1)
                elif c < 0.75:
                    self.gen_print(1)
                elif c < 0.85:
                    self.gen_swap(1)
                else:
                    self.gen_gosub(1)
            self.stmt_budget = saved
            self.emit("RETURN", 0)
        self.sub_level = 0
        return "\n".join(self.lines) + "\n"


class GenFlat:
    """Mode B: flat line-numbered program with forward GOTO / IF…GOTO /
    numeric GOSUB — targets the __pc state-machine emitter. Reuses Gen for
    expression generation."""

    def __init__(self, seed):
        self.g = Gen(seed)
        self.g.use_2d = False
        self.r = self.g.r

    def program(self):
        g, r = self.g, self.r
        stmts = []          # main statement texts (line numbers assigned after)

        def flat_stmt():
            c = r.random()
            if c < 0.5:
                if r.random() < 0.75:
                    tgt = (r.choice(NUMVARS) if r.random() < 0.6
                           else g.arr1_ref())
                    return f"{tgt} = ({g.nexpr(0)}) MOD 32749"
                v = r.choice(STRVARS)
                return f"{v} = LEFT$({g.sexpr(0)}, 40)"
            if c < 0.7:
                n = r.randint(1, 3)
                parts = [g.nexpr(1) if r.random() < 0.6 else g.sexpr(1)
                         for _ in range(n)]
                return "PRINT " + "; ".join(parts)
            if c < 0.8:
                return f"SWAP {r.choice(NUMVARS)}, {r.choice(NUMVARS)}"
            return None     # jump slot — filled below

        n_main = r.randint(14, 24)
        for _ in range(n_main):
            stmts.append(flat_stmt())

        # Line numbering: DIMs first, then main, dump, END, subs.
        out = ["' fuzz-generated program (genfuzz.py, mode B: line-numbered)"]
        num = 10

        def line(text):
            nonlocal num
            out.append(f"{num} {text}")
            num += 10

        for name, upper in ARRAYS:
            line(f"DIM {name}({upper})")
        main_first = num
        main_nums = [main_first + 10 * i for i in range(len(stmts))]
        dump_first = main_first + 10 * len(stmts)

        # Sub region line numbers (after END): decide count now so GOSUBs
        # can target them.
        n_subs = r.randint(0, 2)
        # dump block: 3 fixed lines + per-array prints (flat, no FOR)
        dump_len = 3 + len(ARRAYS)
        end_num = dump_first + 10 * dump_len
        sub_nums = []
        s = end_num + 10
        for _ in range(n_subs):
            body = r.randint(1, 3)
            sub_nums.append((s, body))
            s += 10 * (body + 1)    # body lines + RETURN

        for i, st in enumerate(stmts):
            this = main_nums[i]
            if st is None:
                # jump slot: forward GOTO / IF…GOTO to a later main line or
                # the dump; or a GOSUB into the sub region.
                c = r.random()
                later = [n for n in main_nums if n > this] + [dump_first]
                tgt = r.choice(later)
                if c < 0.25 and sub_nums:
                    st = f"GOSUB {r.choice(sub_nums)[0]}"
                elif c < 0.55:
                    st = f"IF {g.cond()} THEN GOTO {tgt}"
                elif c < 0.7:
                    st = f"GOTO {tgt}"
                else:
                    st = f"IF {g.cond()} THEN {r.choice(NUMVARS)} = ({g.nexpr(1)}) MOD 32749"
            out.append(f"{this} {st}")
        num = dump_first

        line('PRINT "-- dump --"')
        line("PRINT " + "; ".join(NUMVARS))
        line("PRINT " + '; "|"; '.join(STRVARS))
        for name, upper in ARRAYS:
            elems = "; ".join(f"{name}({k})" for k in range(0, upper + 1, 3))
            line(f"PRINT {elems}")
        assert num == end_num, (num, end_num)
        line("END")
        for start, body in sub_nums:
            assert num == start, (num, start)
            for _ in range(body):
                line(flat_stmt() or f"{r.choice(NUMVARS)} = ({g.nexpr(1)}) MOD 32749")
            line("RETURN")
        return "\n".join(out) + "\n"


if __name__ == "__main__":
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    if seed % 3 == 0:
        sys.stdout.write(GenFlat(seed).program())
    else:
        sys.stdout.write(Gen(seed).program())

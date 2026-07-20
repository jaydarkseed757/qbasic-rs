#!/usr/bin/env python3
"""qbref — reference interpreter for the genfuzz.py QBasic subset.

An INDEPENDENT implementation of QB semantics used as the differential-fuzzing
oracle against qbc-transpiled native binaries. Python floats are IEEE f64, so
arithmetic matches the transpiler's all-numerics-are-f64 model bit-for-bit.

Semantics implemented (QBasic 1.1):
- precedence: ^ (left-assoc) > unary- > * / > \\ > MOD > + - > relational
  > NOT > AND > OR > XOR (EQV/IMP not generated)
- \\ and MOD round both operands with CINT (banker's; Python round() matches),
  then i64 ops: \\ truncates toward zero, MOD takes the dividend's sign
- comparisons yield -1.0 / 0.0 (string comparisons compare ASCII, same as
  Python); NOT/AND/OR/XOR are bitwise on int(v)
- PRINT: numbers as " n " / "-n " (trailing space always), `,` advances to the
  next 14-column zone, trailing `;` suppresses the newline
- PRINT USING plain `#` fields: right-justify `[-]digits` in the field width,
  `%`-prefix on overflow (only `#` runs + literal text are generated)
- SWAP exchanges two lvalues (scalar or array element, numeric or string)
- strings are 1-indexed; LEFT$/RIGHT$/MID$ clamp like QB
- undefined variables read as 0 / ""; DIM arr(n)/arr(n, m) allocates 0..n
- mode A: GOSUB <name> subroutines after END (parsed as label blocks)
- mode B: flat line-numbered programs (GOTO/IF…THEN GOTO/GOSUB <line>) run
  by a program-counter executor

Usage: qbref.py prog.bas
"""
import sys

MAXSTEPS = 5_000_000


class Halt(Exception):
    pass


class EndProgram(Exception):
    pass


# ── Lexer ────────────────────────────────────────────────────────────────────
def tokenize(s):
    toks = []
    i, n = 0, len(s)
    while i < n:
        c = s[i]
        if c == ' ':
            i += 1
            continue
        if c == '"':
            j = s.index('"', i + 1)
            toks.append(('str', s[i + 1:j]))
            i = j + 1
            continue
        if c.isdigit():
            j = i
            while j < n and s[j].isdigit():
                j += 1
            toks.append(('num', float(s[i:j])))
            i = j
            continue
        if c.isalpha():
            j = i
            while j < n and (s[j].isalnum() or s[j] == '$'):
                j += 1
                if s[j - 1] == '$':
                    break
            toks.append(('id', s[i:j].upper()))
            i = j
            continue
        for op in ('<>', '<=', '>='):
            if s.startswith(op, i):
                toks.append(('op', op))
                i += 2
                break
        else:
            toks.append(('op', c))
            i += 1
    toks.append(('end', ''))
    return toks


# ── Expression parser/evaluator (recursive descent, QB precedence) ──────────
class Expr:
    def __init__(self, toks, env):
        self.t = toks
        self.i = 0
        self.env = env

    def peek(self):
        return self.t[self.i]

    def take(self):
        tok = self.t[self.i]
        self.i += 1
        return tok

    def expect_op(self, op):
        k, v = self.take()
        assert k == 'op' and v == op, f"expected {op}, got {v}"

    def parse(self):
        return self.p_imp()

    def p_imp(self):
        v = self.p_eqv()
        while self.peek() == ('id', 'IMP'):
            self.take()
            v = float(~int(v) | int(self.p_eqv()))
        return v

    def p_eqv(self):
        v = self.p_xor()
        while self.peek() == ('id', 'EQV'):
            self.take()
            v = float(~(int(v) ^ int(self.p_xor())))
        return v

    def p_xor(self):
        v = self.p_or()
        while self.peek() == ('id', 'XOR'):
            self.take()
            v = float(int(v) ^ int(self.p_or()))
        return v

    def p_or(self):
        v = self.p_and()
        while self.peek() == ('id', 'OR'):
            self.take()
            v = float(int(v) | int(self.p_and()))
        return v

    def p_and(self):
        v = self.p_not()
        while self.peek() == ('id', 'AND'):
            self.take()
            v = float(int(v) & int(self.p_not()))
        return v

    def p_not(self):
        if self.peek() == ('id', 'NOT'):
            self.take()
            return float(~int(self.p_not()))
        return self.p_rel()

    def p_rel(self):
        v = self.p_add()
        k, o = self.peek()
        if k == 'op' and o in ('=', '<>', '<', '>', '<=', '>='):
            self.take()
            w = self.p_add()
            r = {'=': v == w, '<>': v != w, '<': v < w,
                 '>': v > w, '<=': v <= w, '>=': v >= w}[o]
            return -1.0 if r else 0.0
        return v

    def p_add(self):
        v = self.p_mod()
        while True:
            k, o = self.peek()
            if k == 'op' and o == '+':
                self.take()
                v = v + self.p_mod()  # numeric add or string concat
            elif k == 'op' and o == '-' and not isinstance(v, str):
                self.take()
                v = v - self.p_mod()
            else:
                return v

    def p_mod(self):
        v = self.p_idiv()
        while self.peek() == ('id', 'MOD'):
            self.take()
            w = self.p_idiv()
            a, b = cint(v), cint(w)
            r = abs(a) % abs(b)
            v = float(-r if a < 0 else r)
        return v

    def p_idiv(self):
        v = self.p_mul()
        while self.peek() == ('op', '\\'):
            self.take()
            w = self.p_mul()
            a, b = cint(v), cint(w)
            q = abs(a) // abs(b)
            v = float(-q if (a < 0) != (b < 0) else q)
        return v

    def p_mul(self):
        v = self.p_neg()
        while True:
            k, o = self.peek()
            if k == 'op' and o == '*':
                self.take()
                v = v * self.p_neg()
            elif k == 'op' and o == '/':
                self.take()
                v = v / self.p_neg()
            else:
                return v

    def p_neg(self):
        if self.peek() == ('op', '-'):
            self.take()
            return -self.p_neg()
        return self.p_pow()

    def p_pow(self):
        v = self.p_primary()
        while self.peek() == ('op', '^'):
            self.take()
            # ^ is LEFT-assoc in QB; a unary sign on the exponent binds tight
            if self.peek() == ('op', '-'):
                self.take()
                v = v ** -self.p_primary()
            else:
                v = v ** self.p_primary()
        return v

    def p_primary(self):
        k, v = self.take()
        if k == 'num':
            return v
        if k == 'str':
            return v
        if k == 'op' and v == '(':
            e = self.parse()
            self.expect_op(')')
            return e
        if k == 'id':
            return self.p_ident(v)
        raise AssertionError(f"unexpected token {k}:{v}")

    def args(self):
        self.expect_op('(')
        out = [self.parse()]
        while self.peek() == ('op', ','):
            self.take()
            out.append(self.parse())
        self.expect_op(')')
        return out

    def p_ident(self, name):
        env = self.env
        if name == 'ABS':
            return abs(self.args()[0])
        if name == 'SGN':
            x = self.args()[0]
            return 1.0 if x > 0 else (-1.0 if x < 0 else 0.0)
        if name == 'INT':
            import math
            return float(math.floor(self.args()[0]))
        if name == 'FIX':
            import math
            return float(math.trunc(self.args()[0]))
        if name == 'CINT':
            return float(cint(self.args()[0]))
        if name == 'LEN':
            return float(len(self.args()[0]))
        if name == 'ASC':
            s = self.args()[0]
            return float(ord(s[0])) if s else 0.0
        if name == 'INSTR':
            a = self.args()
            s1, s2 = a[-2], a[-1]
            start = int(a[0]) if len(a) == 3 else 1
            if start < 1:
                start = 1        # QB treats start < 1 as 1
            if start > len(s1):
                return 0.0       # start past the end → 0
            if not s2:
                return float(start)   # null needle → start
            p = s1.find(s2, start - 1)
            return float(p + 1)
        if name == 'CHR$':
            return chr(int(self.args()[0]))
        if name == 'STR$':
            a = self.args()[0]
            s = fmt_num(a)
            return (' ' + s) if a >= 0 else s
        if name == 'LEFT$':
            s, l = self.args()
            return s[:max(int(l), 0)]
        if name == 'RIGHT$':
            s, l = self.args()
            l = max(int(l), 0)
            return s[len(s) - min(l, len(s)):]
        if name == 'MID$':
            a = self.args()
            s, pos = a[0], int(a[1])
            start = min(max(pos - 1, 0), len(s))
            rest = s[start:]
            return rest[:max(int(a[2]), 0)] if len(a) == 3 else rest
        if name == 'UCASE$':
            return self.args()[0].upper()
        if name == 'LCASE$':
            return self.args()[0].lower()
        # Array element or scalar variable
        if self.peek() == ('op', '('):
            idx = tuple(int(x) for x in self.args())
            return env.get_arr(name, idx)
        if name.endswith('$'):
            return env.svars.get(name, '')
        return env.nvars.get(name, 0.0)


def cint(x):
    return int(round(x))  # Python round() = banker's rounding, matching QB CINT


def fmt_num(n):
    if n == float('inf'):
        return '1E+38'
    if n != n:
        return '0'
    if n == int(n) and abs(n) < 1e15:
        return str(int(n))
    return repr(n)


# ── Interpreter core ────────────────────────────────────────────────────────
class Env:
    def __init__(self):
        self.nvars = {}
        self.svars = {}
        self.arrays = {}     # name → dict[(idx tuple)] (defaults 0.0)
        self.out = []
        self.col = 1         # 1-based print cursor column
        self.steps = 0

    def tick(self):
        self.steps += 1
        if self.steps > MAXSTEPS:
            raise Halt()

    def write(self, s):
        self.out.append(s)
        nl = s.rfind('\n')
        if nl >= 0:
            self.col = len(s) - nl
        else:
            self.col += len(s)

    def get_arr(self, name, idx):
        return self.arrays.setdefault(name, {}).get(idx, 0.0)

    def set_arr(self, name, idx, v):
        self.arrays.setdefault(name, {})[idx] = v


def ev(env, src):
    return Expr(tokenize(src), env).parse()


# ── lvalues (assignment / SWAP operands) ────────────────────────────────────
def lv_ref(env, text):
    """Resolve an lvalue to a stable reference — the index expressions are
    evaluated exactly ONCE here (QB computes operand addresses before any
    exchange/assignment; SWAP must not re-evaluate an index whose variables
    it just changed)."""
    t = text.strip()
    tu = t.upper()
    if '(' in t:
        name = tu[:tu.index('(')].strip()
        idx = tuple(int(x) for x in
                    Expr(tokenize(t[t.index('('):]), env).args())
        return ('arr', name, idx)
    if tu.endswith('$'):
        return ('str', tu, None)
    return ('num', tu, None)


def ref_get(env, ref):
    kind, name, idx = ref
    if kind == 'arr':
        return env.get_arr(name, idx)
    if kind == 'str':
        return env.svars.get(name, '')
    return env.nvars.get(name, 0.0)


def ref_set(env, ref, v):
    kind, name, idx = ref
    if kind == 'arr':
        env.set_arr(name, idx, v)
    elif kind == 'str':
        env.svars[name] = v
    else:
        env.nvars[name] = v


def exec_mid_assign(env, arg):
    """MID$(V$, pos[, len]) = val — in-place replacement, length preserved
    (mirrors qb_mid_assign: no-op past the end; replaces at most min(len,
    remaining) chars, and only as many as val provides)."""
    eq = find_top_eq(arg)
    lhs, rhs = arg[:eq].strip(), arg[eq + 1:].strip()
    inner = lhs[lhs.index('(') + 1:lhs.rindex(')')]
    parts, _ = split_top(inner, ',')
    var = parts[0].strip().upper()
    pos = int(ev(env, parts[1]))
    ln = int(ev(env, parts[2])) if len(parts) == 3 else None
    val = ev(env, rhs)
    s0 = list(env.svars.get(var, ''))
    start = max(pos - 1, 0)
    if start >= len(s0):
        return
    max_replace = len(s0) - start
    replace_len = min(ln, max_replace) if ln is not None else max_replace
    for i, c in enumerate(val[:replace_len]):
        s0[start + i] = c
    env.svars[var] = ''.join(s0)


def lv_get(env, text):
    return ref_get(env, lv_ref(env, text))


def lv_set(env, text, v):
    ref_set(env, lv_ref(env, text), v)


# ── PRINT ───────────────────────────────────────────────────────────────────
def split_top(s, seps):
    """Split on top-level separator chars (respecting quotes/parens)."""
    items, kinds = [], []
    depth, instr, cur = 0, False, ''
    for c in s:
        if instr:
            cur += c
            if c == '"':
                instr = False
            continue
        if c == '"':
            instr = True
            cur += c
        elif c == '(':
            depth += 1
            cur += c
        elif c == ')':
            depth -= 1
            cur += c
        elif depth == 0 and c in seps:
            items.append(cur.strip())
            kinds.append(c)
            cur = ''
        else:
            cur += c
    if cur.strip():
        items.append(cur.strip())
        kinds.append(None)
    elif kinds:
        kinds[-1] = kinds[-1] or None
    return items, kinds


def exec_print(env, arg):
    arg = arg.strip()
    if not arg:
        env.write('\n')
        return
    items, seps = split_top(arg, ';,')
    for i, item in enumerate(items):
        if item:
            v = ev(env, item)
            if isinstance(v, str):
                env.write(v)
            else:
                s = fmt_num(v)
                env.write((' ' + s + ' ') if v >= 0 else (s + ' '))
        sep = seps[i] if i < len(seps) else None
        if sep == ',':
            nxt = ((env.col - 1) // 14 + 1) * 14 + 1
            env.write(' ' * (nxt - env.col))
    if seps and seps[-1] in (';', ','):
        return
    env.write('\n')


def exec_print_using(env, arg):
    """PRINT USING "fmt"; e1[; e2][;] — plain `#` runs + literal text only."""
    a = arg.strip()
    assert a.startswith('"')
    endq = a.index('"', 1)
    fmt = a[1:endq]
    rest = a[endq + 1:].strip()
    assert rest.startswith(';')
    rest = rest[1:]
    items, seps = split_top(rest, ';')
    vals = [ev(env, it) for it in items if it]
    trailing = bool(seps) and seps[-1] == ';'

    out = []
    vi = 0
    i = 0
    while i < len(fmt):
        if fmt[i] == '#':
            w = 0
            while i < len(fmt) and fmt[i] == '#':
                w += 1
                i += 1
            v = vals[vi] if vi < len(vals) else 0.0
            vi += 1
            digits = str(int(abs(v)))
            signed = ('-' if v < 0 else '') + digits
            if len(digits) > w:
                out.append('%' + signed)
            else:
                out.append(' ' * (w - len(signed)) + signed)
        else:
            out.append(fmt[i])
            i += 1
    env.write(''.join(out))
    if not trailing:
        env.write('\n')


# ── Structured (mode A) parser + executor ───────────────────────────────────
def parse_block(lines, i, terminators):
    block = []
    while i < len(lines):
        raw = lines[i].strip()
        up = raw.upper()
        if any(up == t or up.startswith(t + ' ') for t in terminators):
            return block, i
        i += 1
        if not raw or raw.startswith("'"):
            continue
        if up == 'END':
            block.append(('end',))
        elif up.startswith('DIM '):
            block.append(('dim', raw[4:]))
        elif up.startswith('PRINT USING'):
            block.append(('printusing', raw[11:]))
        elif up.startswith('PRINT'):
            block.append(('print', raw[5:]))
        elif up.startswith('SWAP '):
            a, b = split_top(raw[5:], ',')[0]
            block.append(('swap', a, b))
        elif up.startswith('MID$('):
            block.append(('midassign', raw))
        elif up.startswith('GOSUB '):
            block.append(('gosub', raw[6:].strip().upper()))
        elif up.startswith('IF '):
            arms = []
            cond = raw[3:raw.upper().rindex(' THEN')]
            body, i = parse_block(lines, i, ['ELSEIF', 'ELSE', 'END IF'])
            arms.append((cond, body))
            while lines[i].strip().upper().startswith('ELSEIF'):
                l2 = lines[i].strip()
                c2 = l2[7:l2.upper().rindex(' THEN')]
                i += 1
                b2, i = parse_block(lines, i, ['ELSEIF', 'ELSE', 'END IF'])
                arms.append((c2, b2))
            els = []
            if lines[i].strip().upper() == 'ELSE':
                i += 1
                els, i = parse_block(lines, i, ['END IF'])
            i += 1
            block.append(('if', arms, els))
        elif up.startswith('FOR '):
            body_line = raw[4:]
            var, rest = body_line.split('=', 1)
            ru = rest.upper()
            step = '1'
            if ' STEP ' in ru:
                sp = ru.rindex(' STEP ')
                step = rest[sp + 6:]
                rest = rest[:sp]
            to = rest.upper().rindex(' TO ')
            a, b = rest[:to], rest[to + 4:]
            body, i = parse_block(lines, i, ['NEXT'])
            i += 1
            block.append(('for', var.strip().upper(), a, b, step, body))
        elif up.startswith('WHILE'):
            body, i = parse_block(lines, i, ['WEND'])
            i += 1
            block.append(('while', raw[5:], body))
        elif up == 'DO' or up.startswith('DO '):
            head = raw[2:].strip()
            body, i = parse_block(lines, i, ['LOOP'])
            tail = lines[i].strip()[4:].strip()
            i += 1
            block.append(('do', head, body, tail))
        elif up.startswith('SELECT CASE'):
            sel = raw[11:]
            cases = []
            while not lines[i].strip().upper().startswith('CASE'):
                i += 1
            while lines[i].strip().upper().startswith('CASE'):
                spec = lines[i].strip()[4:].strip()
                i += 1
                b, i = parse_block(lines, i, ['CASE', 'END SELECT'])
                cases.append((spec, b))
            i += 1
            block.append(('select', sel, cases))
        else:
            eq = find_top_eq(raw)
            block.append(('let', raw[:eq].strip(), raw[eq + 1:].strip()))
    return block, i


def find_top_eq(s):
    depth, instr = 0, False
    for i, c in enumerate(s):
        if instr:
            if c == '"':
                instr = False
        elif c == '"':
            instr = True
        elif c == '(':
            depth += 1
        elif c == ')':
            depth -= 1
        elif c == '=' and depth == 0:
            return i
    raise AssertionError(f"no top-level = in {s!r}")


def run_block(env, block, subs):
    for stmt in block:
        env.tick()
        op = stmt[0]
        if op == 'end':
            raise EndProgram()
        elif op == 'dim':
            spec = stmt[1]
            name = spec[:spec.index('(')].strip().upper()
            env.arrays.setdefault(name, {})
        elif op == 'let':
            lv_set(env, stmt[1], ev(env, stmt[2]))
        elif op == 'swap':
            ra, rb = lv_ref(env, stmt[1]), lv_ref(env, stmt[2])
            va, vb = ref_get(env, ra), ref_get(env, rb)
            ref_set(env, ra, vb)
            ref_set(env, rb, va)
        elif op == 'midassign':
            exec_mid_assign(env, stmt[1])
        elif op == 'print':
            exec_print(env, stmt[1])
        elif op == 'printusing':
            exec_print_using(env, stmt[1])
        elif op == 'gosub':
            run_block(env, subs[stmt[1]], subs)
        elif op == 'if':
            done = False
            for cond, body in stmt[1]:
                if ev(env, cond) != 0.0:
                    run_block(env, body, subs)
                    done = True
                    break
            if not done:
                run_block(env, stmt[2], subs)
        elif op == 'for':
            var, a, b, s, body = stmt[1], stmt[2], stmt[3], stmt[4], stmt[5]
            start, stop, step = ev(env, a), ev(env, b), ev(env, s)
            env.nvars[var] = start
            while ((step > 0 and env.nvars[var] <= stop)
                   or (step < 0 and env.nvars[var] >= stop)):
                env.tick()
                run_block(env, body, subs)
                env.nvars[var] += step
        elif op == 'while':
            while ev(env, stmt[1]) != 0.0:
                env.tick()
                run_block(env, stmt[2], subs)
        elif op == 'do':
            head, body, tail = stmt[1], stmt[2], stmt[3]

            def head_ok():
                h = head.upper()
                if h.startswith('WHILE'):
                    return ev(env, head[5:]) != 0.0
                if h.startswith('UNTIL'):
                    return ev(env, head[5:]) == 0.0
                return True

            def tail_ok():
                t = tail.upper()
                if t.startswith('WHILE'):
                    return ev(env, tail[5:]) != 0.0
                if t.startswith('UNTIL'):
                    return ev(env, tail[5:]) == 0.0
                return not head
            while head_ok():
                env.tick()
                run_block(env, body, subs)
                if tail:
                    if not tail_ok():
                        break
                elif not head:
                    break
        elif op == 'select':
            sel = ev(env, stmt[1])
            done = False
            default = None
            for spec, body in stmt[2]:
                su = spec.upper()
                if su == 'ELSE':
                    default = body
                    continue
                if su.startswith('IS'):
                    rest = spec[2:].strip()
                    o = rest[:2] if rest[:2] in ('<=', '>=', '<>') else rest[0]
                    w = ev(env, rest[len(o):])
                    hit = {'<': sel < w, '>': sel > w, '<=': sel <= w,
                           '>=': sel >= w, '<>': sel != w, '=': sel == w}[o]
                elif ' TO ' in su:
                    t = su.index(' TO ')
                    lo = ev(env, spec[:t])
                    hi = ev(env, spec[t + 4:])
                    hit = lo <= sel <= hi
                else:
                    hit = any(ev(env, p) == sel for p in spec.split(','))
                if hit:
                    run_block(env, body, subs)
                    done = True
                    break
            if not done and default is not None:
                run_block(env, default, subs)


def run_structured(src):
    lines = src.splitlines()
    # Split at top-level END: main block, then `NAME:` label blocks to RETURN.
    end_idx = None
    for i, l in enumerate(lines):
        if l.strip().upper() == 'END':
            end_idx = i
            break
    main_lines = lines[:end_idx] if end_idx is not None else lines
    subs = {}
    if end_idx is not None:
        i = end_idx + 1
        while i < len(lines):
            lab = lines[i].strip()
            if not lab or lab.startswith("'"):
                i += 1
                continue
            assert lab.endswith(':'), f"expected label, got {lab!r}"
            name = lab[:-1].upper()
            i += 1
            body_lines = []
            while lines[i].strip().upper() != 'RETURN':
                body_lines.append(lines[i])
                i += 1
            i += 1
            subs[name], _ = parse_block(body_lines, 0, [])
    block, _ = parse_block(main_lines, 0, [])
    env = Env()
    try:
        run_block(env, block, subs)
    except EndProgram:
        pass
    return env


# ── Flat line-numbered (mode B) executor ────────────────────────────────────
def run_flat(src):
    prog = []          # list of (line_number, stmt_text)
    for raw in src.splitlines():
        t = raw.strip()
        if not t or t.startswith("'"):
            continue
        num, rest = t.split(' ', 1)
        prog.append((int(num), rest.strip()))
    index = {num: k for k, (num, _) in enumerate(prog)}

    env = Env()
    stack = []
    pc = 0
    while pc < len(prog):
        env.tick()
        _, st = prog[pc]
        up = st.upper()
        nxt = pc + 1
        if up == 'END':
            break
        elif up == 'RETURN':
            nxt = stack.pop()
        elif up.startswith('DIM '):
            spec = st[4:]
            env.arrays.setdefault(spec[:spec.index('(')].strip().upper(), {})
        elif up.startswith('GOTO '):
            nxt = index[int(st[5:])]
        elif up.startswith('GOSUB '):
            stack.append(pc + 1)
            nxt = index[int(st[6:])]
        elif up.startswith('IF '):
            thn = up.rindex(' THEN')
            cond = st[3:thn]
            action = st[thn + 5:].strip()
            if ev(env, cond) != 0.0:
                au = action.upper()
                if au.startswith('GOTO '):
                    nxt = index[int(action[5:])]
                else:
                    eq = find_top_eq(action)
                    lv_set(env, action[:eq], ev(env, action[eq + 1:]))
        elif up.startswith('PRINT USING'):
            exec_print_using(env, st[11:])
        elif up.startswith('PRINT'):
            exec_print(env, st[5:])
        elif up.startswith('SWAP '):
            a, b = split_top(st[5:], ',')[0]
            ra, rb = lv_ref(env, a), lv_ref(env, b)
            va, vb = ref_get(env, ra), ref_get(env, rb)
            ref_set(env, ra, vb)
            ref_set(env, rb, va)
        elif up.startswith('MID$('):
            exec_mid_assign(env, st)
        else:
            eq = find_top_eq(st)
            lv_set(env, st[:eq], ev(env, st[eq + 1:]))
        pc = nxt
    return env


def main():
    src = open(sys.argv[1]).read()
    first_code = next((l.strip() for l in src.splitlines()
                       if l.strip() and not l.strip().startswith("'")), '')
    try:
        if first_code[:1].isdigit():
            env = run_flat(src)
        else:
            env = run_structured(src)
    except Halt:
        sys.stderr.write("qbref: step budget exhausted\n")
        sys.exit(2)
    sys.stdout.write(''.join(env.out))


if __name__ == '__main__':
    main()

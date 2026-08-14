' Regression test: a SUB/FUNCTION parameter is legal QB even when it shares
' its name (case-insensitively) with a module-level CONST -- it simply
' shadows the CONST within that one SUB/FUNCTION, same as any other local.
' Naively emitted, this breaks Rust: a plain identifier PATTERN (a fn
' parameter or `let` binding) that names a visible `const` item is treated
' by rustc as a refutable pattern MATCHING that constant, not a fresh
' binding, so `fn f(cx: &mut f64)` fails to compile when `const cx: f64` is
' also in scope -- even though ordinary expression-position reads of the
' name are completely unambiguous. Found via basic-src/orbits.bas.
' Covers: plain numeric SUB param, numeric FUNCTION param, and a scalar TYPE
' param whose flattened field name collides (Nudge's `p` vs CONST P__CX --
' deliberately using a DIFFERENTLY-named local `pt` in main so this isolates
' the PARAMETER collision; a plain local variable colliding with a CONST
' this way is a separate, still-open gap -- see CLAUDE.md's Known Issues).

CONST CX = 160
CONST NAME = 5
CONST P__CX = 99  ' collides with Nudge's Pt-param p's flattened field p__cx

TYPE Pt
    CX AS SINGLE
    Y AS SINGLE
END TYPE

DECLARE SUB ShowNum (cx AS SINGLE)
DECLARE FUNCTION Doubled% (cx AS SINGLE)
DECLARE SUB Nudge (p AS Pt)

PRINT CX; NAME; P__CX

CALL ShowNum(42)
PRINT Doubled%(21)

DIM pt AS Pt
pt.CX = 7
pt.Y = 9
CALL Nudge(pt)
PRINT pt.CX; pt.Y

SUB ShowNum (cx AS SINGLE)
    cx = cx + 1
    PRINT cx
END SUB

FUNCTION Doubled% (cx AS SINGLE)
    Doubled% = cx * 2
END FUNCTION

SUB Nudge (p AS Pt)
    p.CX = p.CX + 1
    p.Y = p.Y + 1
END SUB

' Regression: identifier collisions with a module-level CONST.
'
' Rust refuses `let cx = ...` while a `const cx` is in scope (E0530), and
' treats a bare identifier PATTERN naming a visible const as a match rather
' than a fresh binding — but QB is happy to let a procedure-local (or a
' parameter) shadow a module CONST. Both halves need the emitted name
' renamed at the declaration AND at every use.
'
' Parameters were fixed earlier (disambig/_p + value_params). This covers
' the `let`-binding twin: a SUB-local DIM shadowing a CONST.
'
' Also covers SUB Foo(t, t$) — QB gives the procedure TWO distinct params
' sharing a base name. A bare `t` parses as Single, so it must resolve to
' the numeric param, not to the string one.

DECLARE SUB ShowLocal ()
DECLARE SUB Both (t, t$)

CONST CX = 160

PRINT "module CX ="; CX
CALL ShowLocal
PRINT "module CX still ="; CX
CALL Both(5, "hi")

SUB ShowLocal
  ' Procedure-local shadowing the module CONST — legal QB.
  DIM cx AS INTEGER
  cx = 5
  cx = cx + 2
  PRINT "local cx ="; cx
END SUB

SUB Both (t, t$)
  PRINT "num ="; t
  PRINT "str ="; t$
  t = t + 1
  t$ = t$ + "!"
  PRINT "num2 ="; t
  PRINT "str2 ="; t$
END SUB

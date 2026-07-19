' Regression test: a sigiled DIM t$ declares a DISTINCT QB variable from the
' numeric t in the same DIM statement (mario.bas title screen). The emitter
' must not classify bare-t uses as string, and must declare the sigiled
' variable under its typed name (t_s), in both SUB and main-body scopes.
DECLARE SUB Demo ()

CALL Demo

DIM n, n$
n = 7
n$ = "main-" + STR$(n)
n = n * 2
PRINT n
PRINT n$
PRINT "done"

SUB Demo
    DIM t, mf, t$
    t = 41
    t = t + 1
    t$ = "sub-" + STR$(t)
    PRINT t
    PRINT t$
    IF t MOD 2 = 0 THEN PRINT "even" ELSE PRINT "odd"
END SUB

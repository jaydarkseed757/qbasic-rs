' Regression test: purely-local sigil-less `DIM x AS STRING` scalars.
' (1) Assignment to a local string scalar must get the .to_string() treatment
'     -- was emitting a bare assignment against a String binding (E0308).
' (2) The emitter's per-scope bookkeeping must not leak between scopes: a
'     numeric local reusing a bare name that was STRING-typed inside an
'     earlier-processed SUB must not be misclassified as a string.
DECLARE SUB SetN ()

CALL SetN
PRINT "sub done"

' Numeric local reusing a bare name that is STRING-typed inside SetN below.
' If the emitter's scope-reset were missing, this would be misclassified.
DIM n AS INTEGER
n = 42
PRINT n

' Main-body local string scalar -- the primary fix under test.
DIM s AS STRING
s = "world"
PRINT "hello " + s
IF s = "" THEN PRINT "empty" ELSE PRINT "not empty"
s = ""
IF s = "" THEN PRINT "now empty"

PRINT "done"

SUB SetN
    DIM n AS STRING
    n = "sub-local"
    PRINT n
END SUB

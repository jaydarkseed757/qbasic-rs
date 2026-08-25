' Regression: array operations that used to emit Rust which wouldn't compile
' (or, for the REDIM case, compiled but panicked at runtime).
'
'  - ERASE on a LOCAL string array emitted the un-suffixed name with a 0.0
'    default (E0425 + E0308) — local arrays were always assumed numeric.
'  - ERASE on a SUB-local 2-D array emitted one iter_mut level too few
'    (E0308), because array_dims only ever knew about GLOBAL arrays.
'  - REDIM growing an INNER bound only resized the outer Vec, so the rows
'    kept their old length and indexing the new column panicked.
'  - UBOUND on a SHARED STRING array compared the _s-suffixed name against
'    shared_names (which holds bare names), missed the shared branch and
'    emitted a bare `names_s.len()` with no __gs. (E0425).

DECLARE SUB LocalArrays ()

DIM SHARED names$(5)
names$(1) = "alpha"

' --- UBOUND on a shared string array, in a value context and a condition ---
n = UBOUND(names$) + 1
PRINT "ubound+1="; n
IF UBOUND(names$) > 2 THEN PRINT "shared string array is big"

' --- REDIM growing an inner bound ---
REDIM g(2, 2)
g(2, 2) = 1
REDIM g(2, 5)
g(2, 5) = 7
PRINT "inner grown="; g(2, 5)

CALL LocalArrays
PRINT "done"

SUB LocalArrays
  ' ERASE on a local STRING array
  DIM w$(3)
  w$(1) = "hi"
  ERASE w$
  PRINT "erased str ["; w$(1); "]"

  ' ERASE on a local 2-D numeric array
  DIM grid(3, 3)
  grid(1, 1) = 5
  ERASE grid
  PRINT "erased 2d="; grid(1, 1)
END SUB

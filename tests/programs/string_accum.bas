' Test: T5 string accumulation (s$ = s$ + ... → push_str)
DIM SHARED H$

' Single-term accumulation in a loop
s$ = ""
FOR i = 1 TO 3
  s$ = s$ + CHR$(64 + i)
NEXT i
PRINT s$

' Chain accumulation: r$ = r$ + a + "-"
r$ = ""
FOR i = 1 TO 3
  r$ = r$ + CHR$(64 + i) + "-"
NEXT i
PRINT r$

' Leftmost operand is a DIFFERENT variable — must keep value semantics
m$ = "X"
k$ = "Y"
g$ = m$ + k$
PRINT g$

' Appended term references the LHS — must keep evaluate-then-assign semantics
t$ = "abc"
t$ = t$ + LEFT$(t$, 1)
PRINT t$

' Appended term is a FUNCTION that implicitly reads the shared LHS — must keep
' evaluate-then-assign semantics (Tag$ sees the ORIGINAL H$="A", not "AB")
H$ = "A"
H$ = H$ + "B" + Tag$
PRINT H$

PRINT "done"

FUNCTION Tag$
  Tag$ = "<" + H$ + ">"
END FUNCTION

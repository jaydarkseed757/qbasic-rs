' Regression tests for QB fidelity fixes (code review June 2026):
' operator precedence, ^ associativity, byref array elements,
' multi-counter NEXT, DATA backslash, EQV/IMP, UBOUND/LBOUND on
' string arrays, RND(0)/RND(-n) semantics, QB LCG first value.
DECLARE SUB Swap2 (x, y)

' --- precedence: * / tighter than \ tighter than MOD ---
PRINT 2 * 3 MOD 4
PRINT 10 \ 2 * 3
PRINT 8 MOD 3 \ 2

' --- ^ is left-associative; unary minus looser than ^ ---
PRINT 2 ^ 3 ^ 2
PRINT -2 ^ 2
PRINT 2 ^ -2

' --- array elements pass byref to SUBs ---
DIM a(5)
a(1) = 100
a(2) = 200
CALL Swap2(a(1), a(2))
PRINT a(1); a(2)

' --- multi-counter NEXT closes both loops ---
FOR i = 1 TO 2
FOR j = 1 TO 2
PRINT i * 10 + j;
NEXT j, i
PRINT

' --- DATA strings with backslashes survive ---
DATA "C:\temp\new"
READ s$
PRINT s$

' --- EQV / IMP bitwise operators ---
PRINT 5 EQV 3; 5 IMP 3; 0 IMP 5

' --- UBOUND/LBOUND on a string array ---
DIM w$(1 TO 4)
PRINT UBOUND(w$()); LBOUND(w$())

' --- RND: QB LCG first value, RND(0) repeats, RND(-n) reseeds ---
PRINT INT(RND * 10000000)
x = RND
PRINT RND(0) = x
r1 = RND(-7)
r2 = RND(-7)
PRINT r1 = r2
END

SUB Swap2 (x, y)
t = x
x = y
y = t
END SUB

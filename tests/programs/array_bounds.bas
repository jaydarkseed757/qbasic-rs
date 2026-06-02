' Test: arrays with explicit TO lower bounds
' 1D with non-zero lower bound
DIM a(5 TO 10)
FOR i = 5 TO 10
    a(i) = i * 2
NEXT i
FOR i = 5 TO 10
    PRINT a(i)
NEXT i
PRINT LBOUND(a)
PRINT UBOUND(a)

' 2D with both dims using TO
DIM b(2 TO 3, 4 TO 5)
FOR r = 2 TO 3
    FOR c = 4 TO 5
        b(r, c) = r * 10 + c
    NEXT c
NEXT r
FOR r = 2 TO 3
    FOR c = 4 TO 5
        PRINT b(r, c);
    NEXT c
    PRINT
NEXT r

' Default lower bound (0) still works correctly
DIM zz(3)
FOR i = 0 TO 3
    zz(i) = i + 1
NEXT i
FOR i = 0 TO 3
    PRINT zz(i)
NEXT i
PRINT LBOUND(zz)
PRINT UBOUND(zz)

PRINT "done"

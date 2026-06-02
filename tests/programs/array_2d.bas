' Test: 2D arrays — DIM, read/write, nested FOR
DIM grid(3, 3)

' Fill with row*10 + col
FOR r = 1 TO 3
    FOR c = 1 TO 3
        grid(r, c) = r * 10 + c
    NEXT c
NEXT r

' Print row by row
FOR r = 1 TO 3
    FOR c = 1 TO 3
        PRINT grid(r, c);
    NEXT c
    PRINT
NEXT r

PRINT "done"

DEF FNFib (n)
    IF n < 2 THEN
        FNFib = n
    ELSE
        a = 0 : b = 1
        FOR i = 2 TO n
            t = a + b : a = b : b = t
        NEXT i
        FNFib = b
    END IF
END DEF

FOR k = 0 TO 10
    PRINT FNFib(k);
NEXT k
PRINT

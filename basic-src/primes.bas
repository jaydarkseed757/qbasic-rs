
REM QBC FULLSPEED

DEFINT A-Z

CLS
PRINT "================================="
PRINT "   QBasic Prime Number Finder"
PRINT "================================="
PRINT
PRINT "1. List primes up to N"
PRINT "2. Find the Nth prime"
PRINT
INPUT "Choose (1 or 2, default=1): ", choice$
IF choice$ = "" THEN choice$ = "1"

IF choice$ = "1" THEN
    PRINT
    INPUT "Find primes up to (default=100): ", ans$
    IF ans$ = "" THEN
        limit = 100
    ELSE
        limit = VAL(ans$)
    END IF
    IF limit < 2 THEN limit = 2
    IF limit > 32000 THEN
        PRINT "Capping at 32000 (DEFINT limit)."
        limit = 32000
    END IF
    CALL SieveList(limit)

ELSEIF choice$ = "2" THEN
    PRINT
    INPUT "Which prime do you want (default=20): ", ans$
    IF ans$ = "" THEN
        nth = 20
    ELSE
        nth = VAL(ans$)
    END IF
    IF nth < 1 THEN nth = 1
    CALL FindNth(nth)

ELSE
    PRINT "Invalid choice."
END IF

PRINT
PRINT "Done!"
END

'-------------------------------------------------
' Sieve of Eratosthenes up to limit
'-------------------------------------------------
SUB SieveList (limit)
    DIM sieve(32000) AS INTEGER
    DIM i AS INTEGER, j AS INTEGER
    DIM count AS INTEGER

    ' 0 = prime, 1 = composite
    FOR i = 0 TO limit: sieve(i) = 0: NEXT i
    sieve(0) = 1: sieve(1) = 1

    i = 2
    DO WHILE i * i <= limit
        IF sieve(i) = 0 THEN
            j = i * i
            DO WHILE j <= limit
                sieve(j) = 1
                j = j + i
            LOOP
        END IF
        i = i + 1
    LOOP

    PRINT
    PRINT "Primes up to"; limit; ":"
    PRINT STRING$(33, "-")

    count = 0
    col = 0
    FOR i = 2 TO limit
        IF sieve(i) = 0 THEN
            count = count + 1
            ' Print in columns of 8, width 6 each
            PRINT RIGHT$("     " + LTRIM$(STR$(i)), 6);
            col = col + 1
            IF col = 8 THEN
                PRINT
                col = 0
            END IF
        END IF
    NEXT i
    IF col <> 0 THEN PRINT

    PRINT STRING$(33, "-")
    PRINT "Total primes found:"; count
END SUB

'-------------------------------------------------
' Find the Nth prime by trial division
'-------------------------------------------------
SUB FindNth (nth)
    DIM candidate AS LONG
    DIM count AS INTEGER
    DIM i AS LONG
    DIM isPrime AS INTEGER

    PRINT
    PRINT "Searching for prime #"; nth; "..."

    count = 0
    candidate = 2

    DO
        isPrime = -1   ' true

        IF candidate = 2 THEN
            isPrime = -1
        ELSEIF candidate MOD 2 = 0 THEN
            isPrime = 0
        ELSE
            i = 3
            DO WHILE i * i <= candidate
                IF candidate MOD i = 0 THEN
                    isPrime = 0
                    EXIT DO
                END IF
                i = i + 2
            LOOP
        END IF

        IF isPrime THEN
            count = count + 1
            IF count = nth THEN
                PRINT
                PRINT "Prime #"; nth; "is:"; candidate
                EXIT DO
            END IF
        END IF

        IF candidate = 2 THEN
            candidate = 3
        ELSE
            candidate = candidate + 2
        END IF
    LOOP
END SUB


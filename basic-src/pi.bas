'-------------------------------------------------
' PI.BAS - Pi to N decimal places
REM QBC FULLSPEED
' Uses Machin's formula:
'   pi/4 = 4*arctan(1/5) - arctan(1/239)
' Arbitrary precision via integer arrays
'-------------------------------------------------

DEFINT A-Z

CONST MAXDIG = 1000
CONST BASE = 10000       ' 4 decimal digits per cell

DIM a(MAXDIG), b(MAXDIG), c(MAXDIG), t(MAXDIG)

CLS
PRINT "================================="
PRINT "   QBasic Pi Calculator"
PRINT "   Machin Formula Edition"
PRINT "================================="
PRINT
INPUT "Digits of pi to calculate (default=20): ", ans$

IF ans$ = "" THEN
    ndig = 20
ELSE
    ndig = VAL(ans$)
END IF

IF ndig < 1 THEN ndig = 1
IF ndig > 900 THEN
    PRINT "Capping at 900 digits."
    ndig = 900
END IF

PRINT
PRINT "Computing pi to"; ndig; "decimal places..."
PRINT

' Number of BASE-cells needed
cells = INT((ndig / 4) + 4)

'--- Compute 4 * arctan(1/5) ---
CALL ArcTan(5, cells, a())
CALL ScaleMul(4, cells, a())

'--- Compute arctan(1/239) ---
CALL ArcTan(239, cells, b())

'--- pi/4 = 4*arctan(1/5) - arctan(1/239) ---
CALL SubArr(cells, a(), b(), c())

'--- pi = 4 * (pi/4) ---
CALL ScaleMul(4, cells, c())

'--- Format and print ---
CALL PrintPi(ndig, cells, c())

PRINT
PRINT "Done!"
END

SUB ArcTan (x, cells, arr())
    DIM num(MAXDIG), term(MAXDIG), tmp(MAXDIG)
    DIM i AS INTEGER

    FOR i = 0 TO cells: arr(i) = 0: NEXT i

    FOR i = 0 TO cells: num(i) = 0: NEXT i
    num(0) = BASE \ x

    r = BASE MOD x
    FOR i = 1 TO cells
        val = r * BASE
        num(i) = val \ x
        r = val MOD x
    NEXT i

    FOR i = 0 TO cells: term(i) = num(i): NEXT i
    CALL AddArr(cells, arr(), term())

    denom = 1
    sign = -1
    x2 = x * x

    DO
        CALL ScaleDiv(x2, cells, term())
        denom = denom + 2
        FOR i = 0 TO cells: tmp(i) = term(i): NEXT i
        CALL ScaleDiv(denom, cells, tmp())

        allzero = -1
        FOR i = 0 TO cells
            IF tmp(i) <> 0 THEN allzero = 0: EXIT FOR
        NEXT i
        IF allzero THEN EXIT DO

        IF sign = -1 THEN
            CALL SubArr(cells, arr(), tmp(), arr())
        ELSE
            CALL AddArr(cells, arr(), tmp())
        END IF
        sign = -sign
    LOOP
END SUB

SUB SubArr (cells, a(), b(), out())
    DIM i AS INTEGER
    DIM tmp(MAXDIG)
    FOR i = 0 TO cells: tmp(i) = a(i) - b(i): NEXT i
    FOR i = cells TO 1 STEP -1
        IF tmp(i) < 0 THEN
            tmp(i) = tmp(i) + BASE
            tmp(i - 1) = tmp(i - 1) - 1
        END IF
    NEXT i
    FOR i = 0 TO cells: out(i) = tmp(i): NEXT i
END SUB

SUB AddArr (cells, arr(), b())
    DIM i AS INTEGER
    FOR i = 0 TO cells: arr(i) = arr(i) + b(i): NEXT i
    FOR i = cells TO 1 STEP -1
        IF arr(i) >= BASE THEN
            arr(i - 1) = arr(i - 1) + (arr(i) \ BASE)
            arr(i) = arr(i) MOD BASE
        END IF
    NEXT i
END SUB

SUB ScaleMul (n, cells, arr())
    DIM i AS INTEGER
    FOR i = 0 TO cells: arr(i) = arr(i) * n: NEXT i
    FOR i = cells TO 1 STEP -1
        IF arr(i) >= BASE THEN
            arr(i - 1) = arr(i - 1) + (arr(i) \ BASE)
            arr(i) = arr(i) MOD BASE
        END IF
    NEXT i
END SUB

SUB ScaleDiv (n, cells, arr())
    DIM i AS INTEGER
    r = 0
    FOR i = 0 TO cells
        val = arr(i) + r * BASE
        arr(i) = val \ n
        r = val MOD n
    NEXT i
END SUB

SUB PrintPi (ndig, cells, arr())
    DIM i AS INTEGER
    ' arr(0) holds the scaled value: integer_part = arr(0) \ BASE,
    ' first decimal chunk = arr(0) MOD BASE (4 digits).
    PRINT "pi = "; LTRIM$(STR$(arr(0) \ BASE)); ".";

    digitsleft = ndig
    ' First decimal chunk comes from the low 4 digits of arr(0)
    chunk$ = RIGHT$("0000" + LTRIM$(STR$(arr(0) MOD BASE)), 4)
    IF digitsleft >= 4 THEN
        PRINT chunk$;
        digitsleft = digitsleft - 4
    ELSE
        PRINT LEFT$(chunk$, digitsleft);
        digitsleft = 0
    END IF

    i = 1
    DO WHILE digitsleft > 0 AND i <= cells
        chunk$ = RIGHT$("0000" + LTRIM$(STR$(arr(i))), 4)
        IF digitsleft >= 4 THEN
            PRINT chunk$;
            digitsleft = digitsleft - 4
        ELSE
            PRINT LEFT$(chunk$, digitsleft);
            digitsleft = 0
        END IF
        i = i + 1
    LOOP
    PRINT
END SUB

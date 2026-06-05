' =========================================
' QBASIC 4.5 MEGA TEST
' =========================================

DECLARE FUNCTION Factorial& (N%)
DECLARE FUNCTION Fib& (N%)

CLS
RANDOMIZE TIMER

DIM Numbers(20)
DIM Names$(5)
DIM Grid(10, 10)

DATA ALPHA,BETA,GAMMA,DELTA,OMEGA

FOR I = 1 TO 5
    READ Names$(I)
NEXT I

FOR I = 1 TO 20
    Numbers(I) = INT(RND * 1000)
NEXT I

DO
    CLS
    PRINT "QBASIC 4.5 MEGA TEST"
    PRINT STRING$(40, "=")
    PRINT
    PRINT "1. Array Test"
    PRINT "2. String Test"
    PRINT "3. Graphics Test"
    PRINT "4. Random Test"
    PRINT "5. Sort Test"
    PRINT "6. Math Test"
    PRINT "7. ON GOTO Test"
    PRINT "8. 2D Array Test"
    PRINT "9. Exit"
    PRINT

    INPUT "Choice"; Choice

    ON Choice GOTO ArrayTest, StringTest, GraphicsTest, RandomTest, SortTest, MathTest, OnGotoTest, GridTest, ExitProgram

LOOP

' -----------------------------------------
ArrayTest:
CLS
PRINT "ARRAY TEST"
FOR I = 1 TO 20
    PRINT I; TAB(6); Numbers(I)
NEXT
GOSUB PauseSub
GOTO ContinueLoop

' -----------------------------------------
StringTest:
CLS
PRINT "STRING TEST"
FOR I = 1 TO 5
    A$ = Names$(I)
    PRINT A$
    PRINT " LEFT$  = "; LEFT$(A$, 2)
    PRINT " RIGHT$ = "; RIGHT$(A$, 2)
    PRINT " LEN    = "; LEN(A$)
    PRINT
NEXT
GOSUB PauseSub
GOTO ContinueLoop

' -----------------------------------------
GraphicsTest:
SCREEN 13
CLS

FOR I = 1 TO 200
    X1 = INT(RND * 320)
    Y1 = INT(RND * 200)
    X2 = INT(RND * 320)
    Y2 = INT(RND * 200)
    C = INT(RND * 256)

    LINE (X1, Y1)-(X2, Y2), C
NEXT

FOR I = 1 TO 100
    X = INT(RND * 320)
    Y = INT(RND * 200)
    C = INT(RND * 256)

    CIRCLE (X, Y), INT(RND * 20), C
NEXT

LOCATE 1, 1
PRINT "PRESS ANY KEY"

DO
LOOP UNTIL INKEY$ <> ""

SCREEN 0
WIDTH 80
GOTO ContinueLoop

' -----------------------------------------
RandomTest:
CLS

CountLow = 0
CountHigh = 0

FOR I = 1 TO 1000
    N = INT(RND * 100)

    IF N < 50 THEN
        CountLow = CountLow + 1
    ELSE
        CountHigh = CountHigh + 1
    END IF
NEXT

PRINT "LOW  (<50): "; CountLow
PRINT "HIGH (>=50): "; CountHigh

GOSUB PauseSub
GOTO ContinueLoop

' -----------------------------------------
SortTest:
CLS

DIM Temp(20)

FOR I = 1 TO 20
    Temp(I) = Numbers(I)
NEXT

FOR I = 1 TO 19
    FOR J = I + 1 TO 20
        IF Temp(J) < Temp(I) THEN
            T = Temp(I)
            Temp(I) = Temp(J)
            Temp(J) = T
        END IF
    NEXT
NEXT

PRINT "SORTED VALUES"

FOR I = 1 TO 20
    PRINT Temp(I)
NEXT

GOSUB PauseSub
GOTO ContinueLoop

' -----------------------------------------
MathTest:
CLS

PRINT "FACTORIALS"

FOR I = 1 TO 10
    PRINT I; "="; Factorial&(I)
NEXT

PRINT
PRINT "FIBONACCI"

FOR I = 1 TO 15
    PRINT Fib&(I);
NEXT

PRINT

GOSUB PauseSub
GOTO ContinueLoop

' -----------------------------------------
OnGotoTest:
CLS

FOR I = 1 TO 25

    X = INT(RND * 4) + 1

    ON X GOSUB Sub1, Sub2, Sub3, Sub4

NEXT

GOSUB PauseSub
GOTO ContinueLoop

Sub1:
PRINT "SUBROUTINE 1"
RETURN

Sub2:
PRINT "SUBROUTINE 2"
RETURN

Sub3:
PRINT "SUBROUTINE 3"
RETURN

Sub4:
PRINT "SUBROUTINE 4"
RETURN

' -----------------------------------------
GridTest:
CLS

FOR X = 1 TO 10
    FOR Y = 1 TO 10
        Grid(X, Y) = X * Y
    NEXT
NEXT

FOR X = 1 TO 10
    FOR Y = 1 TO 10
        PRINT USING "###"; Grid(X, Y);
    NEXT
    PRINT
NEXT

GOSUB PauseSub
GOTO ContinueLoop

' -----------------------------------------
ContinueLoop:
LOOP

' -----------------------------------------
PauseSub:
PRINT
PRINT "PRESS ENTER..."
LINE INPUT A$
RETURN

' -----------------------------------------
ExitProgram:
END

' =========================================
FUNCTION Factorial& (N%)
    Result& = 1

    FOR K = 1 TO N%
        Result& = Result& * K
    NEXT

    Factorial& = Result&
END FUNCTION

' =========================================
FUNCTION Fib& (N%)
    IF N% <= 2 THEN
        Fib& = 1
        EXIT FUNCTION
    END IF

    A& = 1
    B& = 1

    FOR K = 3 TO N%
        T& = A& + B&
        A& = B&
        B& = T&
    NEXT

    Fib& = B&
END FUNCTION


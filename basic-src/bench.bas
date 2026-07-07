REM  BENCH.BAS -- QBasic 1.1 interpreter benchmark for the Mega Demo.
REM  Times the operations the demo's hot loops actually use and reports
REM  each one's ops/sec plus the ops-per-60fps-frame budget it implies.
REM  Run via bench.sh (same DOSBox-X cycles as the demo).  Results are
REM  printed and also written to BENCH.TXT.

DEFINT A-Z

DECLARE SUB EmptySub ()
DECLARE SUB Rec (label$, total&, t0!)

DIM SHARED nm$(14), rate&(14), reps&(14), secs!(14)
DIM SHARED nRes

DIM arr(255)
DIM sinT(255)
DIM spr(300)
DIM o AS LONG
DIM a AS LONG

FOR i = 0 TO 255
    sinT(i) = INT(SIN(i * 6.28318 / 256) * 127)
    arr(i) = i
NEXT i

nRes = 0

SCREEN 13
PRINT "Benchmarking (approx 30s)..."
DEF SEG = &HA000

' --- 1. Empty FOR/NEXT: pure loop overhead ---
t0! = TIMER
FOR j = 1 TO 20
    FOR i = 1 TO 20000
    NEXT i
NEXT j
CALL Rec("EMPTY FOR/NEXT", 400000, t0!)

' --- 2. Integer add ---
a = 0
t0! = TIMER
FOR j = 1 TO 15
    FOR i = 1 TO 20000
        a = a + 1
    NEXT i
NEXT j
CALL Rec("INTEGER ADD", 300000, t0!)

' --- 3. Integer multiply + integer divide ---
t0! = TIMER
FOR j = 1 TO 10
    FOR i = 1 TO 20000
        b = (CLng(i) * 3) \ 2
    NEXT i
NEXT j
CALL Rec("INT MUL+IDIV", 200000, t0!)

' --- 4. Array element read+write ---
t0! = TIMER
FOR j = 1 TO 10
    FOR i = 1 TO 20000
        arr(100) = arr(100) XOR 1
    NEXT i
NEXT j
CALL Rec("ARRAY RD+WR", 200000, t0!)

' --- 5. Sine-LUT hot-path step: index wrap + lookup + scale ---
ang = 0
t0! = TIMER
FOR j = 1 TO 5
    FOR i = 1 TO 20000
        ang = (ang + 3) AND 255
        v = (sinT(ang) * 90) \ 128
    NEXT i
NEXT j
CALL Rec("LUT SINE STEP", 100000, t0!)

' --- 6. POKE to VGA framebuffer ---
t0! = TIMER
FOR j = 1 TO 5
    FOR i = 1 TO 20000
        POKE 32000, 5
    NEXT i
NEXT j
CALL Rec("POKE", 100000, t0!)

' --- 7. PEEK + POKE read-modify-write (what killed shadebobs v1) ---
t0! = TIMER
FOR j = 1 TO 4
    FOR i = 1 TO 20000
        POKE 32000, (PEEK(32000) + 1) AND 255
    NEXT i
NEXT j
CALL Rec("PEEK+POKE RMW", 80000, t0!)

' --- 8. PSET ---
t0! = TIMER
FOR j = 1 TO 3
    FOR i = 1 TO 20000
        PSET (160, 100), 5
    NEXT i
NEXT j
CALL Rec("PSET", 60000, t0!)

' --- 9. LINE, full 320px horizontal ---
t0! = TIMER
FOR i = 1 TO 20000
    LINE (0, 120)-(319, 120), 5
NEXT i
CALL Rec("LINE 320PX", 20000, t0!)

' --- 10. LINE BF 24x24 filled box ---
t0! = TIMER
FOR i = 1 TO 20000
    LINE (50, 130)-(73, 153), 5, BF
NEXT i
CALL Rec("LINE BF 24x24", 20000, t0!)

' --- 11. GET 24x24 sprite ---
t0! = TIMER
FOR i = 1 TO 20000
    GET (50, 130)-(73, 153), spr
NEXT i
CALL Rec("GET 24x24", 20000, t0!)

' --- 12. PUT 24x24 sprite (PSET action) ---
t0! = TIMER
FOR i = 1 TO 20000
    PUT (100, 130), spr, PSET
NEXT i
CALL Rec("PUT 24x24", 20000, t0!)

' --- 13. Empty SUB call ---
t0! = TIMER
FOR j = 1 TO 10
    FOR i = 1 TO 20000
        CALL EmptySub
    NEXT i
NEXT j
CALL Rec("EMPTY SUB CALL", 200000, t0!)

DEF SEG

' ------- Report -------
SCREEN 0
WIDTH 80
fmt$ = "\                \ ######### ####.## ########## #########"
hd1$ = "QBasic 1.1 interpreter benchmark -- DOSBox-X (cycles per bench.sh)"
hd2$ = "TEST                    ITERS SECONDS    OPS/SEC OPS/FRAME (60fps)"

PRINT hd1$
PRINT
PRINT hd2$
FOR i = 0 TO nRes - 1
    PRINT USING fmt$; nm$(i); reps&(i); secs!(i); rate&(i); rate&(i) \ 60
NEXT i

OPEN "BENCH.TXT" FOR OUTPUT AS #1
PRINT #1, hd1$
PRINT #1, ""
PRINT #1, hd2$
FOR i = 0 TO nRes - 1
    PRINT #1, USING fmt$; nm$(i); reps&(i); secs!(i); rate&(i); rate&(i) \ 60
NEXT i
CLOSE #1

PRINT
PRINT "Results written to BENCH.TXT -- press any key to exit."
DO
LOOP WHILE INKEY$ = ""
SYSTEM

' Records one result; assumes the test just ran with start time t0!
SUB Rec (label$, total&, t0!)
    DEFINT A-Z
    dt! = TIMER - t0!
    IF dt! < 0 THEN dt! = dt! + 86400   ' midnight wrap
    IF dt! < .01 THEN dt! = .01
    nm$(nRes) = label$
    reps&(nRes) = total&
    secs!(nRes) = dt!
    rate&(nRes) = total& / dt!
    nRes = nRes + 1
    PRINT "done:"; nRes
END SUB

SUB EmptySub
END SUB

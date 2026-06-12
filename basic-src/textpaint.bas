REM QBC TITLE textpaint demo
' Demonstrates:
'   1. QBC_TEXT_FB font rendering — SCREEN 0 (8x16 double-scan)
'      and SCREEN 9 (8x14 proportional)
'   2. PAINT CHR$() pattern tiling with various patterns and border colors

' ── Part 1: SCREEN 0 text quality demo ───────────────────────────────────────
SCREEN 0
WIDTH 80, 25
COLOR 15, 1         ' white on blue
CLS

LOCATE 1, 28: COLOR 14, 1: PRINT "QBC Text Font Demo"
LOCATE 2, 1:  COLOR 7, 1:  PRINT STRING$(80, CHR$(196))  ' horizontal line

LOCATE 4, 5:  COLOR 11, 1: PRINT "SCREEN 0 (80x25) -- 8x16 double-scan font"
LOCATE 6, 5:  COLOR 15, 1: PRINT "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
LOCATE 7, 5:  COLOR 15, 1: PRINT "0123456789 !@#$%^&*()-=_+[]{}|;':,./<>?"
LOCATE 9, 5:  COLOR 14, 1: PRINT "Box drawing: " + CHR$(218) + CHR$(196) + CHR$(194) + CHR$(196) + CHR$(191)
LOCATE 10, 5: COLOR 14, 1: PRINT "             " + CHR$(179) + " " + CHR$(179) + " " + CHR$(179)
LOCATE 11, 5: COLOR 14, 1: PRINT "             " + CHR$(195) + CHR$(196) + CHR$(197) + CHR$(196) + CHR$(180)
LOCATE 12, 5: COLOR 14, 1: PRINT "             " + CHR$(192) + CHR$(196) + CHR$(193) + CHR$(196) + CHR$(217)

LOCATE 14, 5: COLOR 10, 1: PRINT "Colors:";
FOR c = 0 TO 15
  COLOR c, 1
  PRINT " " + STR$(c);
NEXT c
COLOR 15, 1

LOCATE 16, 5: PRINT "Press any key to continue..."
SLEEP

' ── Part 2: SCREEN 9 PAINT CHR$() pattern demo ───────────────────────────────
SCREEN 9
CLS

' Title
COLOR 14, 0
LOCATE 1, 25: PRINT "PAINT CHR$() Pattern Tiling"

' --- Panel 1: CHR$(85) = 01010101 checkerboard, border=15 (white) ---
LINE (20,20)-(130,100), 15, B
COLOR 9, 0      ' bright blue fg
PAINT (75, 60), CHR$(85), 15
LOCATE 9, 3: COLOR 7, 0: PRINT "CHR$(85)=01010101"
LOCATE 10, 3: PRINT "border=white(15)"

' --- Panel 2: CHR$(170) = 10101010 (opposite phase), border=15 ---
LINE (150,20)-(260,100), 15, B
COLOR 12, 0     ' bright red fg
PAINT (205, 60), CHR$(170), 15
LOCATE 9, 21: COLOR 7, 0: PRINT "CHR$(170)=10101010"
LOCATE 10, 21: PRINT "border=white(15)"

' --- Panel 3: non-solid border color = green(2) ---
LINE (280,20)-(390,100), 2, B   ' green border
COLOR 13, 0    ' magenta fg
PAINT (335, 60), CHR$(85), 2
LOCATE 9, 39: COLOR 7, 0: PRINT "CHR$(85)"
LOCATE 10, 39: PRINT "border=green(2)"

' --- Panel 4: two-row pattern (solid/empty alternating rows) ---
LINE (410,20)-(520,100), 15, B
COLOR 11, 0    ' bright cyan fg
PAINT (465, 60), CHR$(255) + CHR$(0), 15
LOCATE 9, 57: COLOR 7, 0: PRINT "CHR$(255)+CHR$(0)"
LOCATE 10, 57: PRINT "row-alternating"

' --- Panel 5: diagonal stripe CHR$(0x88)=10001000 in colored box ---
LINE (540,20)-(630,100), 3, B   ' cyan border
COLOR 14, 0    ' yellow fg
PAINT (585, 60), CHR$(136), 3   ' 136 = 10001000
LOCATE 9, 72: COLOR 7, 0: PRINT "CHR$(136)"
LOCATE 10, 72: PRINT "border=cyan(3)"

' --- Panel 6: dense vs sparse patterns side by side ---
LINE (20,120)-(310,200), 15, B
LINE (165,120)-(165,200), 15   ' divide
' Left half: dense  CHR$(0xEE)=11101110
COLOR 10, 0
PAINT (92, 160), CHR$(238), 15    ' 238 = 11101110
' Right half: sparse CHR$(0x11)=00010001
COLOR 9, 0
PAINT (237, 160), CHR$(17), 15    ' 17  = 00010001
LOCATE 16, 3: COLOR 7, 0: PRINT "Dense: CHR$(238)       Sparse: CHR$(17)"

' --- Panel 7: PAINT stop-at-non-zero border demo ---
' Outer box green, inner box red, fill gap with pattern
LINE (330,120)-(620,200), 2, B    ' green outer
LINE (370,140)-(580,180), 4, B    ' red inner boundary
COLOR 15, 0
PAINT (350, 160), CHR$(85), 2     ' fills only between green outer and red inner
LOCATE 16, 46: COLOR 7, 0: PRINT "Pattern stops at red(4) border"

COLOR 7, 0
LOCATE 24, 22: PRINT "Press any key to exit"
SLEEP
SCREEN 0

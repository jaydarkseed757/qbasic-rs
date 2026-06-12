' Test PAINT CHR$() pattern tiling + POINT() readback
' No SCREEN call: graphics ops use default 640x400 fb, PRINT goes to stdout.

' Test 1: CHR$(255) = 11111111 all-ones -> solid fill via pattern path
' Interior starts at 0 (black); draw white border; fill with all-ones (fg=7)
COLOR 7, 0
LINE (10,10)-(50,50), 15, B
PAINT (30,30), CHR$(255), 15
' Interior fully painted with draw_color (7)
PRINT "T1:"; POINT(30,30)    ' expect 7

' Test 2: CHR$(85) = 01010101 alternating columns
' x=80: 80 mod 8 = 0, bit 7 of 0x55 = 0 -> NOT painted (stays 0)
' x=81: 81 mod 8 = 1, bit 6 of 0x55 = 1 -> painted (fg=7)
COLOR 7, 0
LINE (60,10)-(100,50), 15, B
PAINT (80,30), CHR$(85), 15
PRINT "T2 unpainted:"; POINT(80,30)   ' expect 0
PRINT "T2 painted:";   POINT(81,30)   ' expect 7

' Test 3: non-solid border color (border=2, green)
' Green LINE border; interior black; fill CHR$(170)=10101010, border=2
' BFS stops at green pixels; green border must survive intact
COLOR 14, 0
LINE (110,10)-(150,50), 2, B
PAINT (130,30), CHR$(170), 2
' x=130 mod 8 = 2, bit 5 of 0xAA = 1 -> painted yellow (14)
' x=131 mod 8 = 3, bit 4 of 0xAA = 0 -> stays black (0)
PRINT "T3 painted:";  POINT(130,30)   ' expect 14
PRINT "T3 skipped:";  POINT(131,30)   ' expect 0
PRINT "T3 border:";   POINT(110,30)   ' expect 2

' Test 4: two-row pattern CHR$(255)+CHR$(0) -> alternating solid/empty rows
' y=30: 30 mod 2 = 0 -> all-ones row -> painted
' y=31: 31 mod 2 = 1 -> all-zeros row -> stays 0
COLOR 9, 0
LINE (160,10)-(200,50), 15, B
PAINT (180,30), CHR$(255) + CHR$(0), 15
PRINT "T4 solid row:"; POINT(180,30)  ' expect 9
PRINT "T4 empty row:"; POINT(180,31)  ' expect 0

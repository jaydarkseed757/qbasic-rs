CLS
SCREEN 13

FOR C = 0 TO 255
    X = (C MOD 16) * 20
    Y = (C \ 16) * 12

    LINE (X, Y)-(X + 19, Y + 11), C, BF
NEXT C

LOCATE 25, 1
PRINT "256 Color Palette - Press any key..."
SLEEP



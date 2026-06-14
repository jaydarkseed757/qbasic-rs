' =========================================
' PRIDE FLAG - SCREEN 13 VERSION
' QBASIC / VGA 320x200x256
' =========================================

SCREEN 13
CLS

' Red stripe
LINE (0, 0)-(319, 32), 4, BF

' Orange stripe
LINE (0, 33)-(319, 65), 6, BF

' Yellow stripe
LINE (0, 66)-(319, 98), 14, BF

' Green stripe
LINE (0, 99)-(319, 131), 2, BF

' Blue stripe
LINE (0, 132)-(319, 164), 1, BF

' Purple stripe
LINE (0, 165)-(319, 199), 5, BF

' Title text
COLOR 15
LOCATE 2, 13
PRINT "PRIDE FLAG"

LOCATE 24, 8
PRINT "PRESS ANY KEY TO EXIT"

WHILE INKEY$ = ""
WEND

SCREEN 0
WIDTH 80
CLS
END



' =========================================
' PRIDE FLAG FOR QBASIC
' SCREEN 12 (640x480, 16 colors)
' =========================================

SCREEN 12
CLS

' Redefine palette slots 8-13 with the true pride-flag colors.
' PALETTE expects 6-bit components (0-63): value = R + G*256 + B*65536.
PALETTE 8, 57& + 0& * 256 + 0& * 65536          ' Red    (E40303)
PALETTE 9, 63& + 35& * 256 + 0& * 65536         ' Orange (FF8C00)
PALETTE 10, 63& + 59& * 256 + 0& * 65536        ' Yellow (FFED00)
PALETTE 11, 0& + 32& * 256 + 9& * 65536         ' Green  (008026)
PALETTE 12, 9& + 19& * 256 + 35& * 65536        ' Blue   (24408F)
PALETTE 13, 29& + 1& * 256 + 33& * 65536        ' Purple (750787)

' Draw six equal horizontal stripes (480 / 6 = 80 px each)
FOR i = 0 TO 5
    y = i * 80
    LINE (0, y)-(639, y + 79), 8 + i, BF
NEXT i

' Title — white reads cleanly over every stripe
COLOR 15
LOCATE 2, 36
PRINT "PRIDE"


LOCATE 30, 28
PRINT "Press any key to exit..."

DO
LOOP WHILE INKEY$ = ""

' Restore the default text mode before exiting
PALETTE
SCREEN 0
WIDTH 80
CLS
END

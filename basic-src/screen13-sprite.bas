' =========================================
' SCREEN 13 GET/PUT SPRITE DEMO
' Captures a 256-color sprite with GET and blits
' it around with PUT (PSET and XOR verbs).
' =========================================

SCREEN 13
CLS

' Build a recognizable 24x16 sprite using many of the 256 colors.
FOR y = 0 TO 15
    FOR x = 0 TO 23
        PSET (x + 20, y + 20), ((x * 11 + y * 23) MOD 254) + 1
    NEXT x
NEXT y

' Capture it. 24x16 = 384 bytes -> 2 header + 192 data = 194 INTEGERs.
DIM spr(200) AS INTEGER
GET (20, 20)-(43, 35), spr

' Blit copies across the screen with PSET (overwrite).
PUT (100, 50), spr, PSET
PUT (160, 80), spr, PSET
PUT (60, 120), spr, PSET

' XOR draw then XOR erase at the same spot: the second PUT must restore
' whatever was underneath (XOR is its own inverse).
PUT (220, 120), spr, XOR
PUT (220, 120), spr, XOR

LOCATE 24, 1: PRINT "SCREEN 13 sprite GET/PUT";

END

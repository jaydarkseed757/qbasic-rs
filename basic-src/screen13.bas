REM SCREEN 13 (MCGA 320x200, 256 colors) demo / verification
SCREEN 13

REM 1) Default-palette bands: indices 0..255 across the top.
REM    Indices >15 must show as DISTINCT colors, not wrapped mod 16.
FOR i = 0 TO 255
    LINE (i + 32, 10)-(i + 32, 40), i
NEXT i

REM 2) Custom PALETTE: remap a few high indices via the 18-bit DAC encoding
REM    color = red + 256*green + 65536*blue, each channel 0..63.
PALETTE 100, 63                 ' pure red
PALETTE 101, 63 * 256          ' pure green
PALETTE 102, 63 * 65536        ' pure blue
PALETTE 103, 63 + 63 * 65536   ' magenta

LINE (40, 70)-(110, 130), 100, BF
LINE (120, 70)-(190, 130), 101, BF
LINE (200, 70)-(270, 130), 102, BF
LINE (130, 150)-(190, 190), 103, BF

REM 3) PSET single pixels at high indices (collision-style read-back path)
FOR y = 150 TO 190
    PSET (50 + (y - 150), y), 200
NEXT y

LOCATE 24, 1
PRINT "SCREEN 13 - 256 colors";
END

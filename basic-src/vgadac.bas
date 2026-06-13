REM vgadac.bas — test OUT &H3C8/&H3C9 VGA DAC palette writes
REM Compares OUT-based palette setting against the PALETTE statement.
SCREEN 13

' ── Method 1: PALETTE statement ───────────────────────────────────────────────
' PALETTE idx, red + 256*green + 65536*blue  (each channel 0–63)
PALETTE 16, 63 + 256 * 0 + 65536 * 0    ' index 16 = bright red
PALETTE 17, 0 + 256 * 63 + 65536 * 0    ' index 17 = bright green
PALETTE 18, 0 + 256 * 0 + 65536 * 63    ' index 18 = bright blue

LINE (10, 10)-(100, 70), 16, BF
LINE (10, 80)-(100, 140), 17, BF
LINE (10, 150)-(100, 190), 18, BF

' ── Method 2: OUT &H3C8 / &H3C9 port writes ──────────────────────────────────
' OUT &H3C8, idx  — select palette entry
' OUT &H3C9, r    — red   0–63
' OUT &H3C9, g    — green 0–63
' OUT &H3C9, b    — blue  0–63  (index auto-advances after blue)

OUT &H3C8, 32        ' start at entry 32
OUT &H3C9, 63        ' R = max
OUT &H3C9, 0         ' G = 0
OUT &H3C9, 0         ' B = 0  → entry 32 = bright red; index advances to 33

OUT &H3C9, 0         ' R = 0  (now writing entry 33 — auto-advanced)
OUT &H3C9, 63        ' G = max
OUT &H3C9, 0         ' B = 0  → entry 33 = bright green; index advances to 34

OUT &H3C9, 0         ' R = 0  (now writing entry 34)
OUT &H3C9, 0         ' G = 0
OUT &H3C9, 63        ' B = max → entry 34 = bright blue

LINE (110, 10)-(200, 70), 32, BF
LINE (110, 80)-(200, 140), 33, BF
LINE (110, 150)-(200, 190), 34, BF

' ── Method 3: read back via INP(&H3C9) and verify ────────────────────────────
OUT &H3C7, 32        ' set DAC read pointer to entry 32
r32 = INP(&H3C9)     ' read R of entry 32
g32 = INP(&H3C9)     ' read G
b32 = INP(&H3C9)     ' read B  (pointer auto-advances to 33)

OUT &H3C8, 48        ' write a mixed color via OUT to entry 48
OUT &H3C9, 32        ' R = 32 (half)
OUT &H3C9, 16        ' G = 16 (quarter)
OUT &H3C9, 63        ' B = 63 (full) → magenta-ish

LINE (210, 10)-(310, 70), 48, BF

' ── Labels ───────────────────────────────────────────────────────────────────
LOCATE 24, 1
PRINT "Left=PALETTE  Mid=OUT ports  Right=mixed";
LOCATE 25, 1
PRINT "Entry 32: R="; r32; " G="; g32; " B="; b32; " (expect 63 0 0)";

SLEEP
END

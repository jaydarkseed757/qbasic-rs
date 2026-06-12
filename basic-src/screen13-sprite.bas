' =========================================
' SCREEN 13 GET/PUT SPRITE DEMO
' Exercises 256-color GET/PUT:
'   - round-trip fidelity (colors 0-255)
'   - odd-width sprite (7px wide)
'   - all PUT verbs: PSET, XOR, AND, OR, PRESET
'   - clipping at all four screen edges
' =========================================

SCREEN 13
CLS

' ── Sprite A: 24x16, color sweep 1..254 (even-width, many colors) ─────────────
FOR y = 0 TO 15
    FOR x = 0 TO 23
        PSET (x + 2, y + 2), ((x * 11 + y * 23) MOD 254) + 1
    NEXT x
NEXT y
DIM sprA(200) AS INTEGER
GET (2, 2)-(25, 17), sprA
CLS

' ── Sprite B: 7x5, colors 0/127/128/200/255 (odd-width, sign-edge colors) ──────
PSET (2, 2), 0
PSET (3, 2), 127
PSET (4, 2), 128
PSET (5, 2), 200
PSET (6, 2), 255
PSET (2, 3), 254
PSET (3, 3), 1
PSET (4, 3), 129
PSET (5, 3), 201
PSET (6, 3), 253
PSET (2, 4), 2
PSET (3, 4), 126
PSET (4, 4), 130
PSET (5, 4), 202
PSET (6, 4), 252
PSET (2, 5), 3
PSET (3, 5), 125
PSET (4, 5), 131
PSET (5, 5), 203
PSET (6, 5), 251
PSET (2, 6), 4
PSET (3, 6), 124
PSET (4, 6), 132
PSET (5, 6), 204
PSET (6, 6), 250
DIM sprB(30) AS INTEGER
GET (2, 2)-(8, 6), sprB
CLS

' ── Lay down a solid background stripe so AND/OR/XOR effects are visible ───────
FOR y = 0 TO 199
    FOR x = 0 TO 31
        PSET (x + 200, y), (x * 8 + y) MOD 256
    NEXT x
NEXT y

' ── PSET: simple overwrite — sprA at (10,10) and (100,80) ─────────────────────
PUT (10, 10), sprA, PSET
PUT (100, 80), sprA, PSET

' ── XOR self-inverse: two PUTs at same spot restore background ────────────────
FOR y = 50 TO 65
    FOR x = 50 TO 73
        PSET (x, y), (x * 3 + y * 7) MOD 256
    NEXT x
NEXT y
PUT (50, 50), sprA, XOR
PUT (50, 50), sprA, XOR   ' background fully restored

' ── PRESET: complement each pixel against background ─────────────────────────
PUT (10, 100), sprA, PRESET

' ── AND: mask into textured background ───────────────────────────────────────
PUT (200, 10), sprA, AND

' ── OR: overlay onto textured background ─────────────────────────────────────
PUT (200, 90), sprA, OR

' ── Odd-width sprite B at several positions ───────────────────────────────────
PUT (150, 10), sprB, PSET
PUT (150, 20), sprB, PSET
PUT (170, 10), sprB, XOR
PUT (170, 10), sprB, XOR   ' XOR self-inverse on odd-width

' ── Clipping: partially off each edge ────────────────────────────────────────
PUT (-4,  10), sprA, PSET   ' clip left
PUT (302,  10), sprA, PSET  ' clip right  (320 - 24 + 6 = off by 6px)
PUT ( 10,  -4), sprA, PSET  ' clip top
PUT ( 10, 188), sprA, PSET  ' clip bottom (200 - 16 + 4 = off by 4px)

END

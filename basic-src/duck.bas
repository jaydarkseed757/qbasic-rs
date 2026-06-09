' DUCK.BAS
' A cartoon duck drawn with DRAW and PAINT
' EGA SCREEN 9 (640 x 350, 16 colours)
'
' Colour reference (EGA):
'   0=Black  1=DkBlue  2=DkGreen  3=DkCyan
'   6=Brown  9=Blue   10=Green   11=Cyan
'  14=Yellow 15=White
'
' Duck faces right.  Draw order (back to front):
'   sky/water → sun → tail → body → wing → head → beak → eye

SCREEN 9
CLS

' ── Background ──────────────────────────────────────────
LINE (0, 0)-(639, 229), 9, BF       ' sky (blue)
LINE (0, 230)-(639, 349), 1, BF     ' water (dark blue)

' Water shimmer lines
FOR wy = 248 TO 315 STEP 22
    LINE (0, wy)-(639, wy), 11
    LINE (0, wy + 5)-(639, wy + 5), 3
NEXT wy

' Sun (top-right)
CIRCLE (570, 50), 28, 14
PAINT (570, 50), 14, 14

' ── Tail feathers ───────────────────────────────────────
' Closed polygon: spike pointing up-left from the body's back.
' Vertices approx: base (115,220)→tip (115,165)→return.
' Drawn before the body so the body covers the root.
'
' Path trace:
'  (115,220) H15→(100,205) U25→(100,180) E15→(115,165)
'  F25→(140,190) R25→(165,190) D10→(165,200) L50→(115,200)
'  D20→(115,220) ✓

PSET (115, 220), 0
DRAW "C0 BM115,220"
DRAW "H15 U25 E15 F25 R25 D10 L50 D20"
PAINT (125, 188), 6, 0              ' fill brown

' ── Duck body ───────────────────────────────────────────
' Chamfered rectangle (170,185)→(375,270), corner r=15.
'
' Path trace:
'  (170,185) R205→(375,185) F15→(390,200) D55→(390,255)
'  G15→(375,270) L205→(170,270) H15→(155,255) U55→(155,200)
'  E15→(170,185) ✓

PSET (170, 185), 0
DRAW "C0 BM170,185"
DRAW "R205 F15 D55 G15 L205 H15 U55 E15"
PAINT (272, 227), 14, 0             ' fill yellow

' ── Wing patch ──────────────────────────────────────────
' White chamfered rectangle on the body surface.
'
' Path trace:
'  (215,208) R95→(310,208) F10→(320,218) D32→(320,250)
'  G10→(310,260) L95→(215,260) H10→(205,250) U32→(205,218)
'  E10→(215,208) ✓

PSET (215, 208), 0
DRAW "C0 BM215,208"
DRAW "R95 F10 D32 G10 L95 H10 U32 E10"
PAINT (262, 232), 15, 0             ' fill white

' ── Head ────────────────────────────────────────────────
' Chamfered rectangle (345,118)→(457,205), corner r=12.
'
' Path trace:
'  (345,118) R100→(445,118) F12→(457,130) D63→(457,193)
'  G12→(445,205) L100→(345,205) H12→(333,193) U63→(333,130)
'  E12→(345,118) ✓

PSET (345, 118), 0
DRAW "C0 BM345,118"
DRAW "R100 F12 D63 G12 L100 H12 U63 E12"
PAINT (395, 160), 14, 0             ' fill yellow
' Cover the body-top edge that cuts through the neck junction
LINE (346, 185)-(374, 185), 14

' ── Beak ────────────────────────────────────────────────
' Flat rectangular duck bill extending right from the head.
'
' Path trace:
'  (457,148) R38→(495,148) D28→(495,176) L38→(457,176)
'  U28→(457,148) ✓

PSET (457, 148), 0
DRAW "C0 BM457,148"
DRAW "R38 D28 L38 U28"
PAINT (476, 162), 6, 0              ' fill brown-orange

' Beak centre crease and nostril
LINE (457, 162)-(495, 162), 0
PSET (477, 153), 0

' ── Eye ─────────────────────────────────────────────────
' 3×3 black pupil with a single white highlight pixel
PSET (393, 143), 0
PSET (394, 143), 0
PSET (395, 143), 0
PSET (393, 144), 0
PSET (394, 144), 0
PSET (395, 144), 0
PSET (393, 145), 0
PSET (394, 145), 0
PSET (395, 145), 0
PSET (394, 143), 15                 ' highlight

' ── Water reflection under duck ─────────────────────────
LINE (140, 274)-(470, 274), 11
LINE (170, 279)-(445, 279), 3
LINE (195, 284)-(420, 284), 11

' ── Prompt ──────────────────────────────────────────────
LOCATE 24, 1
COLOR 15, 0
PRINT "Press any key...";
WHILE INKEY$ = "": WEND

END

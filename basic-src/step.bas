REM STEP relative graphics coordinates demo (EGA SCREEN 9)
REM Shows: PSET STEP, LINE STEP chains, CIRCLE STEP, GET/PUT STEP.
REM The same LINE-STEP chain drawn from two different origins proves the
REM shape is built entirely from cursor-relative moves.
SCREEN 9
CLS

REM --- Box 1: outline built from a STEP chain starting at an absolute point.
PSET (100, 100), 15           ' set the graphics cursor (last point referenced)
LINE STEP(0, 0)-STEP(60, 0), 14   ' top edge: 2nd point relative to the 1st
LINE -STEP(0, 40), 14             ' right edge: from cursor, down 40
LINE -STEP(-60, 0), 14            ' bottom edge
LINE -STEP(0, -40), 14            ' left edge, closes the box

REM --- Box 2: identical chain, only the origin changed.
PSET (260, 100), 15
LINE STEP(0, 0)-STEP(60, 0), 12
LINE -STEP(0, 40), 12
LINE -STEP(-60, 0), 12
LINE -STEP(0, -40), 12

REM --- CIRCLE STEP: center relative to the last point (box-2 top-left corner).
CIRCLE STEP(30, 20), 18, 13

REM --- GET a region, then PUT it twice. PUT does NOT move the cursor, so the
REM relative PUT is relative to the last PSET/LINE/CIRCLE — set it explicitly.
DIM Spr(1 TO 100)
GET (100, 100)-(160, 140), Spr
PSET (100, 220), 0                  ' set the cursor (LPR) to (100,220)
PUT (100, 220), Spr, PSET          ' absolute copy
PUT STEP(120, 0), Spr, PSET        ' relative to the cursor -> (220,220)

LOCATE 25, 1
PRINT "STEP demo: two boxes (same chain, diff origin), circle, sprite copies";
END

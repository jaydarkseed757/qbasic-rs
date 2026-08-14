' ORBITS.BAS -- deterministic orbital-mechanics demo (SCREEN 13).
'
' Exercises genuine TYPE-in-TYPE nesting: Vec2 is nested INSIDE Body (not just
' a flat field list like every other bundled program's TYPE), and Bodies() is
' an ARRAY of that nested TYPE, accessed through two-level dotted field
' chains on array elements (Bodies(i).Pos.X). The transpiler's
' flatten_type_fields (emitter/scan.rs) has recursed through nested UserType
' fields since early on, but no bundled program had ever actually forced
' that recursion to run -- this is that first real exercise.
'
' Physics: each planet orbits a fixed, dominant central sun under real
' Newtonian gravity (a = G*Msun / r^2, toward the sun). Initial velocity for
' each planet is set to the exact circular-orbit speed (v = SQR(G*Msun/r)),
' so every orbit is a stable circle by construction -- no drift-chasing
' needed for a clean, repeatable animation. Sun-planet gravity only (no
' planet-planet interaction) keeps a plain fixed-timestep Euler step
' trivially stable. Fully deterministic (no RND) -- a fixed step count runs
' each planet through more than one full lap and then the program ends on
' its own, so this can run headless start-to-finish with no key input.

DECLARE SUB DrawFilledCircle (cx AS SINGLE, cy AS SINGLE, rad AS INTEGER, col AS INTEGER)

TYPE Vec2
    X AS SINGLE
    Y AS SINGLE
END TYPE

TYPE Body
    Pos AS Vec2
    Vel AS Vec2
    Mass AS SINGLE
    Col AS INTEGER
    Rad AS INTEGER
END TYPE

CONST NPLANETS = 3
CONST G = 800
CONST CX = 160
CONST CY = 100

DIM Bodies(NPLANETS) AS Body   ' Bodies(0) = sun, Bodies(1..3) = planets
DIM ORBR(NPLANETS) AS SINGLE   ' orbital radius per planet
DIM PCOL(NPLANETS) AS INTEGER  ' color per planet
DIM PDIR(NPLANETS) AS SINGLE   ' orbital direction, +1 or -1 (retrograde)
DIM i AS INTEGER
DIM frm AS INTEGER
DIM dt AS SINGLE
DIM dx AS SINGLE, dy AS SINGLE, r2 AS SINGLE, r AS SINGLE, acc AS SINGLE

SCREEN 13
COLOR 15

' -- Sun: fixed at screen center, dominant mass --
Bodies(0).Pos.X = CX
Bodies(0).Pos.Y = CY
Bodies(0).Mass = 4000
Bodies(0).Col = 14
Bodies(0).Rad = 5

' -- Planets: inner/fast blue, mid retrograde green, outer/slow red --
ORBR(1) = 24: PCOL(1) = 9: PDIR(1) = 1
ORBR(2) = 45: PCOL(2) = 10: PDIR(2) = -1
ORBR(3) = 75: PCOL(3) = 12: PDIR(3) = 1

FOR i = 1 TO NPLANETS
    Bodies(i).Pos.X = CX + ORBR(i)
    Bodies(i).Pos.Y = CY
    Bodies(i).Vel.X = 0
    Bodies(i).Vel.Y = PDIR(i) * SQR(G * Bodies(0).Mass / ORBR(i))
    Bodies(i).Col = PCOL(i)
    Bodies(i).Rad = 2
NEXT i

CALL DrawFilledCircle(Bodies(0).Pos.X, Bodies(0).Pos.Y, Bodies(0).Rad, Bodies(0).Col)

' -- Simulate: fixed-timestep Euler, one substep per vsync frame --
dt = .006

FOR frm = 1 TO 900
    WAIT &H3DA, 8, 8
    WAIT &H3DA, 8
    FOR i = 1 TO NPLANETS
        dx = Bodies(0).Pos.X - Bodies(i).Pos.X
        dy = Bodies(0).Pos.Y - Bodies(i).Pos.Y
        r2 = dx * dx + dy * dy
        r = SQR(r2)
        acc = G * Bodies(0).Mass / r2
        Bodies(i).Vel.X = Bodies(i).Vel.X + acc * (dx / r) * dt
        Bodies(i).Vel.Y = Bodies(i).Vel.Y + acc * (dy / r) * dt
        Bodies(i).Pos.X = Bodies(i).Pos.X + Bodies(i).Vel.X * dt
        Bodies(i).Pos.Y = Bodies(i).Pos.Y + Bodies(i).Vel.Y * dt
        PSET (Bodies(i).Pos.X, Bodies(i).Pos.Y), Bodies(i).Col
    NEXT i
NEXT frm

' Final positions as small filled discs, drawn on top of the trails.
FOR i = 1 TO NPLANETS
    CALL DrawFilledCircle(Bodies(i).Pos.X, Bodies(i).Pos.Y, Bodies(i).Rad, Bodies(i).Col)
NEXT i

END

SUB DrawFilledCircle (cx AS SINGLE, cy AS SINGLE, rad AS INTEGER, col AS INTEGER)
    CIRCLE (cx, cy), rad, col
    PAINT (cx, cy), col, col
END SUB

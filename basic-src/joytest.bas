' JOYTEST.BAS - joystick (STICK/STRIG) demonstration
'
' Polls the stick each frame and draws its position as a marker inside a
' calibration box, plus a lamp per button. Because qbc drives the stick from
' the KEYBOARD (arrow keys for the axes, SPACE and ENTER for buttons A1/A2),
' this is playable with no joystick attached; run it and move the arrows.
'
' STICK(0) samples the hardware and latches all four axes, then STICK(1)
' returns the y that same sample took - reading each axis independently
' could pair an x and y from different instants. STRIG has two flavours per
' button: the even numbers ask "pressed since I last asked" (a self-clearing
' edge latch) and the odd ones ask "held down right now".
'
' Set QBC_JOYSTICK=off to get the no-stick-attached behaviour instead:
' centred axes and no buttons, which is what a real DOS box reports with
' nothing plugged into the game port.

DEFINT A-Z

CONST BOXL = 100
CONST BOXT = 28
CONST BOXW = 120
CONST BOXH = 100
CONST FRAMES = 240

SCREEN 13

' --- static chrome ---------------------------------------------------
CLS
COLOR 15
LOCATE 1, 11: PRINT "STICK / STRIG DEMO";

' calibration box + centre cross
LINE (BOXL, BOXT)-(BOXL + BOXW, BOXT + BOXH), 8, B
LINE (BOXL + BOXW \ 2, BOXT)-(BOXL + BOXW \ 2, BOXT + BOXH), 1
LINE (BOXL, BOXT + BOXH \ 2)-(BOXL + BOXW, BOXT + BOXH \ 2), 1

COLOR 7
LOCATE 18, 7: PRINT "A1";
LOCATE 18, 30: PRINT "A2";
LOCATE 23, 4: PRINT "ARROWS=stick  SPACE/ENTER=fire";

px = -1
py = -1

FOR frame = 1 TO FRAMES
    ' STICK(0) latches; STICK(1) reads the y from that same sample.
    jx = STICK(0)
    jy = STICK(1)

    ' Map the 0-255 axis range onto the box.
    mx = BOXL + (jx * BOXW) \ 255
    my = BOXT + (jy * BOXH) \ 255

    IF mx <> px OR my <> py THEN
        IF px >= 0 THEN CIRCLE (px, py), 4, 0
        CIRCLE (mx, my), 4, 14
        px = mx
        py = my
    END IF

    ' Button lamps: odd STRIG = held right now.
    IF STRIG(1) THEN c1 = 10 ELSE c1 = 2
    IF STRIG(5) THEN c2 = 10 ELSE c2 = 2
    LINE (48, 146)-(64, 156), c1, BF
    LINE (232, 146)-(248, 156), c2, BF

    ' Even STRIG = "pressed since last asked", and reading it clears the
    ' latch - so a tap is caught even if the frame that polls misses the
    ' actual hold.
    IF STRIG(0) THEN hits = hits + 1

    COLOR 15
    LOCATE 21, 13: PRINT "A1 taps:"; hits;

    WAIT &H3DA, 8
NEXT frame

END

REM  Mega Demo -- QBasic 1.1 / SCREEN 13 (320x200x256)

DEFINT A-Z

CALL Scene1
CALL Scene2
CALL Scene3
CALL Scene4
' CALL Scene5   ' shadebobs -- too slow in interpreter; revisit with CALL ABSOLUTE
CALL Scene6
CALL Scene7
CALL Scene9    ' vector morph (numbered 9: Scene8 is reserved for the finale)
CALL Scene10   ' starship flight
CALL Scene8    ' credits crawl -- ALWAYS the final scene; add new scenes above
END

' ======================================================
' SCENE 1 -- Scrolling starfield
' ======================================================
SUB Scene1
    DEFINT A-Z
    DIM sx(100), sy(100), sz(100), sc(100)
    DIM oadr(100) AS LONG          ' framebuffer offset per star (= erase position)

    SCREEN 13
    DEF SEG = &HA000               ' point POKEs at the VGA framebuffer

    OUT &H3C8, 1: OUT &H3C9, 20: OUT &H3C9, 20: OUT &H3C9, 20
    OUT &H3C8, 2: OUT &H3C9, 40: OUT &H3C9, 40: OUT &H3C9, 40
    OUT &H3C8, 3: OUT &H3C9, 63: OUT &H3C9, 63: OUT &H3C9, 63

    RANDOMIZE TIMER
    FOR i = 1 TO 100
        sx(i) = INT(RND * 320)
        sy(i) = INT(RND * 200)
        sz(i) = INT(RND * 3) + 1
        sc(i) = sz(i)
        oadr(i) = CLng(sy(i)) * 320 + sx(i)
        POKE oadr(i), sc(i)
    NEXT i

    ' Erase/draw via direct framebuffer POKE -- no PSET overhead per pixel.
    ' Address walks by sz each frame (same row); recomputed only on wrap.
    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        FOR i = 1 TO 100
            POKE oadr(i), 0
            sx(i) = sx(i) + sz(i)
            IF sx(i) >= 320 THEN
                sx(i) = 0
                sy(i) = INT(RND * 200)
                oadr(i) = CLng(sy(i)) * 320
            ELSE
                oadr(i) = oadr(i) + sz(i)
            END IF
            POKE oadr(i), sc(i)
        NEXT i
    LOOP WHILE INKEY$ = ""

    ' Fade to black before next scene
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        pv = v * 20 \ 63: OUT &H3C8, 1: OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        pv = v * 40 \ 63: OUT &H3C8, 2: OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        OUT &H3C8, 3: OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v
    NEXT v

    DEF SEG                        ' restore default segment (was &HA000)
END SUB

' ======================================================
' SCENE 2 -- Title card with starfield backdrop, fade-in
' ======================================================
SUB Scene2
    DEFINT A-Z
    DIM rR(6), rG(6), rB(6)  ' 7 rainbow target colours (palette entries 6-12)
    DIM twPh(5), twSp(5)     ' 6 twinkle stars: phase + speed (palette entries 13-18)
    DIM sinT(255)            ' brightness LUT 5..63 for smooth twinkle

    SCREEN 13

    ' Twinkle brightness table: smooth sine ramp, never fully dark
    FOR i = 0 TO 255
        sinT(i) = INT((SIN(i * 6.28318 / 256) * .5 + .5) * 58) + 5
    NEXT i

    ' Star colours
    OUT &H3C8, 1: OUT &H3C9, 15: OUT &H3C9, 15: OUT &H3C9, 15
    OUT &H3C8, 2: OUT &H3C9, 35: OUT &H3C9, 35: OUT &H3C9, 35
    OUT &H3C8, 3: OUT &H3C9, 55: OUT &H3C9, 55: OUT &H3C9, 55

    ' Rainbow target colours (entries 6-12, all start black for fade-in)
    rR(0) = 63: rG(0) =  0: rB(0) =  0  ' red
    rR(1) = 63: rG(1) = 30: rB(1) =  0  ' orange
    rR(2) = 63: rG(2) = 63: rB(2) =  0  ' yellow
    rR(3) =  0: rG(3) = 50: rB(3) =  0  ' green
    rR(4) =  0: rG(4) = 50: rB(4) = 50  ' cyan
    rR(5) =  0: rG(5) =  0: rB(5) = 63  ' blue
    rR(6) = 35: rG(6) =  0: rB(6) = 63  ' violet
    FOR c = 0 TO 6
        OUT &H3C8, c + 6: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    NEXT c

    ' Static background stars -- fixed seed = same layout every run
    RANDOMIZE 42
    FOR i = 1 TO 80
        PSET (INT(RND * 320), INT(RND * 200)), INT(RND * 3) + 1
    NEXT i

    ' Draw each character in its own cycling rainbow colour (invisible until fade)
    ' "JAY'S QBASIC" : 12 chars x 24px = 288px wide, centred x=16
    FOR i = 1 TO 12
        CALL DrawText(MID$("JAY'S QBASIC", i, 1), 16 + (i - 1) * 24, 70, 3, (i - 1) MOD 7 + 6)
    NEXT i
    ' "MEGA DEMO" : 9 chars x 24px = 216px wide, centred x=52
    FOR i = 1 TO 9
        CALL DrawText(MID$("MEGA DEMO", i, 1), 52 + (i - 1) * 24, 106, 3, (i - 1) MOD 7 + 6)
    NEXT i

    ' 6 twinkle stars: each on its own palette entry (13-18) with an independent
    ' phase/speed so they sparkle out of sync. Drawn on top; animated via palette
    ' only (zero pixel writes per frame). Start mid-grey so they show during fade.
    FOR k = 0 TO 5
        ' Reroll positions inside the title text bands (y=70..129) so a
        ' twinkle star never lands on a letter pixel
        DO
            twX = INT(RND * 320)
            twY = INT(RND * 200)
        LOOP WHILE twY >= 70 AND twY <= 129
        PSET (twX, twY), 13 + k
        twPh(k) = INT(RND * 256)
        twSp(k) = INT(RND * 4) + 2
        OUT &H3C8, 13 + k: OUT &H3C9, 40: OUT &H3C9, 40: OUT &H3C9, 40
    NEXT k

    ' Fade in all 7 rainbow colours simultaneously
    FOR v = 0 TO 63
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        FOR c = 0 TO 6
            OUT &H3C8, c + 6
            OUT &H3C9, rR(c) * v \ 63
            OUT &H3C9, rG(c) * v \ 63
            OUT &H3C9, rB(c) * v \ 63
        NEXT c
    NEXT v

    ' Hold on the title, twinkling the 6 stars until a key is pressed
    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        FOR k = 0 TO 5
            v = sinT(twPh(k))
            OUT &H3C8, 13 + k: OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v
            twPh(k) = (twPh(k) + twSp(k)) AND 255
        NEXT k
    LOOP WHILE INKEY$ = ""

    SCREEN 0
END SUB

' ======================================================
' SCENE 3 -- Rotating 3D wireframe cube
' ======================================================
SUB Scene3
    DEFINT A-Z
    DIM sinT(255)               ' sine LUT: SIN(i*2pi/256) * 128
    DIM vx(7), vy(7), vz(7)    ' cube vertices (original, never modified)
    DIM px(7), py(7)            ' projected coords; also holds last frame for erase
    DIM e1(11), e2(11)          ' edge: vertex index pairs
    DIM tx AS LONG              ' rotation intermediates (LONG to avoid overflow)
    DIM ty AS LONG
    DIM tz AS LONG
    DIM xL AS LONG, yL AS LONG, zL AS LONG   ' vertex coords hoisted to LONG once

    SCREEN 13

    ' Cube colour starts black -- will fade in to cyan (0, 63, 63)
    OUT &H3C8, 4: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0

    ' Precompute 256-step sine table, scaled *128
    ' cos(a) = sinT((a+64) AND 255)  [quarter-period offset]
    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 128 + .5)
    NEXT i

    ' 8 vertices of the cube at +-80 on each axis
    vx(0) = -80: vy(0) = -80: vz(0) = -80
    vx(1) =  80: vy(1) = -80: vz(1) = -80
    vx(2) =  80: vy(2) =  80: vz(2) = -80
    vx(3) = -80: vy(3) =  80: vz(3) = -80
    vx(4) = -80: vy(4) = -80: vz(4) =  80
    vx(5) =  80: vy(5) = -80: vz(5) =  80
    vx(6) =  80: vy(6) =  80: vz(6) =  80
    vx(7) = -80: vy(7) =  80: vz(7) =  80

    ' 12 edges: front face, back face, 4 connecting pillars
    e1(0) = 0: e2(0) = 1:  e1(1) = 1: e2(1) = 2   ' front face
    e1(2) = 2: e2(2) = 3:  e1(3) = 3: e2(3) = 0
    e1(4) = 4: e2(4) = 5:  e1(5) = 5: e2(5) = 6   ' back face
    e1(6) = 6: e2(6) = 7:  e1(7) = 7: e2(7) = 4
    e1(8) = 0: e2(8) = 4:  e1(9) = 1: e2(9) = 5   ' pillars
    e1(10) = 2: e2(10) = 6: e1(11) = 3: e2(11) = 7

    ' Initialise projected coords to screen centre (safe first erase)
    FOR i = 0 TO 7
        px(i) = 160: py(i) = 100
    NEXT i

    angY = 0: angX = 0
    fadeV = 0

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8

        ' Fade cube colour in over first 63 frames: black -> cyan (0,63,63)
        IF fadeV < 63 THEN
            fadeV = fadeV + 1
            OUT &H3C8, 4
            OUT &H3C9, 0           ' R stays 0
            OUT &H3C9, fadeV       ' G 0->63
            OUT &H3C9, fadeV       ' B 0->63
        END IF

        ' Erase last frame's edges (px/py still hold previous frame's positions)
        FOR e = 0 TO 11
            a = e1(e): b = e2(e)
            LINE (px(a), py(a))-(px(b), py(b)), 0
        NEXT e

        ' Fetch sin/cos for both rotation angles this frame
        sinY = sinT(angY)
        cosY = sinT((angY + 64) AND 255)
        sinX = sinT(angX)
        cosX = sinT((angX + 64) AND 255)

        ' Rotate each vertex and project to screen
        FOR i = 0 TO 7
            xL = vx(i): zL = vz(i)              ' convert once, not per term
            ' Y-axis rotation (spin left-right)
            tx = (xL * cosY - zL * sinY) \ 128
            tz = (xL * sinY + zL * cosY) \ 128

            ' X-axis rotation (tumble up-down); uses tz from Y step above
            yL = vy(i)
            ty = (yL * cosX - tz * sinX) \ 128
            tz = (yL * sinX + tz * cosX) \ 128

            ' Perspective projection: focal=200, cube centre at z=300
            ' Reuse LONG tx/ty/tz directly -- no rx/ry/rz round-trip, no CInt/CLng
            denom = tz + 300
            IF denom < 1 THEN denom = 1
            px(i) = 160 + (tx * 200 \ denom)
            py(i) = 100 + (ty * 200 \ denom)
        NEXT i

        ' Draw new edges
        FOR e = 0 TO 11
            a = e1(e): b = e2(e)
            LINE (px(a), py(a))-(px(b), py(b)), 4
        NEXT e

        ' Advance angles: Y rotates twice as fast as X for nice tumble
        angY = (angY + 2) AND 255
        angX = (angX + 1) AND 255

    LOOP WHILE INKEY$ = ""

    ' Fade out
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 4: OUT &H3C9, 0: OUT &H3C9, v: OUT &H3C9, v
    NEXT v

    SCREEN 0
END SUB

' ======================================================
' SCENE 4 -- Plasma
' Pattern written once to VGA framebuffer; animation is palette cycling only.
' ======================================================
SUB Scene4
    DEFINT A-Z
    DIM sinT(255)
    DIM sx(319)                    ' x-only sine component, precomputed per column
    DIM diag(575)                  ' diagonal sine: sinT((x+y) AND 255); x+y max 518
    DIM pr(255), pg(255), pb(255)  ' rainbow palette channels
    DIM addr AS LONG               ' framebuffer offset: 199*320=63680 > INTEGER max
    DIM barBase(7) AS LONG         ' progress-bar row start offsets (hoisted)

    SCREEN 13

    ' Pattern cache: a valid file is exactly 7-byte BSAVE header + 64000
    ' pixel bytes.  OPEN FOR BINARY never errors on a missing file (it
    ' creates an empty one), so LOF doubles as the existence check.
    cached = 0
    OPEN "PLASMA.DAT" FOR BINARY AS #1
    IF LOF(1) = 64007 THEN cached = 1
    CLOSE #1

    ' Rainbow palette: R, G, B are sine waves 120 degrees apart
    ' Values stay in 1-63 (6-bit VGA DAC range)
    FOR i = 0 TO 255
        pr(i) = INT(SIN(i * 6.28318 / 256) * 31 + 32)
        pg(i) = INT(SIN(i * 6.28318 / 256 + 2.09440) * 31 + 32)
        pb(i) = INT(SIN(i * 6.28318 / 256 + 4.18879) * 31 + 32)
    NEXT i

    ' Start with palette blacked out so init is invisible
    OUT &H3C8, 0
    FOR i = 0 TO 255
        OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    NEXT i

    IF cached THEN
        ' Later runs: pull the whole pattern straight into video memory
        ' in one machine-speed statement
        DEF SEG = &HA000
        BLOAD "PLASMA.DAT", 0
        DEF SEG
    ELSE
        ' First run: compute the pattern (progress bar shown), cache it

        ' Unsigned sine table 0-255, plus per-column and diagonal LUTs
        ' (only needed to generate the pattern)
        FOR i = 0 TO 255
            sinT(i) = INT(SIN(i * 6.28318 / 256) * 127 + 128)
        NEXT i
        FOR x = 0 TO 319
            sx(x) = sinT((x * 2) AND 255)
        NEXT x
        FOR k = 0 TO 575
            diag(k) = sinT(k AND 255)
        NEXT k

        ' Progress bar colours
        OUT &H3C8, 253: OUT &H3C9, 63: OUT &H3C9, 63: OUT &H3C9, 63  ' fill: white
        OUT &H3C8, 254: OUT &H3C9, 15: OUT &H3C9, 15: OUT &H3C9, 15  ' track: dark grey

        ' Precompute progress-bar row start offsets (rows y=180..187) -- avoids a
        ' LONG multiply per bar POKE (1600 of them) during init
        FOR barRow = 0 TO 7
            barBase(barRow) = CLng(180 + barRow) * 320
        NEXT barRow

        ' Write plasma map directly to VGA framebuffer (A000h)
        ' Each byte is a palette index; no animation pixel writes ever needed
        DEF SEG = &HA000

        ' Draw empty progress bar track (200px wide, 8px tall, y=180, x=60..259)
        FOR barRow = 0 TO 7
            FOR barX = 60 TO 259
                POKE barBase(barRow) + barX, 254
            NEXT barX
        NEXT barRow

        FOR y = 0 TO 199
            ry = sinT((y * 3) AND 255)
            addr = CLng(y) * 320        ' row start; walked by +1 per pixel below
            FOR x = 0 TO 319
                POKE addr, (sx(x) + ry + diag(x + y)) AND 255
                addr = addr + 1
            NEXT x
            ' Rows 180-187 are the bar itself: plasma just overwrote this bar row,
            ' so rebuild it (fill up to current progress, track for the rest)
            IF y >= 180 AND y <= 187 THEN
                FOR barX = 60 TO 60 + y
                    POKE barBase(y - 180) + barX, 253
                NEXT barX
                FOR barX = 61 + y TO 259
                    POKE barBase(y - 180) + barX, 254
                NEXT barX
            END IF
            ' Advance bar: one pixel column per row computed (y 0..199 -> x 60..259)
            FOR barRow = 0 TO 7
                POKE barBase(barRow) + 60 + y, 253
            NEXT barRow
        NEXT y

        ' Repair pass: replace the finished bar with true plasma values so
        ' the cached image (and the screen) is pristine
        FOR y = 180 TO 187
            ry = sinT((y * 3) AND 255)
            addr = CLng(y) * 320 + 60
            FOR x = 60 TO 259
                POKE addr, (sx(x) + ry + diag(x + y)) AND 255
                addr = addr + 1
            NEXT x
        NEXT y

        BSAVE "PLASMA.DAT", 0, 64000
        DEF SEG
    END IF

    ' Flush keys that may have accumulated during the init pass
    WHILE INKEY$ <> "": WEND

    shift = 0

    ' Fade in: ramp palette brightness 0->63 over 63 frames
    ' Split loop eliminates j=(i+shift) AND 255 per iteration -- DAC auto-increments
    FOR bright = 0 TO 63
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 0
        FOR i = shift TO 255
            OUT &H3C9, pr(i) * bright \ 63
            OUT &H3C9, pg(i) * bright \ 63
            OUT &H3C9, pb(i) * bright \ 63
        NEXT i
        FOR i = 0 TO shift - 1
            OUT &H3C9, pr(i) * bright \ 63
            OUT &H3C9, pg(i) * bright \ 63
            OUT &H3C9, pb(i) * bright \ 63
        NEXT i
        shift = (shift + 2) AND 255
    NEXT bright

    ' Main loop: zero pixel writes -- only palette cycling
    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 0
        FOR i = shift TO 255
            OUT &H3C9, pr(i): OUT &H3C9, pg(i): OUT &H3C9, pb(i)
        NEXT i
        FOR i = 0 TO shift - 1
            OUT &H3C9, pr(i): OUT &H3C9, pg(i): OUT &H3C9, pb(i)
        NEXT i
        shift = (shift + 2) AND 255
    LOOP WHILE INKEY$ = ""

    ' Fade out: ramp brightness 63->0
    FOR bright = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 0
        FOR i = shift TO 255
            OUT &H3C9, pr(i) * bright \ 63
            OUT &H3C9, pg(i) * bright \ 63
            OUT &H3C9, pb(i) * bright \ 63
        NEXT i
        FOR i = 0 TO shift - 1
            OUT &H3C9, pr(i) * bright \ 63
            OUT &H3C9, pg(i) * bright \ 63
            OUT &H3C9, pb(i) * bright \ 63
        NEXT i
        shift = (shift + 2) AND 255
    NEXT bright

    SCREEN 0
END SUB

' ======================================================
' DrawText -- BIOS 8x8 ROM font scaled up by `scale`
'             drawn in palette colour `col`
' Font lives at segment &HF000, offset &HFA6E
' ======================================================
SUB DrawText (txt$, startX, startY, scale, col)
    DEFINT A-Z
    DIM chBase AS LONG
    DIM mask(7)
    mask(0) = 128: mask(1) = 64: mask(2) = 32: mask(3) = 16
    mask(4) = 8:   mask(5) = 4:  mask(6) = 2:  mask(7) = 1

    DEF SEG = &HF000

    FOR i = 1 TO LEN(txt$)
        ch = ASC(MID$(txt$, i, 1))
        cx = startX + (i - 1) * 8 * scale
        chBase = &HFA6E + CLng(ch) * 8   ' hoisted: one LONG multiply per char
        FOR row = 0 TO 7
            b = PEEK(chBase + row)
            IF b THEN
                FOR bit = 0 TO 7
                    IF (b AND mask(bit)) THEN
                        x1 = cx + bit * scale
                        y1 = startY + row * scale
                        LINE (x1, y1)-(x1 + scale - 1, y1 + scale - 1), col, BF
                    END IF
                NEXT bit
            END IF
        NEXT row
    NEXT i

    DEF SEG
END SUB

' ======================================================
' SCENE 5 -- Shadebobs
' Additive-blend radial gradient blobs in a 160x100
' centre region; fire palette (black->red->yellow->white).
' 5 bobs with varied Lissajous paths.  Max 5 overlapping
' blobs = 250 < 255 so output loop needs no clamp.
' ======================================================
SUB Scene5
    DEFINT A-Z
    DIM sinT(255)
    DIM bobD(575)                        ' 24x24 radial gradient blob (max 50)
    DIM spanL(23), spanR(23)            ' first/last lit dx per blob row (skip corners)
    DIM rowAddr(99) AS LONG              ' screen row offsets for effect region
    DIM bAngX(4), bAngY(4)
    DIM bSpdX(4), bSpdY(4)
    DIM bAmpX(4), bAmpY(4)
    DIM obx(4), oby(4), obxOff(4)       ' previous-frame positions for erase
    DIM odx0(4), odx1(4), ody0(4), ody1(4)
    DIM palR(255), palG(255), palB(255)
    DIM addrBase AS LONG, addrPix AS LONG

    SCREEN 13

    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 127)
    NEXT i

    ' Radial blob: max 50 at centre, 0 at radius 11; 5 bobs max = 250 <= 255
    FOR dy = 0 TO 23
        FOR dx = 0 TO 23
            dist = SQR(CLng(dx - 11) * (dx - 11) + CLng(dy - 11) * (dy - 11))
            v = INT(50 - dist * 50 / 11 + .5)
            IF v < 0 THEN v = 0
            bobD(dy * 24 + dx) = v
        NEXT dx
    NEXT dy

    ' Per-row lit spans: skip the zero corners of the circular blob.
    ' (drawing/erasing a 0 is a no-op, so this is purely a speed win)
    FOR dy = 0 TO 23
        spanL(dy) = 99: spanR(dy) = -1
        FOR dx = 0 TO 23
            IF bobD(dy * 24 + dx) > 0 THEN
                IF dx < spanL(dy) THEN spanL(dy) = dx
                spanR(dy) = dx
            END IF
        NEXT dx
    NEXT dy

    FOR y = 0 TO 99
        rowAddr(y) = CLng(y + 50) * 320 + 80
    NEXT y

    bAngX(0) =   0: bAngY(0) =   0: bSpdX(0) = 3: bSpdY(0) = 2: bAmpX(0) = 60: bAmpY(0) = 32
    bAngX(1) =  51: bAngY(1) =  77: bSpdX(1) = 2: bSpdY(1) = 3: bAmpX(1) = 50: bAmpY(1) = 28
    bAngX(2) = 102: bAngY(2) = 154: bSpdX(2) = 5: bSpdY(2) = 4: bAmpX(2) = 65: bAmpY(2) = 36
    bAngX(3) = 153: bAngY(3) = 230: bSpdX(3) = 4: bSpdY(3) = 7: bAmpX(3) = 45: bAmpY(3) = 25
    bAngX(4) = 204: bAngY(4) =  25: bSpdX(4) = 7: bSpdY(4) = 3: bAmpX(4) = 55: bAmpY(4) = 30

    FOR i = 0 TO 63
        palR(i) = i: palG(i) = 0: palB(i) = 0
    NEXT i
    FOR i = 64 TO 127
        palR(i) = 63: palG(i) = i - 64: palB(i) = 0
    NEXT i
    FOR i = 128 TO 191
        palR(i) = 63: palG(i) = 63: palB(i) = i - 128
    NEXT i
    FOR i = 192 TO 255
        palR(i) = 63: palG(i) = 63: palB(i) = 63
    NEXT i

    OUT &H3C8, 0
    FOR i = 0 TO 255
        OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    NEXT i

    ' Precompute initial old-positions so first erase is a harmless no-op
    FOR b = 0 TO 4
        obx(b) = 80 + (sinT(bAngX(b)) * bAmpX(b)) \ 127
        oby(b) = 50 + (sinT(bAngY(b)) * bAmpY(b)) \ 127
        odx0(b) = 11 - obx(b): IF odx0(b) < 0 THEN odx0(b) = 0
        odx1(b) = 170 - obx(b): IF odx1(b) > 23 THEN odx1(b) = 23
        ody0(b) = 11 - oby(b): IF ody0(b) < 0 THEN ody0(b) = 0
        ody1(b) = 110 - oby(b): IF ody1(b) > 23 THEN ody1(b) = 23
        obxOff(b) = obx(b) - 11
    NEXT b

    fadeV = 0

    DO
        DEF SEG = &HA000

        ' -- Erase all old bob footprints (zero their screen pixels) --
        ' All old positions zeroed before any new positions are drawn;
        ' this guarantees PEEK reads 0 when the first bob adds to a pixel.
        FOR b = 0 TO 4
            IF odx0(b) <= odx1(b) AND ody0(b) <= ody1(b) THEN
                addrBase = rowAddr(oby(b) + ody0(b) - 11) + obxOff(b)
                FOR dy = ody0(b) TO ody1(b)
                    sl = spanL(dy): IF sl < odx0(b) THEN sl = odx0(b)
                    sr = spanR(dy): IF sr > odx1(b) THEN sr = odx1(b)
                    IF sl <= sr THEN
                        addrPix = addrBase + sl
                        FOR dx = sl TO sr
                            POKE addrPix, 0
                            addrPix = addrPix + 1
                        NEXT dx
                    END IF
                    addrBase = addrBase + 320
                NEXT dy
            END IF
        NEXT b

        ' -- Advance bob angles --
        FOR b = 0 TO 4
            bAngX(b) = (bAngX(b) + bSpdX(b)) AND 255
            bAngY(b) = (bAngY(b) + bSpdY(b)) AND 255
        NEXT b

        ' -- Draw bobs additively: PEEK current value, add blob, POKE back --
        FOR b = 0 TO 4
            bx = 80 + (sinT(bAngX(b)) * bAmpX(b)) \ 127
            by = 50 + (sinT(bAngY(b)) * bAmpY(b)) \ 127

            dx0 = 11 - bx: IF dx0 < 0 THEN dx0 = 0
            dx1 = 170 - bx: IF dx1 > 23 THEN dx1 = 23
            dy0 = 11 - by: IF dy0 < 0 THEN dy0 = 0
            dy1 = 110 - by: IF dy1 > 23 THEN dy1 = 23

            IF dx0 <= dx1 AND dy0 <= dy1 THEN
                bxOff = bx - 11
                addrBase = rowAddr(by + dy0 - 11) + bxOff
                FOR dy = dy0 TO dy1
                    sl = spanL(dy): IF sl < dx0 THEN sl = dx0
                    sr = spanR(dy): IF sr > dx1 THEN sr = dx1
                    IF sl <= sr THEN
                        addrPix = addrBase + sl
                        bdRow = dy * 24 + sl
                        FOR dx = sl TO sr
                            POKE addrPix, PEEK(addrPix) + bobD(bdRow)
                            addrPix = addrPix + 1
                            bdRow = bdRow + 1
                        NEXT dx
                    END IF
                    addrBase = addrBase + 320
                NEXT dy
                obxOff(b) = bxOff
            END IF

            obx(b) = bx: oby(b) = by
            odx0(b) = dx0: odx1(b) = dx1: ody0(b) = dy0: ody1(b) = dy1
        NEXT b

        DEF SEG

        IF fadeV <= 63 THEN
            WAIT &H3DA, 8, 8
            WAIT &H3DA, 8
            OUT &H3C8, 0
            FOR i = 0 TO 255
                OUT &H3C9, palR(i) * fadeV \ 63
                OUT &H3C9, palG(i) * fadeV \ 63
                OUT &H3C9, palB(i) * fadeV \ 63
            NEXT i
            fadeV = fadeV + 1
        END IF

    LOOP WHILE INKEY$ = ""

    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 0
        FOR i = 0 TO 255
            OUT &H3C9, palR(i) * v \ 63
            OUT &H3C9, palG(i) * v \ 63
            OUT &H3C9, palB(i) * v \ 63
        NEXT i
    NEXT v

    SCREEN 0
END SUB

' ======================================================
' SCENE 6 -- Copper bars (raster bars)
' Every screen row y is filled once with palette index y
' (200 rows <= 256 entries), so the framebuffer never
' changes after init.  Bars move, cross and fade purely
' via DAC writes -- zero pixel writes per frame, same
' cost profile as the plasma's palette cycling.
' ======================================================
SUB Scene6
    DEFINT A-Z
    DIM sinT(255)                  ' signed sine LUT * 127
    DIM tri(23)                    ' 24-row brightness ramp, peak in the middle
    DIM gradR(144), gradG(144), gradB(144)   ' slot 0 = black; k*24+j+1 = bar k row j
    DIM rowC(199)                  ' per screen row: slot index into grad tables
    DIM barR(5), barG(5), barB(5)  ' 6 bar base colours
    DIM barPh(5), barSp(5), barAmp(5)
    DIM oldTop(5)                  ' previous top row per bar, for the erase pass

    SCREEN 13

    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 127)
    NEXT i

    ' Triangle brightness ramp: dark edges, bright centre = shiny metal look
    FOR j = 0 TO 23
        v = 63 - ABS(j - 12) * 63 \ 12
        IF v < 0 THEN v = 0
        tri(j) = v
    NEXT j

    ' Bar base colours (rainbow set from the title card, minus one)
    barR(0) = 63: barG(0) =  0: barB(0) =  0  ' red
    barR(1) = 63: barG(1) = 30: barB(1) =  0  ' orange
    barR(2) =  0: barG(2) = 50: barB(2) =  0  ' green
    barR(3) =  0: barG(3) = 50: barB(3) = 50  ' cyan
    barR(4) =  0: barG(4) =  0: barB(4) = 63  ' blue
    barR(5) = 35: barG(5) =  0: barB(5) = 63  ' violet

    ' Slot colour tables: all per-row RGB math done once here, none per frame
    gradR(0) = 0: gradG(0) = 0: gradB(0) = 0
    FOR k = 0 TO 5
        FOR j = 0 TO 23
            s = k * 24 + j + 1
            gradR(s) = barR(k) * tri(j) \ 63
            gradG(s) = barG(k) * tri(j) \ 63
            gradB(s) = barB(k) * tri(j) \ 63
        NEXT j
    NEXT k

    ' Motion: centre y = 100 + sin*amp/127.  Max amp 70 keeps bar rows
    ' inside 18..182 -- no clipping needed anywhere in the frame loop.
    barPh(0) =   0: barSp(0) = 2: barAmp(0) = 70
    barPh(1) =  42: barSp(1) = 3: barAmp(1) = 60
    barPh(2) =  85: barSp(2) = 2: barAmp(2) = 65
    barPh(3) = 128: barSp(3) = 4: barAmp(3) = 55
    barPh(4) = 170: barSp(4) = 3: barAmp(4) = 68
    barPh(5) = 213: barSp(5) = 5: barAmp(5) = 50

    ' Black out the palette so init is invisible (fade-in reveals the scene)
    OUT &H3C8, 0
    FOR i = 0 TO 255
        OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    NEXT i

    ' Fill each row with its own palette index -- the whole trick.
    ' 200 horizontal LINEs; the framebuffer is never touched again.
    FOR y = 0 TO 199
        LINE (0, y)-(319, y), y
    NEXT y

    ' Static text overlay on entry 250: bars pass behind the letters
    CALL DrawText("COPPER BARS", 72, 96, 2, 250)

    ' Valid initial old-positions so the first erase pass is harmless
    FOR k = 0 TO 5
        oldTop(k) = 88 + (sinT(barPh(k)) * barAmp(k)) \ 127
    NEXT k

    fadeV = 0

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8

        ' -- Erase old bar rows in the row->slot map --
        FOR k = 0 TO 5
            t = oldTop(k)
            FOR j = t TO t + 23
                rowC(j) = 0
            NEXT j
        NEXT k

        ' -- Advance and rewrite: later bars overwrite = in front when crossing --
        FOR k = 0 TO 5
            barPh(k) = (barPh(k) + barSp(k)) AND 255
            t = 88 + (sinT(barPh(k)) * barAmp(k)) \ 127
            s = k * 24 + 1
            FOR j = t TO t + 23
                rowC(j) = s
                s = s + 1
            NEXT j
            oldTop(k) = t
        NEXT k

        ' -- DAC pass: entries 0-199 written sequentially (auto-increment) --
        IF fadeV < 63 THEN
            fadeV = fadeV + 1
            OUT &H3C8, 0
            FOR i = 0 TO 199
                s = rowC(i)
                OUT &H3C9, gradR(s) * fadeV \ 63
                OUT &H3C9, gradG(s) * fadeV \ 63
                OUT &H3C9, gradB(s) * fadeV \ 63
            NEXT i
            OUT &H3C8, 250: OUT &H3C9, fadeV: OUT &H3C9, fadeV: OUT &H3C9, fadeV
        ELSE
            OUT &H3C8, 0
            FOR i = 0 TO 199
                s = rowC(i)
                OUT &H3C9, gradR(s): OUT &H3C9, gradG(s): OUT &H3C9, gradB(s)
            NEXT i
        END IF
    LOOP WHILE INKEY$ = ""

    ' Fade out (bars hold still) then leave
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 0
        FOR i = 0 TO 199
            s = rowC(i)
            OUT &H3C9, gradR(s) * v \ 63
            OUT &H3C9, gradG(s) * v \ 63
            OUT &H3C9, gradB(s) * v \ 63
        NEXT i
        OUT &H3C8, 250: OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v
    NEXT v

    SCREEN 0
END SUB

' ======================================================
' SCENE 7 -- Tunnel
' Static tunnel pattern (depth rings + kaleidoscope
' twist) written once at init; the flying motion is
' palette rotation over entries 0-239 -- zero pixel
' writes per frame, same trick as the plasma.
' 4-fold symmetry: SQR/ATN computed for one quadrant
' only (16,000 floats), mirrored to the other three.
' Entry 240 = black disc hiding the centre singularity.
' ======================================================
SUB Scene7
    DEFINT A-Z
    DIM palR(239), palG(239), palB(239)
    DIM barBase(7) AS LONG              ' progress-bar row start offsets
    DIM aTR AS LONG, aTL AS LONG        ' four mirrored pixel walkers
    DIM aBR AS LONG, aBL AS LONG
    DIM r AS SINGLE, a AS SINGLE        ' radius / angle (init only)
    DIM bri AS SINGLE

    SCREEN 13

    ' Warp palette: deep-blue troughs -> cyan-white crests.
    ' Two bands per 240-entry cycle so the rings read clearly.
    FOR i = 0 TO 239
        bri = (SIN(i * 6.28318 / 120) + 1) / 2
        palR(i) = INT(bri * bri * 50)
        palG(i) = INT(bri * 60)
        palB(i) = INT(20 + bri * 43)
    NEXT i

    ' Pattern cache, same scheme as the plasma: valid file = 7-byte BSAVE
    ' header + 64000 pixel bytes
    cached = 0
    OPEN "TUNNEL.DAT" FOR BINARY AS #1
    IF LOF(1) = 64007 THEN cached = 1
    CLOSE #1

    ' Black out the palette so init is invisible
    OUT &H3C8, 0
    FOR i = 0 TO 255
        OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    NEXT i

    IF cached THEN
        ' Later runs: load the pattern straight into video memory
        DEF SEG = &HA000
        BLOAD "TUNNEL.DAT", 0
        DEF SEG
    ELSE
        ' First run: compute the pattern (progress bar shown), cache it

        ' Progress bar colours
        OUT &H3C8, 253: OUT &H3C9, 63: OUT &H3C9, 63: OUT &H3C9, 63  ' fill: white
        OUT &H3C8, 254: OUT &H3C9, 15: OUT &H3C9, 15: OUT &H3C9, 15  ' track: grey

        ' Progress bar lives at rows y=190..197, x=60..259
        FOR barRow = 0 TO 7
            barBase(barRow) = CLng(190 + barRow) * 320
        NEXT barRow

        DEF SEG = &HA000

        ' Empty bar track
        FOR barRow = 0 TO 7
            FOR barX = 60 TO 259
                POKE barBase(barRow) + barX, 254
            NEXT barX
        NEXT barRow

        ' Compute one quadrant, write 4 mirrored pixels per value.
        ' Centre (160,100): right x=160+dx / left x=159-dx,
        '                   bottom y=100+dy / top y=99-dy
        FOR dy = 0 TO 99
            aTR = CLng(99 - dy) * 320 + 160
            aTL = aTR - 1
            aBR = CLng(100 + dy) * 320 + 160
            aBL = aBR - 1
            FOR dx = 0 TO 159
                r = SQR(CLng(dx) * dx + CLng(dy) * dy)
                IF r < 24 THEN
                    c = 240             ' black disc hides centre aliasing
                ELSE
                    IF dx = 0 THEN a = 1.570796 ELSE a = ATN(dy / dx)
                    c = (INT(24000 / r) + INT(a * 76.4)) MOD 240
                END IF
                POKE aTR, c: POKE aTL, c: POKE aBR, c: POKE aBL, c
                aTR = aTR + 1: aTL = aTL - 1
                aBR = aBR + 1: aBL = aBL - 1
            NEXT dx
            ' Rows 190-197 are the bar itself: tunnel just overwrote this bar
            ' row, so rebuild it (fill up to current progress, track after)
            IF dy >= 90 AND dy <= 97 THEN
                FOR barX = 60 TO 61 + dy * 2
                    POKE barBase(dy - 90) + barX, 253
                NEXT barX
                FOR barX = 62 + dy * 2 TO 259
                    POKE barBase(dy - 90) + barX, 254
                NEXT barX
            END IF
            ' Advance bar: two columns per quadrant row (100 rows -> 200 px)
            FOR barRow = 0 TO 7
                POKE barBase(barRow) + 60 + dy * 2, 253
                POKE barBase(barRow) + 61 + dy * 2, 253
            NEXT barRow
        NEXT dy

        ' Repair pass: replace the finished bar with real tunnel pixels
        ' (only 8 rows x 200 px; r >= 90 here so never inside the disc)
        FOR y = 190 TO 197
            qdy = y - 100
            aBR = CLng(y) * 320
            FOR x = 60 TO 259
                IF x >= 160 THEN qdx = x - 160 ELSE qdx = 159 - x
                r = SQR(CLng(qdx) * qdx + CLng(qdy) * qdy)
                IF qdx = 0 THEN a = 1.570796 ELSE a = ATN(qdy / qdx)
                POKE aBR + x, (INT(24000 / r) + INT(a * 76.4)) MOD 240
            NEXT x
        NEXT y

        BSAVE "TUNNEL.DAT", 0, 64000
        DEF SEG
    END IF

    ' Flush keys that may have accumulated during the init pass
    WHILE INKEY$ <> "": WEND

    shift = 0

    ' Fade in while already flying
    FOR fadeV = 0 TO 63
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 0
        FOR i = shift TO 239
            OUT &H3C9, palR(i) * fadeV \ 63
            OUT &H3C9, palG(i) * fadeV \ 63
            OUT &H3C9, palB(i) * fadeV \ 63
        NEXT i
        FOR i = 0 TO shift - 1
            OUT &H3C9, palR(i) * fadeV \ 63
            OUT &H3C9, palG(i) * fadeV \ 63
            OUT &H3C9, palB(i) * fadeV \ 63
        NEXT i
        shift = shift + 3: IF shift >= 240 THEN shift = 0
    NEXT fadeV

    ' Main loop: zero pixel writes -- palette rotation only.
    ' Increasing shift moves rings outward = flying forward.
    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 0
        FOR i = shift TO 239
            OUT &H3C9, palR(i): OUT &H3C9, palG(i): OUT &H3C9, palB(i)
        NEXT i
        FOR i = 0 TO shift - 1
            OUT &H3C9, palR(i): OUT &H3C9, palG(i): OUT &H3C9, palB(i)
        NEXT i
        shift = shift + 3: IF shift >= 240 THEN shift = 0
    LOOP WHILE INKEY$ = ""

    ' Fade out, still flying
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 0
        FOR i = shift TO 239
            OUT &H3C9, palR(i) * v \ 63
            OUT &H3C9, palG(i) * v \ 63
            OUT &H3C9, palB(i) * v \ 63
        NEXT i
        FOR i = 0 TO shift - 1
            OUT &H3C9, palR(i) * v \ 63
            OUT &H3C9, palG(i) * v \ 63
            OUT &H3C9, palB(i) * v \ 63
        NEXT i
        shift = shift + 3: IF shift >= 240 THEN shift = 0
    NEXT v

    SCREEN 0
END SUB

' ======================================================
' SCENE 8 -- Credits crawl (ALWAYS the final scene)
' Star-Wars-style scroller: yellow text rises through a
' 240x140 window and vanishes over a horizon line, with
' static stars around it and drifting stars inside.
' The scroll is a single GET/PUT pair per step -- the
' block copy runs in the interpreter's internal machine
' code, so the only per-step BASIC work is one new
' scanline of text entering at the bottom.
' Ends on its own after the last credit clears the
' window (or early on keypress), then fades to black.
' ======================================================
SUB Scene8
    DEFINT A-Z
    DIM t$(30)                 ' credit lines
    DIM gb(127)                ' glyph bytes for the incoming line (16 chars x 8 rows)
    DIM mask(7)
    DIM rowA AS LONG           ' framebuffer offset of the window's bottom scanline
    DIM chB AS LONG            ' BIOS font address (exceeds INTEGER range)
    bufN = 16700               ' variable bound -> dynamic (far) array, keeps the
    DIM buf(bufN)              ' 33 KB GET/PUT block out of the 64 KB data segment

    ' Scroll window: x 40..279 (240 wide = 15 chars at scale 2), y 45..184.
    ' y=45 is the horizon: PUT overwrites the top scanline each step, so
    ' text disappears row by row as it crosses it.

    SCREEN 13

    mask(0) = 128: mask(1) = 64: mask(2) = 32: mask(3) = 16
    mask(4) = 8:   mask(5) = 4:  mask(6) = 2:  mask(7) = 1

    n = 0
    t$(n) = "MEGA DEMO": n = n + 1
    t$(n) = "": n = n + 1
    t$(n) = "": n = n + 1
    t$(n) = "* CODE *": n = n + 1
    t$(n) = "JAY": n = n + 1
    t$(n) = "": n = n + 1
    t$(n) = "* VISUALS *": n = n + 1
    t$(n) = "JAY": n = n + 1
    t$(n) = "": n = n + 1
    t$(n) = "* TOOLS *": n = n + 1
    t$(n) = "QBASIC 1.1": n = n + 1
    t$(n) = "DOSBOX-X": n = n + 1
    t$(n) = "": n = n + 1
    t$(n) = "* SCENES *": n = n + 1
    t$(n) = "STARFIELD": n = n + 1
    t$(n) = "TITLE CARD": n = n + 1
    t$(n) = "WIREFRAME CUBE": n = n + 1
    t$(n) = "PLASMA": n = n + 1
    t$(n) = "COPPER BARS": n = n + 1
    t$(n) = "TUNNEL": n = n + 1
    t$(n) = "VECTOR MORPH": n = n + 1
    t$(n) = "STARSHIP": n = n + 1
    t$(n) = "": n = n + 1
    t$(n) = "* GREETZ *": n = n + 1
    t$(n) = "THE DEMOSCENE": n = n + 1
    t$(n) = "": n = n + 1
    t$(n) = "": n = n + 1
    t$(n) = "THANKS FOR": n = n + 1
    t$(n) = "WATCHING!": n = n + 1
    t$(n) = "": n = n + 1
    t$(n) = "THE END": n = n + 1
    nLines = n - 1

    ' Palette: stars on 1-3 (grey ramp), crawl text on 5 (SW yellow).
    ' All start black; faded in during the first 63 frames.
    OUT &H3C8, 1
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 5: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0

    ' Static stars everywhere except the scroll window
    RANDOMIZE TIMER
    FOR i = 1 TO 60
        DO
            x = INT(RND * 320)
            y = INT(RND * 200)
        LOOP WHILE x >= 40 AND x <= 279 AND y >= 45 AND y <= 184
        PSET (x, y), INT(RND * 3) + 1
    NEXT i

    rowA = CLng(184) * 320
    curLine = 0: lineRow = 0: lnLen = 0: startX = 0
    doneRows = 0: fadeV = 0: frame = 0

    DEF SEG = &HA000

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        frame = frame + 1

        ' Fade in stars + text colour over the first 63 frames
        IF fadeV < 63 THEN
            fadeV = fadeV + 1
            v = 20 * fadeV \ 63
            OUT &H3C8, 1: OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v
            v = 35 * fadeV \ 63
            OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v
            v = 55 * fadeV \ 63
            OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v
            OUT &H3C8, 5
            OUT &H3C9, 63 * fadeV \ 63
            OUT &H3C9, 52 * fadeV \ 63
            OUT &H3C9, 0
        END IF

        ' Scroll every 2nd frame (30 px/s) -- stately crawl speed
        IF (frame AND 1) = 0 THEN
            ' Move the whole window up 1px in two statements
            GET (40, 46)-(279, 184), buf
            PUT (40, 45), buf, PSET

            ' Fresh bottom scanline
            LINE (40, 184)-(279, 184), 0

            ' New credit line entering? Prefetch its glyph rows from the
            ' BIOS ROM font so the per-scanline draw needs no DEF SEG swap
            IF lineRow = 0 THEN
                IF curLine <= nLines THEN ln$ = t$(curLine) ELSE ln$ = ""
                lnLen = LEN(ln$)
                startX = 40 + (240 - lnLen * 16) \ 2
                IF lnLen THEN
                    DEF SEG = &HF000
                    FOR c = 0 TO lnLen - 1
                        chB = &HFA6E + CLng(ASC(MID$(ln$, c + 1, 1))) * 8
                        FOR rw = 0 TO 7
                            gb(c * 8 + rw) = PEEK(chB + rw)
                        NEXT rw
                    NEXT c
                    DEF SEG = &HA000
                END IF
            END IF

            ' Rows 0-15 of a line are glyph pixels (8 font rows, scale 2);
            ' rows 16-23 are the gap between lines (left empty -- stars
            ' inside the window would ride along with the text)
            IF lineRow < 16 AND lnLen > 0 THEN
                gRow = lineRow \ 2
                FOR c = 0 TO lnLen - 1
                    b = gb(c * 8 + gRow)
                    IF b THEN
                        cx = startX + c * 16
                        FOR bit = 0 TO 7
                            IF b AND mask(bit) THEN
                                POKE rowA + cx, 5
                                POKE rowA + cx + 1, 5
                            END IF
                            cx = cx + 2
                        NEXT bit
                    END IF
                NEXT c
            END IF

            lineRow = lineRow + 1
            IF lineRow = 24 THEN
                lineRow = 0
                curLine = curLine + 1
            END IF
            ' After the last line, count scroll steps until the window drains
            IF curLine > nLines THEN doneRows = doneRows + 1
        END IF
    LOOP WHILE INKEY$ = "" AND doneRows < 145

    DEF SEG

    ' Fade to black -- the demo ends here
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        pv = 20 * v \ 63
        OUT &H3C8, 1: OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        pv = 35 * v \ 63
        OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        pv = 55 * v \ 63
        OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        OUT &H3C8, 5: OUT &H3C9, v: OUT &H3C9, 52 * v \ 63: OUT &H3C9, 0
    NEXT v

    SCREEN 0
END SUB

' ======================================================
' SCENE 9 -- Vector morph
' Rotating wireframe that melts between 4 shapes.  All
' shapes share the cube's 8-vertex / 12-edge topology;
' "missing" vertices are collapsed onto one point, whose
' zero-length edges are invisible (a pyramid is a cube
' whose back face shrank to a dot).  Rotation/projection
' math is Scene3's, applied to interpolated vertices.
' ======================================================
SUB Scene9
    DEFINT A-Z
    DIM sinT(255)
    DIM mx(3, 7), my(3, 7), mz(3, 7)   ' 4 shapes x 8 vertices
    DIM vax(7), vay(7), vaz(7)         ' morph source shape
    DIM vbx(7), vby(7), vbz(7)         ' morph target shape
    DIM vx(7), vy(7), vz(7)            ' current (interpolated) vertices
    DIM px(7), py(7)
    DIM e1(11), e2(11)
    DIM tx AS LONG, ty AS LONG, tz AS LONG
    DIM xL AS LONG, yL AS LONG, zL AS LONG

    SCREEN 13

    ' Wireframe colour starts black -- fades in to gold (63, 40, 0)
    OUT &H3C8, 4: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0

    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 128 + .5)
    NEXT i

    ' Shape 0: cube (same as Scene3)
    mx(0, 0) = -80: my(0, 0) = -80: mz(0, 0) = -80
    mx(0, 1) =  80: my(0, 1) = -80: mz(0, 1) = -80
    mx(0, 2) =  80: my(0, 2) =  80: mz(0, 2) = -80
    mx(0, 3) = -80: my(0, 3) =  80: mz(0, 3) = -80
    mx(0, 4) = -80: my(0, 4) = -80: mz(0, 4) =  80
    mx(0, 5) =  80: my(0, 5) = -80: mz(0, 5) =  80
    mx(0, 6) =  80: my(0, 6) =  80: mz(0, 6) =  80
    mx(0, 7) = -80: my(0, 7) =  80: mz(0, 7) =  80

    ' Shape 1: pyramid -- big front base, back face collapsed to an apex
    mx(1, 0) = -90: my(1, 0) = -90: mz(1, 0) = -70
    mx(1, 1) =  90: my(1, 1) = -90: mz(1, 1) = -70
    mx(1, 2) =  90: my(1, 2) =  90: mz(1, 2) = -70
    mx(1, 3) = -90: my(1, 3) =  90: mz(1, 3) = -70
    FOR i = 4 TO 7
        mx(1, i) = 0: my(1, i) = 0: mz(1, i) = 110
    NEXT i

    ' Shape 2: gem -- truncated pyramid (small front, big back)
    mx(2, 0) = -35: my(2, 0) = -35: mz(2, 0) = -70
    mx(2, 1) =  35: my(2, 1) = -35: mz(2, 1) = -70
    mx(2, 2) =  35: my(2, 2) =  35: mz(2, 2) = -70
    mx(2, 3) = -35: my(2, 3) =  35: mz(2, 3) = -70
    mx(2, 4) = -100: my(2, 4) = -100: mz(2, 4) = 50
    mx(2, 5) =  100: my(2, 5) = -100: mz(2, 5) = 50
    mx(2, 6) =  100: my(2, 6) =  100: mz(2, 6) = 50
    mx(2, 7) = -100: my(2, 7) =  100: mz(2, 7) = 50

    ' Shape 3: antiprism star -- front square turned 45 degrees and
    ' stretched to points, back square small and straight
    mx(3, 0) =    0: my(3, 0) = -110: mz(3, 0) = -60
    mx(3, 1) =  110: my(3, 1) =    0: mz(3, 1) = -60
    mx(3, 2) =    0: my(3, 2) =  110: mz(3, 2) = -60
    mx(3, 3) = -110: my(3, 3) =    0: mz(3, 3) = -60
    mx(3, 4) = -60: my(3, 4) = -60: mz(3, 4) = 60
    mx(3, 5) =  60: my(3, 5) = -60: mz(3, 5) = 60
    mx(3, 6) =  60: my(3, 6) =  60: mz(3, 6) = 60
    mx(3, 7) = -60: my(3, 7) =  60: mz(3, 7) = 60

    ' Same 12-edge list as the cube
    e1(0) = 0: e2(0) = 1:  e1(1) = 1: e2(1) = 2
    e1(2) = 2: e2(2) = 3:  e1(3) = 3: e2(3) = 0
    e1(4) = 4: e2(4) = 5:  e1(5) = 5: e2(5) = 6
    e1(6) = 6: e2(6) = 7:  e1(7) = 7: e2(7) = 4
    e1(8) = 0: e2(8) = 4:  e1(9) = 1: e2(9) = 5
    e1(10) = 2: e2(10) = 6: e1(11) = 3: e2(11) = 7

    ' Start as the cube, first morph target is the pyramid
    FOR i = 0 TO 7
        vx(i) = mx(0, i): vy(i) = my(0, i): vz(i) = mz(0, i)
        px(i) = 160: py(i) = 100
    NEXT i
    curS = 0: nxtS = 1
    morphing = 0: holdT = 120: mt = 0

    angY = 0: angX = 0
    fadeV = 0

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8

        IF fadeV < 63 THEN
            fadeV = fadeV + 1
            OUT &H3C8, 4
            OUT &H3C9, fadeV
            OUT &H3C9, 40 * fadeV \ 63
            OUT &H3C9, 0
        END IF

        ' Erase last frame's edges
        FOR e = 0 TO 11
            a = e1(e): b = e2(e)
            LINE (px(a), py(a))-(px(b), py(b)), 0
        NEXT e

        ' Morph state machine: hold a shape, then blend to the next
        ' over 64 frames.  (target-source) * mt stays within INTEGER:
        ' max delta ~220 x 64 = 14,080 < 32,767.
        IF morphing THEN
            mt = mt + 1
            FOR i = 0 TO 7
                vx(i) = vax(i) + (vbx(i) - vax(i)) * mt \ 64
                vy(i) = vay(i) + (vby(i) - vay(i)) * mt \ 64
                vz(i) = vaz(i) + (vbz(i) - vaz(i)) * mt \ 64
            NEXT i
            IF mt = 64 THEN
                morphing = 0
                holdT = 120
                curS = nxtS
                nxtS = (nxtS + 1) AND 3
            END IF
        ELSE
            holdT = holdT - 1
            IF holdT = 0 THEN
                morphing = 1: mt = 0
                FOR i = 0 TO 7
                    vax(i) = mx(curS, i): vbx(i) = mx(nxtS, i)
                    vay(i) = my(curS, i): vby(i) = my(nxtS, i)
                    vaz(i) = mz(curS, i): vbz(i) = mz(nxtS, i)
                NEXT i
            END IF
        END IF

        sinY = sinT(angY)
        cosY = sinT((angY + 64) AND 255)
        sinX = sinT(angX)
        cosX = sinT((angX + 64) AND 255)

        ' Rotate + project (identical math to Scene3)
        FOR i = 0 TO 7
            xL = vx(i): zL = vz(i)
            tx = (xL * cosY - zL * sinY) \ 128
            tz = (xL * sinY + zL * cosY) \ 128

            yL = vy(i)
            ty = (yL * cosX - tz * sinX) \ 128
            tz = (yL * sinX + tz * cosX) \ 128

            denom = tz + 300
            IF denom < 1 THEN denom = 1
            px(i) = 160 + (tx * 200 \ denom)
            py(i) = 100 + (ty * 200 \ denom)
        NEXT i

        FOR e = 0 TO 11
            a = e1(e): b = e2(e)
            LINE (px(a), py(a))-(px(b), py(b)), 4
        NEXT e

        angY = (angY + 2) AND 255
        angX = (angX + 1) AND 255

    LOOP WHILE INKEY$ = ""

    ' Fade out
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 4: OUT &H3C9, v: OUT &H3C9, 40 * v \ 63: OUT &H3C9, 0
    NEXT v

    SCREEN 0
END SUB

' ======================================================
' SCENE 10 -- Starship flight
' A 2D polygon fighter (9 LINE segments) swoops around
' on sine paths, banking into its turns, while a 3D
' perspective starfield streams toward the viewer.
' Stars are POKE erase/draw (Scene1 style, plus a depth
' divide); the ship is erase-then-redraw LINEs (Scene3
' style).  Banking is a real 7-point sine-LUT rotation.
' ======================================================
SUB Scene10
    DEFINT A-Z
    DIM sinT(255)                  ' signed sine LUT * 128
    DIM rowAddr(199) AS LONG       ' framebuffer row offsets
    DIM sx(59), sy(59), sz(59)     ' starfield world coords (z = depth)
    DIM oadr(59) AS LONG           ' last drawn address per star (-1 = off-screen)
    DIM spx(6), spy(6)             ' ship shape, local coords
    DIM dpx(6), dpy(6)             ' transformed points, this frame
    DIM opx(6), opy(6)             ' transformed points, last frame (for erase)
    DIM se1(7), se2(7)             ' ship edge list

    SCREEN 13

    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 128 + .5)
    NEXT i

    FOR y = 0 TO 199
        rowAddr(y) = CLng(y) * 320
    NEXT y

    ' Ship seen from behind-above: nose, body, swept wings, tail
    spx(0) =   0: spy(0) = -18     ' nose
    spx(1) =  -8: spy(1) =   6     ' body left
    spx(2) =   8: spy(2) =   6     ' body right
    spx(3) = -26: spy(3) =  14     ' wingtip left
    spx(4) =  26: spy(4) =  14     ' wingtip right
    spx(5) =   0: spy(5) =  10     ' tail centre
    ' spx(6)/spy(6) set per frame: flickering thruster flame end

    se1(0) = 0: se2(0) = 1         ' nose -> body left
    se1(1) = 0: se2(1) = 2         ' nose -> body right
    se1(2) = 1: se2(2) = 5         ' body left -> tail
    se1(3) = 2: se2(3) = 5         ' body right -> tail
    se1(4) = 1: se2(4) = 3         ' wing left, leading edge
    se1(5) = 3: se2(5) = 5         ' wing left, trailing edge
    se1(6) = 2: se2(6) = 4         ' wing right, leading edge
    se1(7) = 4: se2(7) = 5         ' wing right, trailing edge

    ' Palette: 1-3 star greys (far -> near), 4 ship steel-blue, 6 flame
    ' orange; everything starts black and fades in
    OUT &H3C8, 1
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 4: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 6: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0

    RANDOMIZE TIMER
    FOR i = 0 TO 59
        sx(i) = INT(RND * 320) - 160
        sy(i) = INT(RND * 200) - 100
        sz(i) = 16 + INT(RND * 234)
        oadr(i) = -1
    NEXT i

    ' Init "previous" ship points so the first erase is harmless
    FOR i = 0 TO 6
        opx(i) = 160: opy(i) = 100
    NEXT i

    phX = 0: phY = 64
    fadeV = 0

    DEF SEG = &HA000

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8

        IF fadeV < 63 THEN
            fadeV = fadeV + 1
            v = 20 * fadeV \ 63
            OUT &H3C8, 1: OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v
            v = 35 * fadeV \ 63
            OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v
            v = 55 * fadeV \ 63
            OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v
            OUT &H3C8, 4
            OUT &H3C9, 40 * fadeV \ 63
            OUT &H3C9, 52 * fadeV \ 63
            OUT &H3C9, fadeV
            OUT &H3C8, 6
            OUT &H3C9, fadeV
            OUT &H3C9, 30 * fadeV \ 63
            OUT &H3C9, 0
        END IF

        ' -- 3D starfield: erase, advance depth, project, draw --
        FOR i = 0 TO 59
            IF oadr(i) >= 0 THEN POKE oadr(i), 0
            sz(i) = sz(i) - 3
            IF sz(i) < 16 THEN
                sx(i) = INT(RND * 320) - 160
                sy(i) = INT(RND * 200) - 100
                sz(i) = 250
            END IF
            px = 160 + (sx(i) * 128) \ sz(i)
            py = 100 + (sy(i) * 128) \ sz(i)
            IF px < 0 OR px > 319 OR py < 0 OR py > 199 THEN
                oadr(i) = -1
            ELSE
                IF sz(i) > 170 THEN
                    c = 1
                ELSEIF sz(i) > 85 THEN
                    c = 2
                ELSE
                    c = 3
                END IF
                oadr(i) = rowAddr(py) + px
                POKE oadr(i), c
            END IF
        NEXT i

        ' -- Erase last frame's ship + flame --
        FOR e = 0 TO 7
            LINE (opx(se1(e)), opy(se1(e)))-(opx(se2(e)), opy(se2(e))), 0
        NEXT e
        LINE (opx(5), opy(5))-(opx(6), opy(6)), 0

        ' -- Move on a Lissajous path; bank into the horizontal velocity
        '    (velocity of sin is cos = sine table + quarter period) --
        phX = (phX + 2) AND 255
        phY = (phY + 3) AND 255
        shipX = 160 + (sinT(phX) * 90) \ 128
        shipY = 100 + (sinT(phY) * 50) \ 128
        bk = (sinT((phX + 64) AND 255) * 18) \ 128
        sinB = sinT(bk AND 255)
        cosB = sinT((bk + 64) AND 255)

        ' Thruster flame flickers in ship-local coords, banks with the hull
        spx(6) = 0: spy(6) = 16 + INT(RND * 9)

        FOR i = 0 TO 6
            dpx(i) = shipX + (spx(i) * cosB - spy(i) * sinB) \ 128
            dpy(i) = shipY + (spx(i) * sinB + spy(i) * cosB) \ 128
        NEXT i

        ' -- Draw ship + flame --
        FOR e = 0 TO 7
            LINE (dpx(se1(e)), dpy(se1(e)))-(dpx(se2(e)), dpy(se2(e))), 4
        NEXT e
        LINE (dpx(5), dpy(5))-(dpx(6), dpy(6)), 6

        FOR i = 0 TO 6
            opx(i) = dpx(i): opy(i) = dpy(i)
        NEXT i

    LOOP WHILE INKEY$ = ""

    DEF SEG

    ' Fade out
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        pv = 20 * v \ 63
        OUT &H3C8, 1: OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        pv = 35 * v \ 63
        OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        pv = 55 * v \ 63
        OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        OUT &H3C8, 4: OUT &H3C9, 40 * v \ 63: OUT &H3C9, 52 * v \ 63: OUT &H3C9, v
        OUT &H3C8, 6: OUT &H3C9, v: OUT &H3C9, 30 * v \ 63: OUT &H3C9, 0
    NEXT v

    SCREEN 0
END SUB

REM  Mega Demo -- QBasic 1.1 / SCREEN 13 (320x200x256)

DEFINT A-Z

CALL Scene1
CALL Scene2
CALL Scene3
CALL Scene4
' CALL Scene5   ' shadebobs -- too slow in interpreter; revisit with CALL ABSOLUTE
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
        WAIT &H3DA, 8
        pv = v * 20 \ 63: OUT &H3C8, 1: OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        pv = v * 40 \ 63: OUT &H3C8, 2: OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        OUT &H3C8, 3: OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v
    NEXT v
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
        PSET (INT(RND * 320), INT(RND * 200)), 13 + k
        twPh(k) = INT(RND * 256)
        twSp(k) = INT(RND * 4) + 2
        OUT &H3C8, 13 + k: OUT &H3C9, 40: OUT &H3C9, 40: OUT &H3C9, 40
    NEXT k

    ' Fade in all 7 rainbow colours simultaneously
    FOR v = 0 TO 63
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

    ' Unsigned sine table: 0-255
    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 127 + 128)
    NEXT i

    ' Precompute x component once -- saves one table lookup per pixel during init
    FOR x = 0 TO 319
        sx(x) = sinT((x * 2) AND 255)
    NEXT x

    ' Precompute diagonal component, indexed directly by (x+y) -- removes the
    ' per-pixel running-index add+AND in the init loop
    FOR k = 0 TO 575
        diag(k) = sinT(k AND 255)
    NEXT k

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

    ' Progress bar colours (overwritten by palette cycling on first animation frame)
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
        ' Advance bar: one pixel column per row computed (y 0..199 -> x 60..259)
        FOR barRow = 0 TO 7
            POKE barBase(barRow) + 60 + y, 253
        NEXT barRow
    NEXT y
    DEF SEG

    ' Flush keys that may have accumulated during the init pass
    WHILE INKEY$ <> "": WEND

    shift = 0

    ' Fade in: ramp palette brightness 0->63 over 63 frames
    ' Split loop eliminates j=(i+shift) AND 255 per iteration -- DAC auto-increments
    FOR bright = 0 TO 63
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
    DIM addr AS LONG
    DIM mask(7)
    mask(0) = 128: mask(1) = 64: mask(2) = 32: mask(3) = 16
    mask(4) = 8:   mask(5) = 4:  mask(6) = 2:  mask(7) = 1

    DEF SEG = &HF000

    FOR i = 1 TO LEN(txt$)
        ch = ASC(MID$(txt$, i, 1))
        cx = startX + (i - 1) * 8 * scale
        FOR row = 0 TO 7
            addr = &HFA6E + CLng(ch) * 8 + row
            b = PEEK(addr)
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

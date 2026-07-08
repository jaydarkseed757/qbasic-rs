REM  Mega Demo -- QBasic 1.1 / SCREEN 13 (320x200x256)

DEFINT A-Z

' Intro jingle: ascending 3-tone chime (PC speaker via PLAY, blocks briefly)
PLAY "O4 L4 C E G"

CALL Scene1
CALL Scene2
CALL Scene3
CALL Scene4
CALL Scene5    ' shadebobs (bit-field PUT additive blend)
CALL Scene11   ' dot sphere
CALL Scene6
CALL Scene7
CALL Scene14   ' rotozoomer
CALL Scene9    ' vector morph (numbered 9: Scene8 is reserved for the finale)
CALL Scene10   ' starship flight
CALL Scene13   ' death star trench run
CALL Scene15   ' platformer vignette (mario homage)
CALL Scene12   ' wavy sine scroller
CALL Scene8    ' credits crawl -- ALWAYS the final scene; add new scenes above
END

' ---- Scene15 sprite art: 4 sprites x 16 rows, 16 chars per row ----
' legend: . transparent  R red  S skin  B brown  O overalls
'         G goomba body  W white  D goomba feet
SpriteData:
' runner, frame 1 (legs spread)
DATA "......RRRRR....."
DATA ".....RRRRRRRRR.."
DATA ".....BBBSSBS...."
DATA "....BSBSSSBSS..."
DATA "....BSBBSSSBSSS."
DATA "....BBSSSSBBBB.."
DATA "......SSSSSS...."
DATA "....RRRRRRR....."
DATA "...RRRRRRRRRR..."
DATA "..SSRROROORRSS.."
DATA "..SSROOOOOORSS.."
DATA "....OOOOOOOO...."
DATA "....OOO..OOO...."
DATA "...OOO....OOO..."
DATA "..BBB......BBB.."
DATA ".BBBB......BBBB."
' runner, frame 2 (legs together)
DATA "......RRRRR....."
DATA ".....RRRRRRRRR.."
DATA ".....BBBSSBS...."
DATA "....BSBSSSBSS..."
DATA "....BSBBSSSBSSS."
DATA "....BBSSSSBBBB.."
DATA "......SSSSSS...."
DATA ".....RRRRRR....."
DATA "....RRRRRRRR...."
DATA "....RROOOORR...."
DATA "....SROOOORS...."
DATA ".....OOOOOO....."
DATA ".....OOOOO......"
DATA "......OOOO......"
DATA ".....BBBB......."
DATA "....BBBBB......."
' runner, jump (arm up, legs tucked)
DATA "......RRRRR..SS."
DATA ".....RRRRRRRR.SS"
DATA ".....BBBSSBS..S."
DATA "....BSBSSSBSS..."
DATA "....BSBBSSSBSS.."
DATA "....BBSSSSBBB..."
DATA "..SS..SSSSSS...."
DATA "..SSRRRRRRR....."
DATA "...RRRRRRRRRR..."
DATA "...RRROOOORR...."
DATA "....ROOOOOOR...."
DATA ".....OOOOOO....."
DATA "....OOO.OOO....."
DATA "....OOB.BOO....."
DATA "....BBB.BBB....."
DATA "................"
' mushroom man (goomba)
DATA "................"
DATA "....GGGGGGGG...."
DATA "...GGGGGGGGGG..."
DATA "..GGGGGGGGGGGG.."
DATA ".GGWWGGGGGGWWGG."
DATA ".GWBBWGGGGWBBWG."
DATA "GGGWWGGGGGGWWGGG"
DATA "GGGGGGGGGGGGGGGG"
DATA "GGGGGGGGGGGGGGGG"
DATA ".GGGGGGGGGGGGGG."
DATA "..GGGGGGGGGGGG.."
DATA "...GGGGGGGGGG..."
DATA "..DDDD....DDDD.."
DATA ".DDDDD....DDDDD."
DATA ".DDDDDD..DDDDDD."
DATA "................"

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
        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

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
        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

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

        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

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
        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

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
' SCENE 5 -- Shadebobs (bit-field PUT edition)
' True additive blending with zero per-pixel BASIC work:
' each of 4 bobs owns 2 bits of the pixel byte.  Draw =
' PUT sprite with OR (sets only its own bits); erase =
' PUT constant mask with AND (clears only its own bits,
' even under overlaps).  The palette does the adding:
' entry b renders the SUM of the four 2-bit levels
' through a fire ramp, so crossings glow genuinely
' hotter.  8 machine-speed PUTs per frame.
' ======================================================
SUB Scene5
    DEFINT A-Z
    DIM sinT(255)                      ' signed sine LUT * 127
    DIM lvl(575)                       ' 24x24 radial gradient, levels 0-3
    DIM palR(255), palG(255), palB(255)
    DIM fr(12), fg(12), fb(12)         ' fire ramp for bit-sums 0..12
    DIM bAngX(3), bAngY(3)
    DIM bSpdX(3), bSpdY(3)
    DIM bAmpX(3), bAmpY(3)
    DIM ox(3), oy(3)                   ' previous top-left per bob (erase)
    DIM addr AS LONG
    sprN = 290 * 8                     ' 4 draw sprites + 4 erase masks;
    DIM spr(sprN)                      ' variable bound -> dynamic array

    SCREEN 13

    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 127)
    NEXT i

    ' Radial gradient quantized to 4 levels: 3 at the core, 0 outside r=11
    FOR dy = 0 TO 23
        FOR dx = 0 TO 23
            v = INT((11 - SQR(CLng(dx - 11) * (dx - 11) + CLng(dy - 11) * (dy - 11))) * 4 / 11 + .5)
            IF v < 0 THEN v = 0
            IF v > 3 THEN v = 3
            lvl(dy * 24 + dx) = v
        NEXT dx
    NEXT dy

    ' Fire ramp over summed brightness 0..12: a single bob's core (3) is
    ' full red; overlaps push through orange/yellow (6-8) to white (9+)
    FOR s = 0 TO 12
        v = s * 21: IF v > 63 THEN v = 63
        fr(s) = v
        v = (s - 3) * 13: IF v < 0 THEN v = 0
        IF v > 63 THEN v = 63
        fg(s) = v
        v = (s - 8) * 16: IF v < 0 THEN v = 0
        IF v > 63 THEN v = 63
        fb(s) = v
    NEXT s

    ' Expand: palette entry i = fire colour of the sum of its 4 bit-fields
    FOR i = 0 TO 255
        s = (i AND 3) + (i \ 4 AND 3) + (i \ 16 AND 3) + (i \ 64 AND 3)
        palR(i) = fr(s): palG(i) = fg(s): palB(i) = fb(s)
    NEXT i

    ' Black out the palette: sprite prep below stays invisible
    OUT &H3C8, 0
    FOR i = 0 TO 255
        OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    NEXT i

    ' Build each bob's sprites via screen-corner GETs: the draw sprite is
    ' the gradient shifted into the bob's own 2-bit field (shl = 1, 4, 16,
    ' 64), the erase mask a constant block of 255 - 3*shl
    DEF SEG = &HA000
    shl = 1
    FOR k = 0 TO 3
        FOR dy = 0 TO 23
            addr = CLng(dy) * 320
            FOR dx = 0 TO 23
                POKE addr + dx, lvl(dy * 24 + dx) * shl
            NEXT dx
        NEXT dy
        GET (0, 0)-(23, 23), spr(k * 580)
        LINE (0, 0)-(23, 23), 255 - 3 * shl, BF
        GET (0, 0)-(23, 23), spr(k * 580 + 290)
        LINE (0, 0)-(23, 23), 0, BF
        shl = shl * 4
    NEXT k
    DEF SEG

    ' Lissajous paths.  PUT cannot clip, so top-left coords must stay in
    ' x 0..296 / y 0..176: base (136, 76) = centre (148, 88) minus 12
    bAngX(0) =   0: bAngY(0) =   0: bSpdX(0) = 3: bSpdY(0) = 2: bAmpX(0) = 120: bAmpY(0) = 70
    bAngX(1) =  64: bAngY(1) =  96: bSpdX(1) = 2: bSpdY(1) = 3: bAmpX(1) = 100: bAmpY(1) = 62
    bAngX(2) = 128: bAngY(2) = 160: bSpdX(2) = 5: bSpdY(2) = 4: bAmpX(2) = 130: bAmpY(2) = 75
    bAngX(3) = 192: bAngY(3) =  48: bSpdX(3) = 4: bSpdY(3) = 7: bAmpX(3) =  85: bAmpY(3) = 55

    ' Valid initial old-positions: first AND-erase on black is a no-op
    FOR b = 0 TO 3
        ox(b) = 136 + (sinT(bAngX(b)) * bAmpX(b)) \ 127
        oy(b) = 76 + (sinT(bAngY(b)) * bAmpY(b)) \ 127
    NEXT b

    fadeV = 0

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8

        IF fadeV < 63 THEN
            fadeV = fadeV + 1
            OUT &H3C8, 0
            FOR i = 0 TO 255
                OUT &H3C9, palR(i) * fadeV \ 63
                OUT &H3C9, palG(i) * fadeV \ 63
                OUT &H3C9, palB(i) * fadeV \ 63
            NEXT i
        END IF

        ' Bit-fields are independent, so erase/draw can interleave per bob
        FOR b = 0 TO 3
            PUT (ox(b), oy(b)), spr(b * 580 + 290), AND    ' clear own bits
            bAngX(b) = (bAngX(b) + bSpdX(b)) AND 255
            bAngY(b) = (bAngY(b) + bSpdY(b)) AND 255
            tx = 136 + (sinT(bAngX(b)) * bAmpX(b)) \ 127
            ty = 76 + (sinT(bAngY(b)) * bAmpY(b)) \ 127
            PUT (tx, ty), spr(b * 580), OR                 ' add own bits
            ox(b) = tx: oy(b) = ty
        NEXT b

        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

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
        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

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
        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

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
    DIM t$(36)                 ' credit lines
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
    t$(n) = "SHADEBOBS": n = n + 1
    t$(n) = "DOT SPHERE": n = n + 1
    t$(n) = "COPPER BARS": n = n + 1
    t$(n) = "TUNNEL": n = n + 1
    t$(n) = "ROTOZOOMER": n = n + 1
    t$(n) = "VECTOR MORPH": n = n + 1
    t$(n) = "STARSHIP": n = n + 1
    t$(n) = "TRENCH RUN": n = n + 1
    t$(n) = "PLATFORMER": n = n + 1
    t$(n) = "WAVY SCROLLER": n = n + 1
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

        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

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

        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

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

' ======================================================
' SCENE 11 -- Dot sphere
' Globe of 82 dots (8 latitude rings + 2 poles) spinning
' about a slowly wobbling tilted axis.  The spin is
' nearly free: each dot just advances around its own
' ring, so it costs two sine lookups instead of a
' rotation matrix.  Orthographic projection with depth
' shown as brightness (3 greens) -- no divides anywhere,
' all INTEGER math, POKE erase/draw like the starfields.
' ======================================================
SUB Scene11
    DEFINT A-Z
    DIM sinT(255)                  ' signed sine LUT * 128
    DIM rrA(81), py0(81), ph(81)   ' per dot: ring radius, height, ring phase
    DIM oadr(81) AS LONG           ' last drawn address per dot (for erase)
    DIM rowAddr(199) AS LONG

    SCREEN 13

    ' Depth greens 1-3 (far/dim -> near/bright), black until fade-in
    OUT &H3C8, 1
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0

    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 128 + .5)
    NEXT i

    FOR y = 0 TO 199
        rowAddr(y) = CLng(y) * 320
    NEXT y

    ' 8 latitude rings x 10 dots + 2 poles, radius 70; rings staggered so
    ' dots do not line up in vertical columns
    n = 0
    FOR j = 1 TO 8
        FOR k = 0 TO 9
            rrA(n) = INT(SIN(j * 3.14159 / 9) * 70)
            py0(n) = INT(COS(j * 3.14159 / 9) * 70)
            ph(n) = (k * 26 + j * 13) AND 255
            n = n + 1
        NEXT k
    NEXT j
    py0(n) = 70: rrA(n) = 0: ph(n) = 0: n = n + 1
    py0(n) = -70: rrA(n) = 0: ph(n) = 0
    ' oadr() defaults to 0: the first-frame erase hits one corner pixel of
    ' an all-black screen -- harmless

    ang = 0: wob = 0: fadeV = 0

    DEF SEG = &HA000

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8

        IF fadeV < 63 THEN
            fadeV = fadeV + 1
            OUT &H3C8, 1
            OUT &H3C9, 0: OUT &H3C9, 16 * fadeV \ 63: OUT &H3C9, 6 * fadeV \ 63
            OUT &H3C9, 0: OUT &H3C9, 36 * fadeV \ 63: OUT &H3C9, 14 * fadeV \ 63
            OUT &H3C9, 16 * fadeV \ 63: OUT &H3C9, fadeV: OUT &H3C9, 28 * fadeV \ 63
        END IF

        ' Axis wobble: tilt oscillates +-10 steps around 36 (~50 degrees);
        ' sin/cos of the tilt are constant across the dot loop
        tiltA = 36 + (sinT(wob) * 10) \ 128
        wob = (wob + 1) AND 255
        sinTl = sinT(tiltA)
        cosTl = sinT((tiltA + 64) AND 255)
        ang = (ang + 2) AND 255

        FOR i = 0 TO 81
            POKE oadr(i), 0
            ' Spin: the dot advances around its own latitude ring
            xs = (rrA(i) * sinT((ang + ph(i)) AND 255)) \ 128
            zs = (rrA(i) * sinT((ang + ph(i) + 64) AND 255)) \ 128
            ' Tilt the whole globe about X
            yr = (py0(i) * cosTl - zs * sinTl) \ 128
            zr = (py0(i) * sinTl + zs * cosTl) \ 128
            ' Depth cue: nearer dots brighter
            IF zr > 25 THEN
                c = 1
            ELSEIF zr > -25 THEN
                c = 2
            ELSE
                c = 3
            END IF
            oadr(i) = rowAddr(100 + yr) + 160 + xs
            POKE oadr(i), c
        NEXT i

        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

    DEF SEG

    ' Fade out
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 1
        OUT &H3C9, 0: OUT &H3C9, 16 * v \ 63: OUT &H3C9, 6 * v \ 63
        OUT &H3C9, 0: OUT &H3C9, 36 * v \ 63: OUT &H3C9, 14 * v \ 63
        OUT &H3C9, 16 * v \ 63: OUT &H3C9, v: OUT &H3C9, 28 * v \ 63
    NEXT v

    SCREEN 0
END SUB

' ======================================================
' SCENE 12 -- Wavy sine scroller
' Giant gold text glides right-to-left, each character
' bobbing on a travelling sine wave.  Every unique glyph
' is pre-rendered ONCE into a GET sprite at init (BIOS
' font via DrawText, scale 3 = 24x24); per frame each
' visible char is one LINE-BF box erase + one PUT --
' both single fast statements.  QBasic PUT cannot clip,
' so two decorative pillars frame the band and chars
' slide out from / vanish behind them.
' ======================================================
SUB Scene12
    DEFINT A-Z
    DIM sinT(255)                  ' signed sine LUT * 128
    DIM cmap(95)                   ' ASCII-32 -> sprite slot + 1 (0 = space/none)
    DIM cx(15), cy(15), ci(15)     ' active chars: x, last drawn y, sprite slot
    sprN = 290 * 45                ' 45 slots x (4 + 24*24)/2 ints; variable
    DIM spr(sprN)                  ' bound -> dynamic array, off the 64 KB DGROUP

    SCREEN 13

    ' Palette (all faded in from black):
    ' 1-3 stars, 5 text gold, 8 pillar body, 9 pillar edge
    OUT &H3C8, 1
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 5: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 8
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0

    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 128 + .5)
    NEXT i

    msg$ = "JAY'S QBASIC MEGA DEMO ... GREETZ TO THE DEMOSCENE ... "
    msg$ = msg$ + "QBASIC 1.1 FOREVER ... KEEP THE PIXELS FLYING ...      "
    msgLen = LEN(msg$)

    ' Pre-render each unique glyph once: draw at top-left (palette is
    ' black, so invisible), GET into its sprite slot, wipe the corner
    nSlots = 0
    FOR i = 1 TO msgLen
        a = ASC(MID$(msg$, i, 1)) - 32
        IF a > 0 THEN
            IF cmap(a) = 0 THEN
                CALL DrawText(MID$(msg$, i, 1), 0, 0, 3, 5)
                GET (0, 0)-(23, 23), spr(nSlots * 290)
                LINE (0, 0)-(23, 23), 0, BF
                nSlots = nSlots + 1
                cmap(a) = nSlots
            END IF
        END IF
    NEXT i

    ' Static stars above and below the scroll band (rows 55-144)
    RANDOMIZE TIMER
    FOR i = 1 TO 50
        DO
            x = INT(RND * 320)
            y = INT(RND * 200)
        LOOP WHILE y >= 55 AND y <= 144
        PSET (x, y), INT(RND * 3) + 1
    NEXT i

    cnt = 0: mi = 0: spawnCtr = 0: waveT = 0: fadeV = 0

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
            OUT &H3C8, 5
            OUT &H3C9, fadeV
            OUT &H3C9, 50 * fadeV \ 63
            OUT &H3C9, 10 * fadeV \ 63
            OUT &H3C8, 8
            OUT &H3C9, 8 * fadeV \ 63
            OUT &H3C9, 12 * fadeV \ 63
            OUT &H3C9, 34 * fadeV \ 63
            OUT &H3C9, 22 * fadeV \ 63
            OUT &H3C9, 32 * fadeV \ 63
            OUT &H3C9, 60 * fadeV \ 63
        END IF

        ' -- Erase old boxes, glide everyone 2px left --
        FOR s = 0 TO cnt - 1
            IF ci(s) THEN
                LINE (cx(s), cy(s))-(cx(s) + 23, cy(s) + 23), 0, BF
            END IF
            cx(s) = cx(s) - 2
        NEXT s

        ' -- Retire the leftmost char once fully behind the left pillar --
        IF cnt > 0 THEN
            IF cx(0) <= 8 THEN
                FOR s = 0 TO cnt - 2
                    cx(s) = cx(s + 1): cy(s) = cy(s + 1): ci(s) = ci(s + 1)
                NEXT s
                cnt = cnt - 1
            END IF
        END IF

        ' -- Spawn the next message char behind the right pillar --
        spawnCtr = spawnCtr - 2
        IF spawnCtr <= 0 THEN
            a = ASC(MID$(msg$, mi + 1, 1)) - 32
            IF a > 0 THEN ci(cnt) = cmap(a) ELSE ci(cnt) = 0
            cx(cnt) = 296
            cy(cnt) = 88
            cnt = cnt + 1
            mi = mi + 1: IF mi = msgLen THEN mi = 0
            spawnCtr = spawnCtr + 26
        END IF

        ' -- Travelling wave: y from each char's x plus a time phase --
        waveT = (waveT + 5) AND 255
        FOR s = 0 TO cnt - 1
            y = 88 + (sinT((cx(s) * 2 + waveT) AND 255) * 28) \ 128
            cy(s) = y
            IF ci(s) THEN
                PUT (cx(s), y), spr((ci(s) - 1) * 290), PSET
            END IF
        NEXT s

        ' -- Pillars redrawn over the spawn/retire zones --
        LINE (0, 55)-(31, 144), 8, BF
        LINE (288, 55)-(319, 144), 8, BF
        LINE (31, 55)-(31, 144), 9
        LINE (288, 55)-(288, 144), 9

        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

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
        OUT &H3C8, 5: OUT &H3C9, v: OUT &H3C9, 50 * v \ 63: OUT &H3C9, 10 * v \ 63
        OUT &H3C8, 8
        OUT &H3C9, 8 * v \ 63: OUT &H3C9, 12 * v \ 63: OUT &H3C9, 34 * v \ 63
        OUT &H3C9, 22 * v \ 63: OUT &H3C9, 32 * v \ 63: OUT &H3C9, 60 * v \ 63
    NEXT v

    SCREEN 0
END SUB

' ======================================================
' SCENE 13 -- Death Star trench run
' One-point-perspective wireframe trench rushing toward
' the viewer: static wall/floor seams converge on the
' vanishing point, cross-seam "rungs" sweep forward (3
' clipped LINEs each) and respawn at the far end.  An
' X-wing polygon banks low over the floor, firing red
' laser bolts that converge on the glowing exhaust port
' at the vanishing point.  All erase-then-redraw LINEs
' plus two divides per rung -- Scene3-class budget.
' ======================================================
SUB Scene13
    DEFINT A-Z
    DIM sinT(255)                       ' signed sine LUT * 128
    DIM rz(9)                           ' rung depths
    DIM olx(9), orx(9), ofy(9), oty(9)  ' last drawn rung coords (erase)
    DIM spx(11), spy(11)                ' X-wing shape, local coords
    DIM dpx(11), dpy(11)                ' transformed, this frame
    DIM opx(11), opy(11)                ' transformed, last frame (erase)
    DIM we1(7), we2(7)                  ' edge list: fuselage + 4 wings
    DIM oex(3), oey(3)                  ' engine glow dots, last frame
    DIM fx(3), fyA(3)                   ' bolt origins (wingtips at fire time)
    DIM obx1(3), oby1(3), obx2(3), oby2(3)   ' bolt lines, last frame

    SCREEN 13

    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 128 + .5)
    NEXT i

    ' X-wing seen from behind: diamond fuselage + 4 wings in an X
    spx(0) =   0: spy(0) =  -5          ' fuselage diamond
    spx(1) =   4: spy(1) =   0
    spx(2) =   0: spy(2) =   5
    spx(3) =  -4: spy(3) =   0
    spx(4) =  -3: spy(4) =  -3          ' wing roots (UL, UR, LL, LR)
    spx(5) =   3: spy(5) =  -3
    spx(6) =  -3: spy(6) =   3
    spx(7) =   3: spy(7) =   3
    spx(8) = -24: spy(8) = -13          ' wingtips (UL, UR, LL, LR)
    spx(9) =  24: spy(9) = -13
    spx(10) = -24: spy(10) = 13
    spx(11) =  24: spy(11) = 13

    we1(0) = 0: we2(0) = 1              ' fuselage diamond
    we1(1) = 1: we2(1) = 2
    we1(2) = 2: we2(2) = 3
    we1(3) = 3: we2(3) = 0
    we1(4) = 4: we2(4) = 8              ' wings root -> tip
    we1(5) = 5: we2(5) = 9
    we1(6) = 6: we2(6) = 10
    we1(7) = 7: we2(7) = 11

    ' Palette (faded in from black): 1 static seams, 2 rungs, 3 exhaust
    ' port glow, 4 X-wing hull, 6 engine glow, 7 laser red
    OUT &H3C8, 1
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 4: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 6: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 7: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0

    ' Rungs evenly spaced in depth; retire at z<72 (off-screen), respawn far
    FOR i = 0 TO 9
        rz(i) = 76 + i * 44
        olx(i) = 0: orx(i) = 0: ofy(i) = 0: oty(i) = 0
    NEXT i

    ' Ship starts centred; init previous points so first erase is harmless
    FOR i = 0 TO 11
        opx(i) = 160: opy(i) = 150
    NEXT i
    FOR k = 0 TO 3
        oex(k) = 160: oey(k) = 150
    NEXT k

    RANDOMIZE TIMER
    p1 = 0: p2 = 96
    boltU = -1: boltW = 90: boltDrawn = 0
    fadeV = 0

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8

        IF fadeV < 63 THEN
            fadeV = fadeV + 1
            v = 26 * fadeV \ 63
            OUT &H3C8, 1: OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, v + 2 * fadeV \ 63
            v = 46 * fadeV \ 63
            OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, 50 * fadeV \ 63
            OUT &H3C9, fadeV: OUT &H3C9, fadeV: OUT &H3C9, 56 * fadeV \ 63
            OUT &H3C8, 4
            OUT &H3C9, 40 * fadeV \ 63
            OUT &H3C9, 52 * fadeV \ 63
            OUT &H3C9, fadeV
            OUT &H3C8, 6
            OUT &H3C9, fadeV: OUT &H3C9, 30 * fadeV \ 63: OUT &H3C9, 0
            OUT &H3C8, 7
            OUT &H3C9, fadeV: OUT &H3C9, 8 * fadeV \ 63: OUT &H3C9, 8 * fadeV \ 63
        ELSE
            ' Engine glow flicker, palette-only (zero pixel cost)
            OUT &H3C8, 6
            OUT &H3C9, 40 + INT(RND * 24): OUT &H3C9, 20 + INT(RND * 16): OUT &H3C9, 0
        END IF

        ' -- Erase everything from last frame --
        IF boltDrawn THEN
            FOR k = 0 TO 3
                LINE (obx1(k), oby1(k))-(obx2(k), oby2(k)), 0
            NEXT k
            boltDrawn = 0
        END IF
        FOR e = 0 TO 7
            LINE (opx(we1(e)), opy(we1(e)))-(opx(we2(e)), opy(we2(e))), 0
        NEXT e
        FOR k = 0 TO 3
            PSET (oex(k), oey(k)), 0
        NEXT k
        FOR i = 0 TO 9
            LINE (olx(i), oty(i))-(olx(i), ofy(i)), 0
            LINE (orx(i), oty(i))-(orx(i), ofy(i)), 0
            LINE (olx(i), ofy(i))-(orx(i), ofy(i)), 0
        NEXT i

        ' -- Static trench seams (redrawn: erases nick them) + exhaust port --
        ' Endpoints are the z=60 projection; LINE clips off-screen parts
        LINE (160, 90)-(-53, 218), 1
        LINE (160, 90)-(373, 218), 1
        LINE (160, 90)-(-53, -59), 1
        LINE (160, 90)-(373, -59), 1
        PSET (160, 90), 3

        ' -- Rungs: advance depth, project, draw (3 clipped LINEs each) --
        FOR i = 0 TO 9
            rz(i) = rz(i) - 5
            IF rz(i) < 72 THEN rz(i) = rz(i) + 440
            xo = 12800 \ rz(i)              ' wall half-width 100, focal 128
            lx = 160 - xo: rx = 160 + xo
            fy = 90 + 7680 \ rz(i)          ' floor 60 below eye
            ty = 90 - 8960 \ rz(i)          ' wall tops 70 above eye
            LINE (lx, ty)-(lx, fy), 2
            LINE (rx, ty)-(rx, fy), 2
            LINE (lx, fy)-(rx, fy), 2
            olx(i) = lx: orx(i) = rx: ofy(i) = fy: oty(i) = ty
        NEXT i

        ' -- X-wing: weave low over the floor, bank into the turn --
        p1 = (p1 + 2) AND 255
        p2 = (p2 + 3) AND 255
        shipX = 160 + (sinT(p1) * 22) \ 128
        shipY = 150 + (sinT(p2) * 8) \ 128
        bk = (sinT((p1 + 64) AND 255) * 14) \ 128
        sinB = sinT(bk AND 255)
        cosB = sinT((bk + 64) AND 255)

        FOR i = 0 TO 11
            dpx(i) = shipX + (spx(i) * cosB - spy(i) * sinB) \ 128
            dpy(i) = shipY + (spx(i) * sinB + spy(i) * cosB) \ 128
        NEXT i

        FOR e = 0 TO 7
            LINE (dpx(we1(e)), dpy(we1(e)))-(dpx(we2(e)), dpy(we2(e))), 4
        NEXT e

        ' Engine glow dots, a third of the way out each wing
        FOR k = 0 TO 3
            ex = dpx(4 + k) + (dpx(8 + k) - dpx(4 + k)) \ 3
            ey = dpy(4 + k) + (dpy(8 + k) - dpy(4 + k)) \ 3
            PSET (ex, ey), 6
            oex(k) = ex: oey(k) = ey
        NEXT k

        FOR i = 0 TO 11
            opx(i) = dpx(i): opy(i) = dpy(i)
        NEXT i

        ' -- Laser bolts: 4 shots from the wingtips converge on the port --
        IF boltU < 0 THEN
            boltW = boltW - 1
            IF boltW <= 0 THEN
                boltU = 0
                FOR k = 0 TO 3
                    fx(k) = dpx(8 + k): fyA(k) = dpy(8 + k)
                NEXT k
            END IF
        ELSE
            FOR k = 0 TO 3
                obx1(k) = fx(k) + (160 - fx(k)) * boltU \ 18
                oby1(k) = fyA(k) + (90 - fyA(k)) * boltU \ 18
                obx2(k) = fx(k) + (160 - fx(k)) * (boltU + 3) \ 18
                oby2(k) = fyA(k) + (90 - fyA(k)) * (boltU + 3) \ 18
                LINE (obx1(k), oby1(k))-(obx2(k), oby2(k)), 7
            NEXT k
            boltDrawn = 1
            boltU = boltU + 1
            IF boltU > 15 THEN
                boltU = -1
                boltW = 70 + INT(RND * 80)
            END IF
        END IF

        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

    ' Fade out
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        pv = 26 * v \ 63
        OUT &H3C8, 1: OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, pv
        pv = 46 * v \ 63
        OUT &H3C9, pv: OUT &H3C9, pv: OUT &H3C9, 50 * v \ 63
        OUT &H3C9, v: OUT &H3C9, v: OUT &H3C9, 56 * v \ 63
        OUT &H3C8, 4: OUT &H3C9, 40 * v \ 63: OUT &H3C9, 52 * v \ 63: OUT &H3C9, v
        OUT &H3C8, 6: OUT &H3C9, v: OUT &H3C9, 30 * v \ 63: OUT &H3C9, 0
        OUT &H3C8, 7: OUT &H3C9, v: OUT &H3C9, 8 * v \ 63: OUT &H3C9, 8 * v \ 63
    NEXT v

    SCREEN 0
END SUB

' ======================================================
' SCENE 14 -- Rotozoomer
' A 64x64 checkerboard bitmap continuously rotates and
' zooms (breathes in/out).  True inverse-mapped texture
' sampling at a coarse 16x20 block grid (200 samples).
' Optimized for the P66 budget two ways:
'  1. Incremental stepping -- the map is affine, so the 4
'     source deltas are computed once per frame and each
'     block is just two adds + AND 8191 wrap (fixed point,
'     texture = 8192 units); \128 becomes a LUT read.
'     No multiplies, divides, or LONGs in the hot loop.
'  2. Dirty blocks -- prev() caches each block's colour;
'     LINE BF runs only when it changed (checkerboard is
'     mostly flat, so most blocks skip most frames).
' ======================================================
SUB Scene14
    DEFINT A-Z
    DIM sinT(255)              ' signed sine LUT * 127
    DIM texA(4095)             ' 64x64 checkerboard texture, palette indices
    DIM prev(199)              ' last drawn colour per block (0 = never drawn)
    DIM rxL AS LONG, ryL AS LONG
    lutN = 8191                ' fixed point: 64 texels x 128 subunits = 8192
    DIM txL(lutN)              ' u -> texel column (replaces \128 divide)
    DIM tyOff(lutN)            ' v -> texel row offset (replaces \128 + *64)

    SCREEN 13

    FOR i = 0 TO 255
        sinT(i) = INT(SIN(i * 6.28318 / 256) * 127)
    NEXT i

    ' Fixed-point -> texel lookup tables: one array read instead of an
    ' integer divide (and a multiply) per block in the hot loop
    FOR i = 0 TO 8191
        txL(i) = i \ 128
        tyOff(i) = txL(i) * 64
    NEXT i

    ' 4-colour checkerboard in 8x8 texel cells, tiled 8x8 times to fill 64x64
    FOR ty = 0 TO 63
        FOR tx = 0 TO 63
            texA(ty * 64 + tx) = (((tx \ 8) + (ty \ 8)) AND 3) + 1
        NEXT tx
    NEXT ty

    ' Palette: 4 checker colours, black until fade-in
    OUT &H3C8, 1: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 2: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 3: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    OUT &H3C8, 4: OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0

    ang = 0: zoomPh = 64
    fadeV = 0

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8

        IF fadeV < 63 THEN
            fadeV = fadeV + 1
            OUT &H3C8, 1: OUT &H3C9, 63 * fadeV \ 63: OUT &H3C9, 0: OUT &H3C9, 0
            OUT &H3C8, 2: OUT &H3C9, 63 * fadeV \ 63: OUT &H3C9, 30 * fadeV \ 63: OUT &H3C9, 0
            OUT &H3C8, 3: OUT &H3C9, 0: OUT &H3C9, 50 * fadeV \ 63: OUT &H3C9, 50 * fadeV \ 63
            OUT &H3C8, 4: OUT &H3C9, 35 * fadeV \ 63: OUT &H3C9, 0: OUT &H3C9, 63 * fadeV \ 63
        END IF

        ang = (ang + 2) AND 255
        zoomPh = (zoomPh + 1) AND 255
        zoomVal = 128 + (sinT(zoomPh) * 64) \ 127     ' breathing zoom: ~64..192

        cosA = sinT((ang + 64) AND 255)
        sinA = sinT(ang)
        cosA2 = (cosA * zoomVal) \ 128
        sinA2 = (sinA * zoomVal) \ 128

        ' The map is affine, so all multiplies happen ONCE per frame here:
        ' stepping a block right/down always adds the same source deltas.
        ' Coordinates live in wrap-free fixed point (texture = 8192 units),
        ' so AND 8191 replaces bounds handling, negatives included.
        dux = 16 * cosA2: duy = 16 * sinA2            ' block-right step
        dvx = -20 * sinA2: dvy = 20 * cosA2           ' block-down step

        ' Source coords of the top-left block's centre (dx=-152, dy=-90)
        rxL = CLng(-152) * cosA2 - CLng(-90) * sinA2
        ryL = CLng(-152) * sinA2 + CLng(-90) * cosA2
        rowU = (rxL + 4096) AND 8191
        rowV = (ryL + 4096) AND 8191

        blk = 0
        py = 0
        FOR by = 0 TO 9
            u = rowU: v = rowV
            px = 0
            FOR bx = 0 TO 19
                col = texA(tyOff(v) + txL(u))
                ' Dirty-block check: the checkerboard is mostly flat, so
                ' most blocks keep their colour -- skip the LINE BF fill
                IF col <> prev(blk) THEN
                    prev(blk) = col
                    LINE (px, py)-(px + 15, py + 19), col, BF
                END IF
                u = (u + dux) AND 8191
                v = (v + duy) AND 8191
                px = px + 16
                blk = blk + 1
            NEXT bx
            rowU = (rowU + dvx) AND 8191
            rowV = (rowV + dvy) AND 8191
            py = py + 20
        NEXT by

        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

    ' Fade out
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 1: OUT &H3C9, v: OUT &H3C9, 0: OUT &H3C9, 0
        OUT &H3C8, 2: OUT &H3C9, v: OUT &H3C9, 30 * v \ 63: OUT &H3C9, 0
        OUT &H3C8, 3: OUT &H3C9, 0: OUT &H3C9, 50 * v \ 63: OUT &H3C9, 50 * v \ 63
        OUT &H3C8, 4: OUT &H3C9, 35 * v \ 63: OUT &H3C9, 0: OUT &H3C9, 63 * v \ 63
    NEXT v

    SCREEN 0
END SUB

' ======================================================
' SCENE 15 -- Platformer vignette (mario homage)
' A 16x16 pixel-art runner (red cap, overalls) does a
' scripted lap: runs right, jumps onto two floating
' platforms, then a big leap over a patrolling mushroom
' man before wrapping around.  Sprite art lives in module
' DATA strings, rendered once at init into GET sprites
' with AND-masks for true transparency over the sky
' (same masking trick as the shadebobs).  Gravity in
' quarter-pixel fixed point.  Per frame: 2 LINE BF
' erases + 4 masked PUTs + a handful of physics ops.
' ======================================================
SUB Scene15
    DEFINT A-Z
    DIM rw$(15)                    ' one sprite's 16 rows while parsing
    DIM pR(11), pG(11), pB(11)     ' palette targets for fade
    sprN = 1040                    ' 4 sprites x (draw 130 + mask 130) ints
    DIM spr(sprN)                  ' variable bound -> dynamic array

    SCREEN 13

    ' Black out the palette so sprite prep + world draw stay invisible
    OUT &H3C8, 0
    FOR i = 0 TO 255
        OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    NEXT i

    ' Palette targets: 1 sky, 2 brick, 3 dark lines, 4 highlight, 5 white,
    ' 6 red, 7 skin, 8 brown, 9 overalls, 10 goomba body, 11 goomba feet
    pR(1) = 23: pG(1) = 37: pB(1) = 63
    pR(2) = 50: pG(2) = 19: pB(2) = 3
    pR(3) = 12: pG(3) = 5: pB(3) = 0
    pR(4) = 63: pG(4) = 42: pB(4) = 20
    pR(5) = 63: pG(5) = 63: pB(5) = 63
    pR(6) = 58: pG(6) = 0: pB(6) = 0
    pR(7) = 63: pG(7) = 40: pB(7) = 26
    pR(8) = 26: pG(8) = 11: pB(8) = 0
    pR(9) = 8: pG(9) = 16: pB(9) = 55
    pR(10) = 42: pG(10) = 22: pB(10) = 8
    pR(11) = 15: pG(11) = 6: pB(11) = 0

    ' Build the 4 sprites (+matching AND-masks) from module DATA via
    ' screen-corner GETs: frames 0/1 = run, 2 = jump, 3 = mushroom man
    RESTORE SpriteData
    FOR f = 0 TO 3
        FOR ry = 0 TO 15
            READ rw$(ry)
        NEXT ry
        ' draw sprite: opaque pixels on a black (0) box
        LINE (0, 0)-(15, 15), 0, BF
        FOR ry = 0 TO 15
            FOR rx = 1 TO 16
                c = -1
                SELECT CASE MID$(rw$(ry), rx, 1)
                    CASE "R": c = 6
                    CASE "S": c = 7
                    CASE "B": c = 8
                    CASE "O": c = 9
                    CASE "G": c = 10
                    CASE "D": c = 11
                    CASE "W": c = 5
                END SELECT
                IF c >= 0 THEN PSET (rx - 1, ry), c
            NEXT rx
        NEXT ry
        GET (0, 0)-(15, 15), spr(f * 260)
        ' mask: 255 where transparent (keeps background), 0 where opaque
        LINE (0, 0)-(15, 15), 255, BF
        FOR ry = 0 TO 15
            FOR rx = 1 TO 16
                IF MID$(rw$(ry), rx, 1) <> "." THEN PSET (rx - 1, ry), 0
            NEXT rx
        NEXT ry
        GET (0, 0)-(15, 15), spr(f * 260 + 130)
    NEXT f

    ' -- Draw the world (still invisible; also wipes the sprite corner) --
    LINE (0, 0)-(319, 199), 1, BF                 ' sky

    ' Two puffy clouds
    FOR k = 0 TO 1
        IF k = 0 THEN cx = 44: cy = 28 ELSE cx = 180: cy = 44
        LINE (cx + 6, cy)-(cx + 21, cy + 3), 5, BF
        LINE (cx + 2, cy + 4)-(cx + 27, cy + 9), 5, BF
        LINE (cx, cy + 6)-(cx + 29, cy + 11), 5, BF
    NEXT k

    ' Ground: brick fill with staggered joint lines (rows 176-199)
    LINE (0, 176)-(319, 199), 2, BF
    LINE (0, 176)-(319, 176), 4                   ' lit top edge
    FOR y = 183 TO 199 STEP 8
        LINE (0, y)-(319, y), 3
    NEXT y
    FOR x = 0 TO 319 STEP 16
        LINE (x, 177)-(x, 182), 3
        LINE (x, 192)-(x, 198), 3
        IF x + 8 <= 319 THEN LINE (x + 8, 184)-(x + 8, 191), 3
    NEXT x

    ' Floating platforms: A at y=146 (x 76-155), B at y=120 (x 160-239);
    ' 16x12 blocks with lit top and shaded right/bottom edges
    FOR k = 0 TO 8
        IF k < 5 THEN
            bx = 76 + k * 16: by = 146            ' platform A: 5 blocks
        ELSE
            bx = 160 + (k - 5) * 16: by = 120     ' platform B: 4 blocks
        END IF
        LINE (bx, by)-(bx + 15, by + 11), 2, BF
        LINE (bx, by)-(bx + 15, by), 4
        LINE (bx + 15, by)-(bx + 15, by + 11), 3
        LINE (bx, by + 11)-(bx + 15, by + 11), 3
    NEXT k

    CALL DrawText("MEGA WORLD 1-1", 104, 6, 1, 5)

    ' -- Actors --
    x = 0: feetQ = 176 * 4                        ' runner: quarter-px feet y
    grounded = 1: vyQ = 0
    runF = 0: animT = 0
    omx = 0: omy = 160
    gx = 236: gdir = 1: ogx = 236                 ' mushroom man patrol
    fadeV = 0

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8

        IF fadeV < 63 THEN
            fadeV = fadeV + 1
            OUT &H3C8, 1
            FOR i = 1 TO 11
                OUT &H3C9, pR(i) * fadeV \ 63
                OUT &H3C9, pG(i) * fadeV \ 63
                OUT &H3C9, pB(i) * fadeV \ 63
            NEXT i
        END IF

        ' -- Erase both actors (their boxes only ever cover sky) --
        LINE (omx, omy)-(omx + 15, omy + 15), 1, BF
        LINE (ogx, 160)-(ogx + 15, 175), 1, BF

        ' -- Mushroom man patrols the ground on the right --
        gx = gx + gdir
        IF gx >= 272 THEN gdir = -1
        IF gx <= 236 THEN gdir = 1

        ' -- Runner: constant speed right, wrap at the edge --
        x = x + 2
        IF x > 302 THEN x = 0
        cx = x + 8

        ' Surface height under the runner's centre
        s = 176
        IF cx >= 84 AND cx <= 148 THEN s = 146
        IF cx >= 168 AND cx <= 232 THEN s = 120

        IF grounded THEN
            IF s > feetQ \ 4 THEN
                grounded = 0: vyQ = 0             ' ran off an edge
            ELSE
                ' scripted jump windows: onto A, onto B, big leap over
                ' the mushroom man to the wrap point
                fy = feetQ \ 4
                IF fy = 176 AND cx >= 60 AND cx <= 72 THEN grounded = 0: vyQ = -18
                IF fy = 146 AND cx >= 140 AND cx <= 152 THEN grounded = 0: vyQ = -18
                IF fy = 120 AND cx >= 204 AND cx <= 216 THEN grounded = 0: vyQ = -22
            END IF
        END IF

        IF grounded = 0 THEN
            vyQ = vyQ + 1                         ' gravity, quarter px
            feetQ = feetQ + vyQ
            IF vyQ > 0 AND feetQ >= s * 4 THEN
                feetQ = s * 4: grounded = 1       ' touchdown
            END IF
        END IF

        ' Animation: alternate run frames on the ground, jump pose in air
        IF grounded = 0 THEN
            f = 2
        ELSE
            animT = animT + 1
            IF animT >= 6 THEN animT = 0: runF = 1 - runF
            f = runF
        END IF
        my = feetQ \ 4 - 16

        ' -- Draw: AND-mask carves the hole, OR stamps the colours --
        PUT (gx, 160), spr(3 * 260 + 130), AND
        PUT (gx, 160), spr(3 * 260), OR
        PUT (x, my), spr(f * 260 + 130), AND
        PUT (x, my), spr(f * 260), OR

        omx = x: omy = my: ogx = gx

        ft = ft + 1
    LOOP WHILE INKEY$ = "" AND ft < 600

    ' Fade out
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 1
        FOR i = 1 TO 11
            OUT &H3C9, pR(i) * v \ 63
            OUT &H3C9, pG(i) * v \ 63
            OUT &H3C9, pB(i) * v \ 63
        NEXT i
    NEXT v

    SCREEN 0
END SUB

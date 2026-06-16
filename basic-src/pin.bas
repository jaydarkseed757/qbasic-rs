' STEEL SLAM - QBasic 1.1 two-level pinball, classic DOS portrait layout.
' SCREEN 13: 320x200, 256 colors, 40x25 text.
' Left side = vertical playfield (PAGES between an UPPER and a LOWER screen);
' right side = scoreboard panel.
'
' Flow: launch the ball up the lane into the UPPER screen (bumpers, rollover
' lanes, drop targets, spinner, a pair of upper flippers). When it falls out the
' bottom it drops to the LOWER screen (slingshots + the main flippers). Draining
' on the LOWER screen loses the ball; you then shoot again into the UPPER.
'
' Controls: Z=left flipper  X=right flipper  S=nudge/bump  SPACE=plunger  ESC=quit
'   (3 nudges within the cooldown window = TILT: flippers go dead for that ball)
'
' QBasic 1.1 notes: no underscores; all DIM at top of proc/module;
'   COLOR fg only (SCREEN 13); LONG for score; save-under ball sprite;
'   LINE takes two points only; locals must not clash with CONST names.

' ============================================================
' MODULE-LEVEL DECLARATIONS
' ============================================================
DIM SHARED bx AS INTEGER
DIM SHARED by AS INTEGER
DIM SHARED obx AS INTEGER
DIM SHARED oby AS INTEGER
DIM SHARED bdx AS INTEGER
DIM SHARED bdy AS INTEGER
DIM SHARED inlane AS INTEGER       ' 1 = parked, 2 = ascending lane, 0 = in play
DIM SHARED plunge AS INTEGER
DIM SHARED lhold AS INTEGER
DIM SHARED rhold AS INTEGER
DIM SHARED olflip AS INTEGER
DIM SHARED orflip AS INTEGER
DIM SHARED gameover AS INTEGER
DIM SHARED kp AS STRING
DIM SHARED score AS LONG
DIM SHARED hiscore AS LONG
DIM SHARED balls AS INTEGER
DIM SHARED bonus AS LONG
DIM SHARED bonusx AS INTEGER
DIM SHARED oldscore AS LONG
DIM SHARED oldballs AS INTEGER
DIM SHARED oldbonus AS LONG

' Which playfield is showing: 0 = upper, 1 = lower
DIM SHARED level AS INTEGER
' Per-level flipper geometry (set by GoUpper / GoLower)
DIM SHARED lpx AS INTEGER
DIM SHARED lpy AS INTEGER
DIM SHARED rpx AS INTEGER
DIM SHARED rpy AS INTEGER
DIM SHARED fliplen AS INTEGER
' Bottom threshold: cross it to exit the upper / drain on the lower
DIM SHARED botY AS INTEGER

' Ball sprite + saved background under it
DIM SHARED ballspr(40) AS INTEGER
DIM SHARED bgspr(40) AS INTEGER

' Bumpers (up to 3; count + positions are set per level by GoUpper/GoLower)
DIM SHARED nbump AS INTEGER
DIM SHARED bmpx(2) AS INTEGER
DIM SHARED bmpy(2) AS INTEGER
DIM SHARED bmpr AS INTEGER
DIM SHARED bmpflash(2) AS INTEGER
DIM SHARED bmpcol(2) AS INTEGER

' Top rollover lanes (4, upper)
DIM SHARED rolx(3) AS INTEGER
DIM SHARED roly AS INTEGER
DIM SHARED rolit(3) AS INTEGER

' Drop targets (4, upper)
DIM SHARED tgtx(3) AS INTEGER
DIM SHARED tgty AS INTEGER
DIM SHARED tgtw AS INTEGER
DIM SHARED tgth AS INTEGER
DIM SHARED tgtup(3) AS INTEGER

' Spinner (upper)
DIM SHARED spinx AS INTEGER
DIM SHARED spiny1 AS INTEGER
DIM SHARED spiny2 AS INTEGER
DIM SHARED spinct AS INTEGER

' Slingshots (lower)
DIM SHARED slflashL AS INTEGER
DIM SHARED slflashR AS INTEGER

' Ball-save / nudge-tilt / anti-trap state
DIM SHARED ballsaver AS INTEGER
DIM SHARED savemsg AS INTEGER
DIM SHARED bumpcount AS INTEGER
DIM SHARED bumpcool AS INTEGER
DIM SHARED tilted AS INTEGER
DIM SHARED bmphits AS INTEGER
DIM SHARED bmpwin AS INTEGER

' ----- Geometry (shared by both levels) -----
CONST PFL = 12           ' play-area left wall
CONST PFR = 138          ' play-area right wall (also lane inner wall on upper)
CONST PFT = 22           ' play-area ceiling
CONST LANER = 156        ' plunger-lane outer wall (upper only)
CONST LANEGAP = 36       ' above this y the lane is open to the field
CONST TBT = 16           ' table top border
CONST TBB = 196          ' table bottom border
CONST TBR = 160          ' table right border (left of the scoreboard)
CONST DIVX = 166         ' scoreboard divider x
CONST BRAD = 3
CONST PLMAX = 20

' ----- Colors (SCREEN 13 palette indices) -----
CONST CBALL = 15
CONST CWALL = 10         ' bright green rails
CONST CWALL2 = 9         ' blue rail shading
CONST CRAIL = 8          ' dark-gray shading
CONST CFELT = 0          ' black playfield
CONST CFLIP = 13         ' magenta flippers
CONST CSLING = 11        ' cyan slingshots
CONST CSLINGF = 15
CONST CROL = 4           ' unlit rollover (red)
CONST CROLL = 14         ' lit rollover (yellow)
CONST CTGT = 10          ' green drop targets
CONST CSPIN = 11         ' cyan spinner
CONST CBKG = 0

' ============================================================
' MAIN
' ============================================================
SCREEN 13
COLOR CBALL

CALL LoadHigh
CALL NewGame
CALL TitleScreen
SCREEN 13
CALL RestoreGamePalette
CALL MakeBallSprite
CALL DrawPanel
CALL ResetBall            ' GoUpper + park the ball in the lane

obx = bx: oby = by
GET (bx - BRAD, by - BRAD)-(bx + BRAD, by + BRAD), bgspr
PUT (bx - BRAD, by - BRAD), ballspr, PSET

DO
    ' ---- Input ----
    kp = INKEY$
    IF (kp = "z" OR kp = "Z") AND tilted = 0 THEN lhold = 3
    IF (kp = "x" OR kp = "X") AND tilted = 0 THEN rhold = 3
    IF (kp = "s" OR kp = "S") AND inlane = 0 AND tilted = 0 THEN CALL BumpTable
    IF kp = CHR$(27) THEN gameover = 1
    IF kp = " " AND inlane = 1 THEN
        IF plunge < PLMAX THEN plunge = plunge + 2
        CALL DrawPlunger
        IF plunge >= PLMAX THEN CALL LaunchBall
    END IF
    IF kp = CHR$(13) AND inlane = 1 AND plunge > 0 THEN CALL LaunchBall

    ' ---- Restore background where the ball was ----
    PUT (obx - BRAD, oby - BRAD), bgspr, PSET

    ' ---- Physics ----
    IF inlane = 1 THEN
        bx = (LANER + PFR) \ 2 + 1
        by = 178 - (plunge \ 10)
    ELSEIF inlane = 2 THEN
        CALL LaneAscent
    ELSE
        bdy = bdy + 1
        IF bdy > 7 THEN bdy = 7
        bx = bx + bdx
        by = by + bdy
        CALL WallBounce
        CALL CheckBumpers
        IF level = 0 THEN
            CALL CheckRollovers
            CALL CheckTargets
            CALL CheckSpinner
        ELSE
            CALL CheckSlings
        END IF
        CALL CheckFlippers
        IF ballsaver > 0 THEN ballsaver = ballsaver - 1
        IF bumpcool > 0 THEN
            bumpcool = bumpcool - 1
            IF bumpcool = 0 THEN bumpcount = 0
        END IF
        IF bmpwin > 0 THEN
            bmpwin = bmpwin - 1
            IF bmpwin = 0 THEN bmphits = 0
        END IF
        ' ---- Bottom of the playfield ----
        IF by + BRAD > botY THEN
            IF level = 0 THEN
                ' fell out of the upper screen -> drop to the lower and arm the
                ' one-time ball-save now that the ball is in the danger zone
                CALL DropToLower
                ballsaver = 200
            ELSE
                IF ballsaver > 0 AND tilted = 0 THEN
                    ballsaver = 0
                    savemsg = 30
                    CALL DropToLower
                ELSE
                    score = score + bonus * bonusx * 10
                    bonus = 0
                    balls = balls - 1
                    IF balls <= 0 THEN
                        gameover = 1
                    ELSE
                        CALL ResetBall
                    END IF
                END IF
            END IF
        END IF
    END IF

    ' ---- Redraw dynamic elements ----
    CALL DrawFlipState
    CALL AnimBumpers
    IF level = 0 THEN
        CALL AnimSpinner
    ELSE
        CALL AnimSlings
    END IF
    CALL DrawScore
    IF savemsg > 0 THEN COLOR 14: LOCATE 20, 23: PRINT "BALL SAVED"

    ' ---- Ball: save background, stamp ball ----
    GET (bx - BRAD, by - BRAD)-(bx + BRAD, by + BRAD), bgspr
    PUT (bx - BRAD, by - BRAD), ballspr, PSET
    obx = bx: oby = by

    IF lhold > 0 THEN lhold = lhold - 1
    IF rhold > 0 THEN rhold = rhold - 1
    IF savemsg > 0 THEN
        savemsg = savemsg - 1
        IF savemsg = 0 THEN LOCATE 20, 23: PRINT "          "
    END IF

    CALL Pause
LOOP UNTIL gameover = 1

' ---- Game over ----
IF score > hiscore THEN
    hiscore = score
    CALL SaveHigh
END IF
COLOR 12: LOCATE 12, 4: PRINT "G A M E   O V E R"
COLOR 7: LOCATE 14, 7: PRINT "Press a key"
DO: kp = INKEY$: LOOP UNTIL kp <> ""
SCREEN 0: COLOR 7: CLS
END

' ─────────────────────────────────────────────
' 256 palette entries for TITLE.BIN (R,G,B 0-63)
' ─────────────────────────────────────────────
PaletteData:
DATA 63,63,63,63,63,62,63,63,61,63,63,40
DATA 63,58,17,63,55,45,63,56,0,62,62,49
DATA 61,42,3,41,50,44,1,53,56,49,37,9
DATA 47,35,9,45,35,13,43,32,9,49,27,1
DATA 42,31,8,40,30,8,18,41,23,33,34,34
DATA 39,29,8,34,28,16,12,33,35,2,29,41
DATA 57,22,5,54,20,0,53,20,1,46,21,3
DATA 42,18,27,46,17,1,40,15,0,35,26,8
DATA 35,17,0,36,14,0,33,25,9,32,23,9
DATA 29,22,12,28,21,10,26,19,11,26,19,8
DATA 24,18,10,23,17,10,21,16,11,21,16,8
DATA 10,22,29,0,26,38,0,23,35,0,21,32
DATA 0,18,30,20,14,10,17,14,12,0,14,31
DATA 1,15,27,63,9,63,61,8,61,58,7,56
DATA 56,12,12,54,6,57,46,5,25,33,12,4
DATA 30,10,5,25,10,3,18,13,9,24,7,4
DATA 19,6,2,16,12,10,15,11,9,14,10,10
DATA 11,11,15,12,9,10,11,8,9,10,7,10
DATA 6,7,9,14,5,2,9,5,8,8,6,8
DATA 8,5,9,7,6,9,6,5,8,6,4,9
DATA 6,4,7,0,12,27,1,11,8,0,12,6
DATA 1,11,6,0,9,17,1,10,8,2,9,7
DATA 0,10,6,0,12,5,0,12,5,1,11,5
DATA 0,11,5,0,10,5,0,10,4,0,9,4
DATA 0,9,4,1,7,14,1,7,7,1,6,14
DATA 1,6,8,0,8,4,0,7,4,1,6,4
DATA 0,6,3,3,4,14,2,4,14,1,5,13
DATA 1,4,14,1,4,6,0,4,4,0,5,3
DATA 0,4,4,17,1,11,12,1,9,11,2,9
DATA 11,1,10,7,2,13,7,3,4,8,1,9
DATA 6,1,8,4,3,13,4,3,9,3,2,12
DATA 3,2,9,4,1,10,5,3,7,4,3,6
DATA 3,2,6,3,2,5,4,0,6,2,2,14
DATA 2,1,12,2,1,12,2,1,12,2,1,12
DATA 2,1,12,2,1,12,2,1,12,2,1,11
DATA 2,1,11,2,1,11,2,1,11,2,2,7
DATA 2,2,7,2,1,8,2,1,6,2,1,9
DATA 2,1,6,2,1,8,2,1,6,2,0,4
DATA 1,2,6,1,1,11,1,1,10,1,1,10
DATA 1,1,10,1,1,10,1,1,7,1,1,10
DATA 1,1,9,1,1,9,1,1,9,1,1,9
DATA 1,1,9,1,1,8,1,1,8,1,1,7
DATA 1,1,8,1,1,8,1,1,7,1,1,7
DATA 1,1,7,1,1,7,1,1,6,1,1,5
DATA 1,0,5,0,3,6,0,2,5,0,3,3
DATA 0,2,3,1,1,5,1,1,5,0,2,3
DATA 0,1,3,1,1,7,1,1,7,1,1,5
DATA 0,1,2,1,0,7,1,0,6,1,0,6
DATA 1,0,6,1,0,6,1,0,5,1,0,5
DATA 1,0,5,1,0,5,1,0,5,1,0,5
DATA 1,0,4,1,0,4,0,0,3,0,0,1
DATA 42,0,0,18,0,0,14,0,10,13,0,10
DATA 13,0,9,13,0,9,12,0,9,12,0,8
DATA 11,0,8,10,0,7,9,0,7,8,0,6
DATA 7,0,7,7,0,7,7,0,6,7,0,6
DATA 6,0,5,5,0,5,4,0,5,3,0,4
DATA 2,0,4,1,0,4,2,0,3,1,0,4
DATA 1,0,4,1,0,4,1,0,3,1,0,2
DATA 0,0,4,0,0,4,0,0,3,0,0,3
DATA 0,0,3,0,0,3,0,0,1,0,0,3
DATA 0,0,2,0,0,2,0,0,2,0,0,2
DATA 0,0,2,0,0,2,0,0,1,0,0,1
DATA 0,0,1,0,0,1,0,0,1,0,0,1
DATA 0,0,1,0,0,0,0,0,0,0,0,0

' ============================================================
SUB NewGame
    score = 0
    balls = 3
    bonus = 0
    bonusx = 1
    oldscore = -1
    oldballs = -1
    oldbonus = -1
    lhold = 0: rhold = 0
    slflashL = 0: slflashR = 0
    ballsaver = 0: savemsg = 0
    bumpcount = 0: bumpcool = 0: tilted = 0
    bmphits = 0: bmpwin = 0
    gameover = 0

    ' Bumpers (count/positions are overridden per level by GoUpper/GoLower)
    nbump = 3
    bmpx(0) = 45: bmpy(0) = 80
    bmpx(1) = 95: bmpy(1) = 80
    bmpx(2) = 70: bmpy(2) = 108
    bmpr = 10
    bmpflash(0) = 0: bmpflash(1) = 0: bmpflash(2) = 0
    bmpcol(0) = 12: bmpcol(1) = 14: bmpcol(2) = 10

    ' Upper rollover lanes
    rolx(0) = 30: rolx(1) = 56: rolx(2) = 82: rolx(3) = 108
    roly = 30
    rolit(0) = 0: rolit(1) = 0: rolit(2) = 0: rolit(3) = 0

    ' Upper drop targets
    tgtx(0) = 36: tgtx(1) = 60: tgtx(2) = 84: tgtx(3) = 108
    tgty = 130: tgtw = 12: tgth = 6
    tgtup(0) = 1: tgtup(1) = 1: tgtup(2) = 1: tgtup(3) = 1

    ' Upper spinner
    spinx = 122: spiny1 = 46: spiny2 = 64: spinct = 0
END SUB

' ============================================================
' Configure + draw the UPPER playfield.
SUB GoUpper
    DIM i AS INTEGER
    level = 0
    fliplen = 25
    lpx = 40: lpy = 152
    rpx = 110: rpy = 152
    botY = 172

    ' reset the upper element states for a fresh visit
    FOR i = 0 TO 3
        rolit(i) = 0
        tgtup(i) = 1
    NEXT i
    ' upper bumper cluster (3)
    nbump = 3
    bmpx(0) = 45: bmpy(0) = 80
    bmpx(1) = 95: bmpy(1) = 80
    bmpx(2) = 70: bmpy(2) = 108
    bmpflash(0) = 0: bmpflash(1) = 0: bmpflash(2) = 0
    spinct = 0

    CALL DrawFrame
    ' ceiling spans play area + lane feed
    LINE (PFL, PFT)-(LANER, PFT), CWALL
    ' left wall, then funnel down to the left flipper
    LINE (PFL, PFT)-(PFL, 138), CWALL
    LINE (PFL, 138)-(lpx - 2, lpy - 2), CWALL
    ' right play wall (open above LANEGAP so a launch spills left) + funnel
    LINE (PFR, LANEGAP)-(PFR, 138), CWALL
    LINE (PFR, 138)-(rpx + 2, rpy - 2), CWALL
    ' plunger lane outer wall + floor
    LINE (LANER, PFT)-(LANER, TBB - 4), CWALL
    LINE (PFR, TBB - 4)-(LANER, TBB - 4), CWALL

    CALL DrawRollovers
    CALL DrawTargets
    CALL DrawBumpers
    CALL DrawSpinner
    CALL RestFlippers
    CALL DrawPlunger
    COLOR 11: LOCATE 21, 29: PRINT "UPPER"
END SUB

' ============================================================
' Configure + draw the LOWER playfield.
SUB GoLower
    level = 1
    fliplen = 34
    lpx = 30: lpy = 170
    rpx = 120: rpy = 170
    botY = 186

    ' lower bumper pair (different layout than the upper cluster)
    nbump = 3
    bmpx(0) = 38: bmpy(0) = 74
    bmpx(1) = 112: bmpy(1) = 74
    bmpx(2) = 75: bmpy(2) = 104
    bmpflash(0) = 0: bmpflash(1) = 0: bmpflash(2) = 0

    CALL DrawFrame
    ' full ceiling
    LINE (PFL, PFT)-(PFR, PFT), CWALL
    ' left wall + funnel to the left flipper
    LINE (PFL, PFT)-(PFL, 138), CWALL
    LINE (PFL, 138)-(lpx - 2, lpy - 2), CWALL
    ' right wall + funnel to the right flipper
    LINE (PFR, PFT)-(PFR, 138), CWALL
    LINE (PFR, 138)-(rpx + 2, rpy - 2), CWALL

    CALL DrawBumpers
    CALL DrawSlings
    CALL RestFlippers
    COLOR 12: LOCATE 21, 29: PRINT "LOWER"
END SUB

' ============================================================
' Drop the ball into the top of the LOWER screen.
SUB DropToLower
    CALL GoLower
    bx = 50
    by = PFT + BRAD + 2
    bdx = 1
    bdy = 2
    inlane = 0
END SUB

' ============================================================
SUB DrawFrame
    ' clear the play region (left of the scoreboard) and draw the table border
    LINE (0, 0)-(DIVX - 2, 199), CBKG, BF
    LINE (8, TBT)-(TBR, TBB), CWALL, B
    LINE (9, TBT + 1)-(TBR - 1, TBB - 1), CWALL2, B
END SUB

' ============================================================
SUB RestFlippers
    olflip = -9: orflip = -9
    lhold = 0: rhold = 0
    CALL DrawLeftFlipper(0, CFLIP)
    CALL DrawRightFlipper(0, CFLIP)
    olflip = 0: orflip = 0
END SUB

' ============================================================
SUB ResetBall
    CALL GoUpper
    bx = (LANER + PFR) \ 2 + 1
    by = 178
    bdx = 0
    bdy = 0
    inlane = 1
    plunge = 0
    tilted = 0
    bumpcount = 0
    bumpcool = 0
    bmphits = 0
    bmpwin = 0
    LOCATE 15, 25: PRINT "    "
    CALL DrawPlunger
END SUB

' ============================================================
SUB LaunchBall
    bdy = -(plunge \ 2) - 16
    IF bdy < -22 THEN bdy = -22
    bdx = 0
    inlane = 2
    plunge = 0
    CALL DrawPlunger
END SUB

' ============================================================
SUB MakeBallSprite
    DIM d AS INTEGER
    d = BRAD * 2
    CIRCLE (BRAD, BRAD), BRAD, CBALL
    PAINT (BRAD, BRAD), CBALL, CBALL
    PSET (BRAD - 1, BRAD - 1), 7
    GET (0, 0)-(d, d), ballspr
    LINE (0, 0)-(d, d), CBKG, BF
END SUB

' ============================================================
SUB DrawPanel
    ' clear the scoreboard region and draw the divider
    LINE (DIVX - 1, 0)-(319, 199), CBKG, BF
    LINE (DIVX, 0)-(DIVX, 199), CWALL2

    COLOR 13: LOCATE 2, 23: PRINT "S T E E L"
    COLOR 11: LOCATE 4, 24: PRINT "S L A M"
    COLOR 8: LOCATE 5, 24: PRINT "(c) 1989"

    COLOR 14: LOCATE 8, 22: PRINT "BONUS X"
    COLOR 11: LOCATE 10, 22: PRINT "BONUS"
    COLOR 10: LOCATE 13, 22: PRINT "SCORE"
    COLOR 9: LOCATE 17, 22: PRINT "BALL"
    COLOR 12: LOCATE 19, 22: PRINT "HIGH"
    COLOR 7: LOCATE 21, 22: PRINT "LEVEL"

    COLOR 7
    LOCATE 22, 22: PRINT "Z X =FLIP S=BUMP"
    LOCATE 23, 22: PRINT "SPACE=LAUNCH"
    LOCATE 24, 22: PRINT "ESC  =QUIT"
END SUB

' ============================================================
SUB DrawScore
    IF bonus <> oldbonus THEN
        COLOR 11: LOCATE 11, 30: PRINT bonus; "  ";
        COLOR 14: LOCATE 8, 32: PRINT bonusx; " ";
        oldbonus = bonus
    END IF
    IF score <> oldscore THEN
        COLOR 15: LOCATE 14, 24: PRINT score; "    ";
        oldscore = score
    END IF
    IF balls <> oldballs THEN
        COLOR 9: LOCATE 17, 30: PRINT balls; " ";
        COLOR 12: LOCATE 19, 27: PRINT hiscore; "  ";
        oldballs = balls
    END IF
    COLOR CBALL
END SUB

' ============================================================
SUB DrawPlunger
    LINE (LANER - 10, 184)-(LANER - 2, 192), CFELT, BF
    IF plunge > 0 THEN
        LINE (LANER - 10, 192 - plunge \ 3)-(LANER - 2, 192), 10, BF
    END IF
END SUB

' ============================================================
' Draw one bumper (flashing = 1 -> white pop; else magenta band + yellow cap).
' Clears the cap to felt first so the PAINT seeds are deterministic.
SUB DrawOneBumper (i AS INTEGER, flashing AS INTEGER)
    LINE (bmpx(i) - bmpr - 1, bmpy(i) - bmpr - 1)-(bmpx(i) + bmpr + 1, bmpy(i) + bmpr + 1), CFELT, BF
    IF flashing = 1 THEN
        CIRCLE (bmpx(i), bmpy(i)), bmpr, 13
        PAINT (bmpx(i), bmpy(i)), 15, 13
    ELSE
        CIRCLE (bmpx(i), bmpy(i)), bmpr, 15
        PAINT (bmpx(i), bmpy(i)), 13, 15
        CIRCLE (bmpx(i), bmpy(i)), bmpr - 4, 14
        PAINT (bmpx(i), bmpy(i)), 14, 14
        PSET (bmpx(i) - 3, bmpy(i) - 3), 15
    END IF
END SUB

' Full draw of every bumper (used once on a page setup).
SUB DrawBumpers
    DIM i AS INTEGER
    DIM f AS INTEGER
    FOR i = 0 TO nbump - 1
        IF bmpflash(i) > 0 THEN f = 1 ELSE f = 0
        CALL DrawOneBumper(i, f)
    NEXT i
END SUB

' Per-frame: only redraw bumpers that are actively flashing (the save-under
' sprite keeps idle bumpers intact, so we skip their expensive PAINT fills).
SUB AnimBumpers
    DIM i AS INTEGER
    FOR i = 0 TO nbump - 1
        IF bmpflash(i) > 0 THEN
            bmpflash(i) = bmpflash(i) - 1
            IF bmpflash(i) > 0 THEN
                CALL DrawOneBumper(i, 1)
            ELSE
                CALL DrawOneBumper(i, 0)
            END IF
        END IF
    NEXT i
END SUB

' ============================================================
SUB DrawRollovers
    DIM i AS INTEGER
    DIM c AS INTEGER
    FOR i = 0 TO 3
        IF rolit(i) = 1 THEN c = CROLL ELSE c = CROL
        CIRCLE (rolx(i), roly), 4, CWALL
        PAINT (rolx(i), roly), c, CWALL
        LINE (rolx(i) - 3, roly - 8)-(rolx(i), roly - 5), CWALL
        LINE (rolx(i), roly - 5)-(rolx(i) + 3, roly - 8), CWALL
    NEXT i
END SUB

' ============================================================
SUB DrawTargets
    DIM i AS INTEGER
    FOR i = 0 TO 3
        IF tgtup(i) = 1 THEN
            LINE (tgtx(i), tgty)-(tgtx(i) + tgtw, tgty + tgth), CTGT, BF
            LINE (tgtx(i), tgty)-(tgtx(i) + tgtw, tgty), 15
        ELSE
            LINE (tgtx(i), tgty)-(tgtx(i) + tgtw, tgty + tgth), CFELT, BF
        END IF
    NEXT i
END SUB

' ============================================================
SUB DrawSpinner
    DIM spmid AS INTEGER
    spmid = (spiny1 + spiny2) \ 2
    LINE (spinx - 6, spiny1 - 2)-(spinx + 6, spiny2 + 2), CFELT, BF
    CIRCLE (spinx, spiny1 - 2), 2, CRAIL
    CIRCLE (spinx, spiny2 + 2), 2, CRAIL
    IF spinct > 0 THEN
        IF (spinct AND 1) = 1 THEN
            LINE (spinx - 5, spmid)-(spinx + 5, spmid), 15
        ELSE
            LINE (spinx - 1, spiny1)-(spinx + 1, spiny2), 15, BF
        END IF
    ELSE
        LINE (spinx - 1, spiny1)-(spinx + 1, spiny2), CSPIN, BF
    END IF
END SUB

' Per-frame: only redraw the spinner while it is actually spinning.
SUB AnimSpinner
    IF spinct > 0 THEN
        spinct = spinct - 1
        CALL DrawSpinner
    END IF
END SUB

' ============================================================
SUB DrawSlings
    DIM c AS INTEGER
    IF slflashL > 0 THEN c = CSLINGF ELSE c = CSLING
    LINE (PFL, 138)-(PFL, 160), c
    LINE (PFL, 138)-(lpx + 4, 156), c
    LINE (lpx + 4, 156)-(PFL, 160), c
    PAINT (PFL + 4, 152), c, c
    IF slflashR > 0 THEN c = CSLINGF ELSE c = CSLING
    LINE (PFR, 138)-(PFR, 160), c
    LINE (PFR, 138)-(rpx - 4, 156), c
    LINE (rpx - 4, 156)-(PFR, 160), c
    PAINT (PFR - 4, 152), c, c
END SUB

' Per-frame: only redraw the slingshots when one is flashing from a hit.
SUB AnimSlings
    DIM sl AS INTEGER
    sl = 0
    IF slflashL > 0 THEN slflashL = slflashL - 1: sl = 1
    IF slflashR > 0 THEN slflashR = slflashR - 1: sl = 1
    IF sl = 1 THEN CALL DrawSlings
END SUB

' ============================================================
SUB DrawFlipState
    IF olflip <> SGN(lhold) THEN
        CALL DrawLeftFlipper(olflip, CFELT)
        CALL DrawLeftFlipper(SGN(lhold), CFLIP)
        olflip = SGN(lhold)
        IF olflip = 1 THEN CALL Blip("T255L32O2E")
    END IF
    IF orflip <> SGN(rhold) THEN
        CALL DrawRightFlipper(orflip, CFELT)
        CALL DrawRightFlipper(SGN(rhold), CFLIP)
        orflip = SGN(rhold)
        IF orflip = 1 THEN CALL Blip("T255L32O2E")
    END IF
END SUB

' ============================================================
SUB DrawLeftFlipper (st AS INTEGER, col AS INTEGER)
    DIM ty AS INTEGER
    IF st = 1 THEN ty = lpy - 12 ELSE ty = lpy + 10
    LINE (lpx, lpy)-(lpx + fliplen, ty), col
    LINE (lpx, lpy + 1)-(lpx + fliplen, ty + 1), col
    LINE (lpx, lpy - 1)-(lpx + fliplen, ty - 1), col
END SUB

' ============================================================
SUB DrawRightFlipper (st AS INTEGER, col AS INTEGER)
    DIM ty AS INTEGER
    IF st = 1 THEN ty = rpy - 12 ELSE ty = rpy + 10
    LINE (rpx, rpy)-(rpx - fliplen, ty), col
    LINE (rpx, rpy + 1)-(rpx - fliplen, ty + 1), col
    LINE (rpx, rpy - 1)-(rpx - fliplen, ty - 1), col
END SUB

' ============================================================
SUB LaneAscent
    bdy = bdy + 1
    IF bdy > 7 THEN bdy = 7
    by = by + bdy
    IF bx - BRAD < PFR + 1 THEN bx = PFR + 1 + BRAD
    IF bx + BRAD > LANER - 1 THEN bx = LANER - 1 - BRAD
    IF by <= LANEGAP THEN
        inlane = 0
        by = LANEGAP
        bx = PFR - BRAD - 2
        bdx = -3
        IF bdy > -2 THEN bdy = -2
    END IF
    IF by + BRAD > TBB - 6 THEN CALL ResetBall
END SUB

' ============================================================
SUB WallBounce
    IF bx - BRAD < PFL THEN bx = PFL + BRAD: bdx = -bdx
    IF level = 0 THEN
        IF by >= LANEGAP AND bx + BRAD > PFR THEN bx = PFR - BRAD: bdx = -bdx
        IF bx + BRAD > LANER THEN bx = LANER - BRAD: bdx = -bdx
    ELSE
        IF bx + BRAD > PFR THEN bx = PFR - BRAD: bdx = -bdx
    END IF
    IF by - BRAD < PFT THEN by = PFT + BRAD: bdy = -bdy
END SUB

' ============================================================
SUB CheckBumpers
    DIM i AS INTEGER
    DIM dx AS LONG
    DIM dy AS LONG
    DIM dist AS INTEGER
    DIM minD AS INTEGER
    minD = BRAD + bmpr
    FOR i = 0 TO nbump - 1
        dx = bx - bmpx(i)
        dy = by - bmpy(i)
        ' cheap bounding-box reject before the square root
        IF ABS(dx) < minD AND ABS(dy) < minD THEN
            dist = INT(SQR(dx * dx + dy * dy))
            IF dist < minD THEN
            IF dist < 1 THEN dist = 1
            bx = bmpx(i) + INT(dx * minD / dist)
            by = bmpy(i) + INT(dy * minD / dist)
            IF bmphits >= 14 THEN
                bdx = INT(dx * 3 / dist)
                bdy = 4
            ELSE
                IF ABS(dx) >= ABS(dy) THEN
                    IF dx >= 0 THEN bdx = ABS(bdx) + 2 ELSE bdx = -ABS(bdx) - 2
                ELSE
                    IF dy >= 0 THEN bdy = ABS(bdy) + 2 ELSE bdy = -ABS(bdy) - 2
                END IF
                IF bdx > 7 THEN bdx = 7
                IF bdx < -7 THEN bdx = -7
                IF bdy > 7 THEN bdy = 7
                IF bdy < -7 THEN bdy = -7
            END IF
            bmpflash(i) = 3
            score = score + 100
            bonus = bonus + 1
            bmphits = bmphits + 1
            bmpwin = 40
            CALL Blip("T255L64O5C")
            END IF
        END IF
    NEXT i
END SUB

' ============================================================
SUB CheckRollovers
    DIM i AS INTEGER
    DIM alllit AS INTEGER
    DIM chg AS INTEGER
    chg = 0
    FOR i = 0 TO 3
        IF rolit(i) = 0 THEN
            IF ABS(bx - rolx(i)) <= 5 AND ABS(by - roly) <= 5 THEN
                rolit(i) = 1
                score = score + 500
                chg = 1
            END IF
        END IF
    NEXT i
    alllit = 1
    FOR i = 0 TO 3
        IF rolit(i) = 0 THEN alllit = 0
    NEXT i
    IF alllit = 1 THEN
        bonusx = bonusx + 1
        score = score + 5000
        FOR i = 0 TO 3
            rolit(i) = 0
        NEXT i
        chg = 1
    END IF
    IF chg = 1 THEN CALL DrawRollovers
END SUB

' ============================================================
SUB CheckTargets
    DIM i AS INTEGER
    DIM alldown AS INTEGER
    FOR i = 0 TO 3
        IF tgtup(i) = 1 THEN
            IF bx + BRAD >= tgtx(i) AND bx - BRAD <= tgtx(i) + tgtw THEN
                IF by + BRAD >= tgty AND by - BRAD <= tgty + tgth THEN
                    tgtup(i) = 0
                    bdy = -ABS(bdy) - 1
                    score = score + 250
                    bonus = bonus + 1
                    LINE (tgtx(i), tgty)-(tgtx(i) + tgtw, tgty + tgth), CFELT, BF
                END IF
            END IF
        END IF
    NEXT i
    alldown = 1
    FOR i = 0 TO 3
        IF tgtup(i) = 1 THEN alldown = 0
    NEXT i
    IF alldown = 1 THEN
        score = score + 2000
        bonusx = bonusx + 1
        FOR i = 0 TO 3
            tgtup(i) = 1
        NEXT i
        CALL DrawTargets
    END IF
END SUB

' ============================================================
SUB CheckSpinner
    IF ABS(bx - spinx) <= 6 AND by >= spiny1 AND by <= spiny2 THEN
        score = score + 50
        spinct = 6
    END IF
END SUB

' ============================================================
SUB CheckSlings
    IF bx <= lpx + 6 AND bx >= PFL AND by >= 140 AND by <= 160 THEN
        IF bmphits >= 14 THEN
            bdx = 2: bdy = 3
        ELSE
            bdx = ABS(bdx) + 3
            bdy = -ABS(bdy) - 2
            IF bdx > 8 THEN bdx = 8
            IF bdy < -8 THEN bdy = -8
        END IF
        bx = lpx + 8
        score = score + 50
        slflashL = 3
        bmphits = bmphits + 1
        bmpwin = 40
        CALL Blip("T255L64O4G")
    END IF
    IF bx >= rpx - 6 AND bx <= PFR AND by >= 140 AND by <= 160 THEN
        IF bmphits >= 14 THEN
            bdx = -2: bdy = 3
        ELSE
            bdx = -ABS(bdx) - 3
            bdy = -ABS(bdy) - 2
            IF bdx < -8 THEN bdx = -8
            IF bdy < -8 THEN bdy = -8
        END IF
        bx = rpx - 8
        score = score + 50
        slflashR = 3
        bmphits = bmphits + 1
        bmpwin = 40
        CALL Blip("T255L64O4G")
    END IF
END SUB

' ============================================================
SUB CheckFlippers
    DIM ltipy AS INTEGER
    DIM rtipy AS INTEGER
    DIM lyhit AS INTEGER
    DIM ryhit AS INTEGER
    DIM lst AS INTEGER
    DIM rst AS INTEGER

    IF tilted = 1 THEN EXIT SUB
    lst = SGN(lhold)
    rst = SGN(rhold)
    IF lst = 1 THEN ltipy = lpy - 12 ELSE ltipy = lpy + 10
    IF rst = 1 THEN rtipy = rpy - 12 ELSE rtipy = rpy + 10

    IF bx + BRAD >= lpx AND bx - BRAD <= lpx + fliplen THEN
        lyhit = lpy + INT((ltipy - lpy) * (bx - lpx) / fliplen)
        IF by + BRAD >= lyhit - 2 AND by + BRAD <= lyhit + 7 AND bdy >= 0 THEN
            by = lyhit - BRAD - 1
            bdy = -ABS(bdy) - 2
            IF lst = 1 THEN bdy = bdy - 3: bdx = bdx + 2
            IF bdy < -10 THEN bdy = -10
            IF bdx > 8 THEN bdx = 8
        END IF
    END IF

    IF bx - BRAD <= rpx AND bx + BRAD >= rpx - fliplen THEN
        ryhit = rpy + INT((rtipy - rpy) * (rpx - bx) / fliplen)
        IF by + BRAD >= ryhit - 2 AND by + BRAD <= ryhit + 7 AND bdy >= 0 THEN
            by = ryhit - BRAD - 1
            bdy = -ABS(bdy) - 2
            IF rst = 1 THEN bdy = bdy - 3: bdx = bdx - 2
            IF bdy < -10 THEN bdy = -10
            IF bdx < -8 THEN bdx = -8
        END IF
    END IF
END SUB

' ============================================================
SUB BumpTable
    bdy = bdy - 2
    IF bx < 75 THEN bdx = bdx + 2 ELSE bdx = bdx - 2
    IF bdy < -8 THEN bdy = -8
    IF bdx > 8 THEN bdx = 8
    IF bdx < -8 THEN bdx = -8
    bumpcount = bumpcount + 1
    bumpcool = 90
    IF bumpcount >= 3 THEN
        tilted = 1
        lhold = 0: rhold = 0
        COLOR 12: LOCATE 15, 25: PRINT "TILT"
    END IF
END SUB

' ============================================================
SUB LoadHigh
    DIM fileN AS INTEGER
    hiscore = 0
    fileN = FREEFILE
    OPEN "PINHI.DAT" FOR APPEND AS fileN
    CLOSE fileN
    fileN = FREEFILE
    OPEN "PINHI.DAT" FOR INPUT AS fileN
    IF NOT EOF(fileN) THEN INPUT #fileN, hiscore
    CLOSE fileN
END SUB

' ============================================================
SUB SaveHigh
    DIM fileN AS INTEGER
    fileN = FREEFILE
    OPEN "PINHI.DAT" FOR OUTPUT AS fileN
    PRINT #fileN, hiscore
    CLOSE fileN
END SUB

' ============================================================
SUB Pause
    DIM i AS INTEGER
    FOR i = 1 TO 500
    NEXT i
END SUB

' ============================================================
' Non-blocking sound: "MB" = music-background, so the note is queued to the
' speaker interrupt and PLAY returns immediately (a blocking SOUND would stall
' the game loop). Keep the notes very short (L64) so they read as blips.
SUB Blip (seq AS STRING)
    PLAY "MB" + seq
END SUB

' ============================================================
SUB TitleScreen
    DIM k AS STRING
    IF DIR$("TITLE.BIN") <> "" THEN
        CALL LoadPalette
        DEF SEG = &HA000
        BLOAD "TITLE.BIN", 0
        DEF SEG
    ELSE
        LINE (0, 0)-(319, 199), CBKG, BF
        COLOR 12: LOCATE 10, 14: PRINT "STEEL  SLAM"
        COLOR 14: LOCATE 12, 15: PRINT "TWO LEVELS"
    END IF
    COLOR 15: LOCATE 21, 11: PRINT "PRESS SPACE TO PLAY"
    COLOR 7: LOCATE 23, 13: PRINT "Z X = FLIPPERS"
    PLAY "T180 O3 L8 C E G O4 C E G L4 C L8 E L2 G"
    DO: k = INKEY$: LOOP UNTIL k <> ""
END SUB

SUB RestoreGamePalette
    ' Reset VGA DAC indices 0-15 to standard CGA/VGA defaults (0-63 scale)
    OUT &H3C8, 0
    OUT &H3C9, 0:  OUT &H3C9, 0:  OUT &H3C9, 0   ' 0  black
    OUT &H3C9, 0:  OUT &H3C9, 0:  OUT &H3C9, 42  ' 1  dark blue
    OUT &H3C9, 0:  OUT &H3C9, 42: OUT &H3C9, 0   ' 2  dark green
    OUT &H3C9, 0:  OUT &H3C9, 42: OUT &H3C9, 42  ' 3  dark cyan
    OUT &H3C9, 42: OUT &H3C9, 0:  OUT &H3C9, 0   ' 4  dark red
    OUT &H3C9, 42: OUT &H3C9, 0:  OUT &H3C9, 42  ' 5  dark magenta
    OUT &H3C9, 42: OUT &H3C9, 21: OUT &H3C9, 0   ' 6  brown
    OUT &H3C9, 42: OUT &H3C9, 42: OUT &H3C9, 42  ' 7  light gray
    OUT &H3C9, 21: OUT &H3C9, 21: OUT &H3C9, 21  ' 8  dark gray
    OUT &H3C9, 21: OUT &H3C9, 21: OUT &H3C9, 63  ' 9  bright blue
    OUT &H3C9, 21: OUT &H3C9, 63: OUT &H3C9, 21  ' 10 bright green
    OUT &H3C9, 21: OUT &H3C9, 63: OUT &H3C9, 63  ' 11 bright cyan
    OUT &H3C9, 63: OUT &H3C9, 21: OUT &H3C9, 21  ' 12 bright red
    OUT &H3C9, 63: OUT &H3C9, 21: OUT &H3C9, 63  ' 13 bright magenta
    OUT &H3C9, 63: OUT &H3C9, 63: OUT &H3C9, 21  ' 14 bright yellow
    OUT &H3C9, 63: OUT &H3C9, 63: OUT &H3C9, 63  ' 15 bright white
END SUB

SUB LoadPalette
    DIM i AS INTEGER, r AS INTEGER, g AS INTEGER, b AS INTEGER
    OUT &H3C8, 0
    RESTORE PaletteData
    FOR i = 0 TO 255
        READ r, g, b
        OUT &H3C9, r
        OUT &H3C9, g
        OUT &H3C9, b
    NEXT i
END SUB

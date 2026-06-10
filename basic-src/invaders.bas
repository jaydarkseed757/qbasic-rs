' ════════════════════════════════════════════════════════════════════
' SPACE INVADERS  —  QBasic 4.5 Port  (c) 1993 QB Port
' Single-file, SCREEN 13 (320x200 256-color), no external libs.
' ════════════════════════════════════════════════════════════════════

' ────────────────────────────────────────────────────────────────────
' 1. CONSTANTS
' ────────────────────────────────────────────────────────────────────
CONST SCR_W = 320
CONST SCR_H = 200
CONST PLAY_TOP = 16
CONST PLAY_BOT = 184

' Colors (EGA palette indices 0-15)
CONST BLACK = 0
CONST DKBLUE = 1
CONST DKGREEN = 2
CONST DKCYAN = 3
CONST DKRED = 4
CONST DKMAG = 5
CONST BROWN = 6
CONST LTGRAY = 7
CONST DKGRAY = 8
CONST BRTBLUE = 9
CONST BRTGREEN = 10
CONST BRTCYAN = 11
CONST BRTRED = 12
CONST BRTMAG = 13
CONST YELLOW = 14
CONST WHITE = 15

' Game states
CONST STATE_TITLE = 0
CONST STATE_PLAYING = 1
CONST STATE_HISCORES = 2
CONST STATE_GAMEOVER = 3
CONST STATE_INITIALS = 4
CONST STATE_QUIT = 5

' Formation constants
CONST INV_COLS = 11
CONST INV_ROWS = 5
CONST INV_SPACEX = 16
CONST INV_SPACEY = 14
CONST INV_STARTX = 16
CONST INV_STARTY = 32

' Sprite dimensions
CONST SHIP_W = 13
CONST SHIP_H = 8
CONST INVA_W = 11
CONST INVA_H = 8
CONST INVB_W = 11
CONST INVB_H = 8
CONST INVC_W = 12
CONST INVC_H = 8
CONST UFO_W = 16
CONST UFO_H = 7
CONST EXPL_W = 13
CONST EXPL_H = 8
CONST PBUL_W = 2
CONST PBUL_H = 6
CONST IBUL_W = 3
CONST IBUL_H = 7

CONST BUNKER_W = 22
CONST BUNKER_H = 16
CONST NUM_BUNKERS = 4

CONST MAX_IBULLETS = 3
CONST MAX_EXPLS = 6

CONST TICK_DELAY = .016

' ────────────────────────────────────────────────────────────────────
' 2. TYPE DEFINITIONS
' ────────────────────────────────────────────────────────────────────
TYPE ScoreEntry
    playerName AS STRING * 3
    score AS LONG
    level AS INTEGER
END TYPE

TYPE BulletType
    x AS INTEGER
    y AS INTEGER
    active AS INTEGER
    btype AS INTEGER
END TYPE

TYPE ExplType
    x AS INTEGER
    y AS INTEGER
    frame AS INTEGER
    timer AS INTEGER
    active AS INTEGER
END TYPE

' ────────────────────────────────────────────────────────────────────
' 3. DIM SHARED (global game state)
' ────────────────────────────────────────────────────────────────────
DIM SHARED gameState AS INTEGER
DIM SHARED score AS LONG
DIM SHARED hiScore AS LONG
DIM SHARED lives AS INTEGER
DIM SHARED level AS INTEGER
DIM SHARED aliveCount AS INTEGER

' Formation
DIM SHARED alive(1 TO INV_COLS, 1 TO INV_ROWS) AS INTEGER
DIM SHARED frmX AS INTEGER   ' formation top-left X
DIM SHARED frmY AS INTEGER   ' formation top-left Y
DIM SHARED frmDX AS INTEGER  ' direction: +1 right, -1 left
DIM SHARED frmStep AS INTEGER
DIM SHARED invFrame AS INTEGER  ' 0 or 1 animation frame
DIM SHARED marchNote AS INTEGER ' 0-3 for march cycle

' Player
DIM SHARED shipX AS INTEGER
DIM SHARED shipY AS INTEGER
DIM SHARED shipOldX AS INTEGER

' Player bullet
DIM SHARED pbulX AS INTEGER
DIM SHARED pbulY AS INTEGER
DIM SHARED pbulActive AS INTEGER

' Invader bullets
DIM SHARED ibul(1 TO MAX_IBULLETS) AS BulletType

' UFO
DIM SHARED ufoX AS INTEGER
DIM SHARED ufoY AS INTEGER
DIM SHARED ufoActive AS INTEGER
DIM SHARED ufoDX AS INTEGER
DIM SHARED ufoTimer AS INTEGER
DIM SHARED ufoScore AS INTEGER

' Explosions
DIM SHARED expl(1 TO MAX_EXPLS) AS ExplType

' Bunkers — pixel arrays 22x16
DIM SHARED bnk(1 TO NUM_BUNKERS, 0 TO BUNKER_W - 1, 0 TO BUNKER_H - 1) _
    AS INTEGER
DIM SHARED bnkX(1 TO NUM_BUNKERS) AS INTEGER
DIM SHARED bnkY AS INTEGER

' Sprite data arrays
DIM SHARED shipSprite(0 TO SHIP_W - 1, 0 TO SHIP_H - 1) AS INTEGER
DIM SHARED invASprite(0 TO 1, 0 TO INVA_W - 1, 0 TO INVA_H - 1) AS INTEGER
DIM SHARED invBSprite(0 TO 1, 0 TO INVB_W - 1, 0 TO INVB_H - 1) AS INTEGER
DIM SHARED invCSprite(0 TO 1, 0 TO INVC_W - 1, 0 TO INVC_H - 1) AS INTEGER
DIM SHARED ufoSprite(0 TO UFO_W - 1, 0 TO UFO_H - 1) AS INTEGER
DIM SHARED explSprite(0 TO 2, 0 TO EXPL_W - 1, 0 TO EXPL_H - 1) AS INTEGER
DIM SHARED pbulSprite(0 TO PBUL_W - 1, 0 TO PBUL_H - 1) AS INTEGER
DIM SHARED ibulSprite(0 TO 2, 0 TO IBUL_W - 1, 0 TO IBUL_H - 1) AS INTEGER

' Timing
DIM SHARED lastTime AS DOUBLE
DIM SHARED tickCount AS LONG
DIM SHARED invFireTick AS INTEGER
DIM SHARED ibulTypeIdx AS INTEGER

' High scores
DIM SHARED scores(1 TO 10) AS ScoreEntry
DIM SHARED newScoreRank AS INTEGER

' Stars (title + hiscore bg)
DIM SHARED starX(1 TO 80) AS INTEGER
DIM SHARED starY(1 TO 80) AS INTEGER
DIM SHARED starC(1 TO 80) AS INTEGER

' Misc
DIM SHARED flashToggle AS INTEGER
DIM SHARED initials AS STRING

' ────────────────────────────────────────────────────────────────────
' 4. SPRITE DATA BLOCKS
' ────────────────────────────────────────────────────────────────────

' === SPRITE: PLAYER SHIP (13x8) ===
' Bright green body (10), dark green edges (2), cyan cockpit (11)
DATA 0, 0, 0, 0, 0, 0,10, 0, 0, 0, 0, 0, 0
DATA 0, 0, 0, 0, 0,10,10,10, 0, 0, 0, 0, 0
DATA 0, 0, 0, 0,10,10,11,10,10, 0, 0, 0, 0
DATA 0, 0, 0,10,10,10,10,10,10,10, 0, 0, 0
DATA 0, 0,10,10,10,10,10,10,10,10,10, 0, 0
DATA 2, 0,10,10,10,10,10,10,10,10,10, 0, 2
DATA 2, 2, 2,10,10,10,10,10,10,10, 2, 2, 2
DATA 2, 2, 2, 2, 0, 0,10, 0, 0, 2, 2, 2, 2

' === SPRITE: INVADER A "Squid" FRAME 0 (11x8) ===
DATA  0, 0,11, 0, 0, 0, 0, 0,11, 0, 0
DATA  0, 0, 0,11, 0, 0, 0,11, 0, 0, 0
DATA  0, 0,11,11,11,11,11,11,11, 0, 0
DATA  0,11,11, 3,11,11,11, 3,11,11, 0
DATA 11,11,11,11,11,11,11,11,11,11,11
DATA 11, 3,11,15,15,11,15,15,11, 3,11
DATA 11, 0, 0,11, 0, 0, 0,11, 0, 0,11
DATA  0, 0,11, 0, 0, 0, 0, 0,11, 0, 0

' === SPRITE: INVADER A "Squid" FRAME 1 (11x8) ===
DATA  0, 0,11, 0, 0, 0, 0, 0,11, 0, 0
DATA  0, 0, 0,11, 0, 0, 0,11, 0, 0, 0
DATA  0, 0,11,11,11,11,11,11,11, 0, 0
DATA  0,11,11, 3,11,11,11, 3,11,11, 0
DATA 11,11,11,11,11,11,11,11,11,11,11
DATA 11, 3,11,15,15,11,15,15,11, 3,11
DATA  0,11, 0,11, 0, 0, 0,11, 0,11, 0
DATA 11, 0, 0, 0,11, 0,11, 0, 0, 0,11

' === SPRITE: INVADER B "Crab" FRAME 0 (11x8) ===
DATA  0,13, 0, 0,13, 0,13, 0, 0,13, 0
DATA  0, 0,13, 0,13, 0,13, 0,13, 0, 0
DATA  0,13,13,13,13,13,13,13,13,13, 0
DATA 13,13, 3,13,15,13,15,13, 3,13,13
DATA 14,13,13,13,13,13,13,13,13,13,14
DATA 14,13, 3,13,13,13,13,13, 3,13,14
DATA  0, 0,13,13, 0, 0, 0,13,13, 0, 0
DATA  0,13, 0, 0,13, 0,13, 0, 0,13, 0

' === SPRITE: INVADER B "Crab" FRAME 1 (11x8) ===
DATA  0,13, 0, 0,13, 0,13, 0, 0,13, 0
DATA 13, 0, 0,13, 0, 0, 0,13, 0, 0,13
DATA 13,13,13,13,13,13,13,13,13,13,13
DATA 13,13, 3,13,15,13,15,13, 3,13,13
DATA 14,13,13,13,13,13,13,13,13,13,14
DATA 14,13, 3,13,13,13,13,13, 3,13,14
DATA  0,13, 0,13, 0, 0, 0,13, 0,13, 0
DATA 13, 0,13, 0, 0, 0, 0, 0,13, 0,13

' === SPRITE: INVADER C "Octopus" FRAME 0 (12x8) ===
DATA  0, 0, 0,12,12,12,12,12, 0, 0, 0, 0
DATA  0, 0,12,12,12,12,12,12,12, 0, 0, 0
DATA  0,12,12, 4,12,12,12, 4,12,12, 0, 0
DATA 12,12,12,12,15,12,15,12,12,12,12, 0
DATA 12,12,12,12,12,12,12,12,12,12,12,12
DATA 12, 4,12, 4,12, 0, 0,12, 4,12, 4,12
DATA 12, 0, 4, 0,12, 0, 0,12, 0, 4, 0,12
DATA  0, 0,12, 0, 0, 0, 0, 0, 0,12, 0, 0

' === SPRITE: INVADER C "Octopus" FRAME 1 (12x8) ===
DATA  0, 0, 0,12,12,12,12,12, 0, 0, 0, 0
DATA  0, 0,12,12,12,12,12,12,12, 0, 0, 0
DATA  0,12,12, 4,12,12,12, 4,12,12, 0, 0
DATA 12,12,12,12,15,12,15,12,12,12,12, 0
DATA 12,12,12,12,12,12,12,12,12,12,12,12
DATA  4,12, 4,12, 0, 0, 0, 0,12, 4,12, 4
DATA  0, 4, 0,12, 0, 0, 0, 0,12, 0, 4, 0
DATA  0,12, 0, 0, 0, 0, 0, 0, 0, 0,12, 0

' === SPRITE: UFO SAUCER (16x7) ===
DATA  0, 0, 0, 0, 4,12,12, 4,12,12, 4, 0, 0, 0, 0, 0
DATA  0, 0, 0,12,12,12,12,12,12,12,12,12, 0, 0, 0, 0
DATA  0, 0,12,12,11,12,11,12,11,12,11,12,12, 0, 0, 0
DATA  0,12,12,12,12,12,12,12,12,12,12,12,12,12, 0, 0
DATA 12,12,12,12,12,12,12,12,12,12,12,12,12,12,12, 0
DATA  0, 4,12, 4,12, 4,12, 4,12, 4,12, 4,12, 4, 0, 0
DATA  0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0

' === SPRITE: EXPLOSION FRAME 0 (13x8) — small starburst ===
DATA  0, 0, 0, 0, 0,14, 0, 0, 0, 0, 0, 0, 0
DATA  0, 0, 0,14, 0, 0, 0,14, 0, 0, 0, 0, 0
DATA  0, 0, 0, 0,14, 0,14, 0, 0, 0, 0, 0, 0
DATA  0,14, 0,14, 0,14, 0,14, 0,14, 0, 0, 0
DATA  0, 0,14, 0,14, 0,14, 0,14, 0, 0, 0, 0
DATA  0, 0, 0,14, 0, 0, 0,14, 0, 0, 0, 0, 0
DATA  0, 0, 0, 0, 0,14, 0, 0, 0, 0, 0, 0, 0
DATA  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0

' === SPRITE: EXPLOSION FRAME 1 (13x8) — medium starburst ===
DATA  0, 0,14, 0, 0,14, 0, 0,14, 0, 0, 0, 0
DATA  0,14, 0,12, 0, 0, 0,12, 0,14, 0, 0, 0
DATA 14, 0,12,14,12,14,12,14,12, 0,14, 0, 0
DATA  0,12,14,12,14,14,14,12,14,12, 0, 0, 0
DATA 14, 0,12,14,14,14,14,14,12, 0,14, 0, 0
DATA  0,14, 0,12, 0,14, 0,12, 0,14, 0, 0, 0
DATA  0, 0,14, 0, 0,14, 0, 0,14, 0, 0, 0, 0
DATA  0, 0, 0, 0,14, 0,14, 0, 0, 0, 0, 0, 0

' === SPRITE: EXPLOSION FRAME 2 (13x8) — fading sparks ===
DATA  0, 0, 4, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0
DATA  0, 0, 0, 8, 0, 0, 0, 8, 0, 0, 0, 0, 0
DATA  0, 4, 0, 0, 8, 0, 8, 0, 0, 4, 0, 0, 0
DATA  0, 0, 8, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0
DATA  0, 4, 0, 0, 8, 0, 8, 0, 0, 4, 0, 0, 0
DATA  0, 0, 0, 8, 0, 0, 0, 8, 0, 0, 0, 0, 0
DATA  0, 0, 4, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0
DATA  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0

' === SPRITE: PLAYER BULLET (2x6) ===
DATA 15,15
DATA 15,15
DATA 15,15
DATA 15,15
DATA 15,15
DATA 15,15

' === SPRITE: INVADER BULLET TYPE 1 zigzag yellow (3x7) ===
DATA  0,14, 0
DATA 14, 0, 0
DATA  0,14, 0
DATA  0, 0,14
DATA  0,14, 0
DATA 14, 0, 0
DATA  0,14, 0

' === SPRITE: INVADER BULLET TYPE 2 straight cyan (3x7) ===
DATA  0,11, 0
DATA  0,11, 0
DATA  0,11, 0
DATA  0,11, 0
DATA  0,11, 0
DATA  0,11, 0
DATA  0,11, 0

' === SPRITE: INVADER BULLET TYPE 3 squiggle magenta (3x7) ===
DATA 13, 0, 0
DATA  0,13, 0
DATA  0, 0,13
DATA  0,13, 0
DATA 13, 0, 0
DATA  0,13, 0
DATA  0, 0,13

' ────────────────────────────────────────────────────────────────────
' 5. LOAD SPRITES GOSUB
' ────────────────────────────────────────────────────────────────────
GOSUB LoadSprites

' ────────────────────────────────────────────────────────────────────
' 6. MAIN PROGRAM FLOW
' ────────────────────────────────────────────────────────────────────
SCREEN 13

' Generate stars
CALL GenStars

' Load high scores
CALL LoadHiScores

hiScore = scores(1).score
gameState = STATE_TITLE

DO
    SELECT CASE gameState
        CASE STATE_TITLE
            CALL ShowTitle
        CASE STATE_PLAYING
            CALL RunGame
        CASE STATE_HISCORES
            CALL ShowHiScores
        CASE STATE_INITIALS
            CALL EnterInitials
        CASE STATE_GAMEOVER
            CALL ShowGameOver
    END SELECT
LOOP UNTIL gameState = STATE_QUIT

SCREEN 0
COLOR 7, 0
CLS
PRINT "Thanks for playing SPACE INVADERS - QB45 Port 1993"
END
' ════════════════════════════════════════════════════════════════════
' SUBs AND FUNCTIONS (alphabetical)
' ════════════════════════════════════════════════════════════════════

' ════════════════════════════════
' SUB AddExplosion(px%, py%)
' Spawns explosion animation at px,py
' ════════════════════════════════
' ─────────────────────────────────────────────────────
' LoadSprites — reads DATA into sprite arrays
' Must appear between END and first SUB in QB45
' ─────────────────────────────────────────────────────
LoadSprites:
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER

    ' Player ship
    FOR r = 0 TO SHIP_H - 1
        FOR c = 0 TO SHIP_W - 1
            READ v
            shipSprite(c, r) = v
        NEXT c
    NEXT r

    ' Invader A frames 0 and 1
    DIM f AS INTEGER
    FOR f = 0 TO 1
        FOR r = 0 TO INVA_H - 1
            FOR c = 0 TO INVA_W - 1
                READ v
                invASprite(f, c, r) = v
            NEXT c
        NEXT r
    NEXT f

    ' Invader B frames 0 and 1
    FOR f = 0 TO 1
        FOR r = 0 TO INVB_H - 1
            FOR c = 0 TO INVB_W - 1
                READ v
                invBSprite(f, c, r) = v
            NEXT c
        NEXT r
    NEXT f

    ' Invader C frames 0 and 1
    FOR f = 0 TO 1
        FOR r = 0 TO INVC_H - 1
            FOR c = 0 TO INVC_W - 1
                READ v
                invCSprite(f, c, r) = v
            NEXT c
        NEXT r
    NEXT f

    ' UFO
    FOR r = 0 TO UFO_H - 1
        FOR c = 0 TO UFO_W - 1
            READ v
            ufoSprite(c, r) = v
        NEXT c
    NEXT r

    ' Explosion frames 0,1,2
    FOR f = 0 TO 2
        FOR r = 0 TO EXPL_H - 1
            FOR c = 0 TO EXPL_W - 1
                READ v
                explSprite(f, c, r) = v
            NEXT c
        NEXT r
    NEXT f

    ' Player bullet
    FOR r = 0 TO PBUL_H - 1
        FOR c = 0 TO PBUL_W - 1
            READ v
            pbulSprite(c, r) = v
        NEXT c
    NEXT r

    ' Invader bullet types 0,1,2
    FOR f = 0 TO 2
        FOR r = 0 TO IBUL_H - 1
            FOR c = 0 TO IBUL_W - 1
                READ v
                ibulSprite(f, c, r) = v
            NEXT c
        NEXT r
    NEXT f

RETURN

SUB AddExplosion (px AS INTEGER, py AS INTEGER)
    DIM i AS INTEGER
    FOR i = 1 TO MAX_EXPLS
        IF expl(i).active = 0 THEN
            expl(i).x = px
            expl(i).y = py
            expl(i).frame = 0
            expl(i).timer = 0
            expl(i).active = 1
            EXIT FOR
        END IF
    NEXT i
END SUB

' ════════════════════════════════
' FUNCTION BoxHit%(ax,ay,aw,ah,bx,by,bw,bh)
' AABB collision test
' ════════════════════════════════
FUNCTION BoxHit (ax AS INTEGER, ay AS INTEGER, _
                 aw AS INTEGER, ah AS INTEGER, _
                 bx AS INTEGER, by AS INTEGER, _
                 bw AS INTEGER, bh AS INTEGER) AS INTEGER
    IF (ax < bx + bw) AND (ax + aw > bx) AND _
       (ay < by + bh) AND (ay + ah > by) THEN
        BoxHit = 1
    ELSE
        BoxHit = 0
    END IF
END FUNCTION

' ════════════════════════════════
' SUB BuildBunkers
' Initialises 4 bunkers with arch shape
' ════════════════════════════════
SUB BuildBunkers ()
    DIM b AS INTEGER, bx AS INTEGER, col AS INTEGER, row AS INTEGER
    bnkY = 155
    FOR b = 1 TO NUM_BUNKERS
        bnkX(b) = 20 + (b - 1) * 72
    NEXT b

    ' Arch shape: fill all, then cut top-centre notch
    FOR b = 1 TO NUM_BUNKERS
        FOR col = 0 TO BUNKER_W - 1
            FOR row = 0 TO BUNKER_H - 1
                ' Cut the arch notch (bottom-centre opening)
                IF row >= BUNKER_H - 6 AND _
                   col >= 7 AND col <= 14 THEN
                    bnk(b, col, row) = 0
                ELSE
                    bnk(b, col, row) = DKGREEN
                END IF
            NEXT row
        NEXT col
    NEXT b
END SUB

' ════════════════════════════════
' SUB CheckCollisions
' All AABB checks for a game tick
' ════════════════════════════════
SUB CheckCollisions ()
    DIM col AS INTEGER, row AS INTEGER
    DIM invX AS INTEGER, invY AS INTEGER
    DIM i AS INTEGER, b AS INTEGER
    DIM iw AS INTEGER, ih AS INTEGER
    DIM pts AS INTEGER

    ' --- Player bullet vs invaders ---
    IF pbulActive THEN
        FOR col = 1 TO INV_COLS
            FOR row = 1 TO INV_ROWS
                IF alive(col, row) THEN
                    invX = frmX + (col - 1) * INV_SPACEX
                    invY = frmY + (row - 1) * INV_SPACEY
                    IF row <= 2 THEN
                        iw = INVA_W : ih = INVA_H
                    ELSEIF row <= 4 THEN
                        iw = INVB_W : ih = INVB_H
                    ELSE
                        iw = INVC_W : ih = INVC_H
                    END IF
                    IF BoxHit(pbulX, pbulY, PBUL_W, PBUL_H, _
                              invX, invY, iw, ih) THEN
                        ' Kill invader
                        alive(col, row) = 0
                        aliveCount = aliveCount - 1
                        ' Score
                        IF row <= 2 THEN pts = 30
                        IF row = 3 OR row = 4 THEN pts = 20
                        IF row = 5 THEN pts = 10
                        score = score + pts
                        IF score > hiScore THEN hiScore = score
                        ' Erase bullet
                        EraseSprite pbulX, pbulY, PBUL_W, PBUL_H
                        pbulActive = 0
                        ' Explosion
                        CALL AddExplosion(invX, invY)
                        CALL SoundInvaderDeath
                        ' Erase invader
                        EraseSprite invX, invY, iw, ih
                        GOTO DonePBulHit
                    END IF
                END IF
            NEXT row
        NEXT col

        ' Player bullet vs UFO
        IF ufoActive THEN
            IF BoxHit(pbulX, pbulY, PBUL_W, PBUL_H, _
                      ufoX, ufoY, UFO_W, UFO_H) THEN
                score = score + ufoScore
                IF score > hiScore THEN hiScore = score
                EraseSprite ufoX, ufoY, UFO_W, UFO_H
                ufoActive = 0
                EraseSprite pbulX, pbulY, PBUL_W, PBUL_H
                pbulActive = 0
                CALL AddExplosion(ufoX, ufoY)
                CALL SoundInvaderDeath
                GOTO DonePBulHit
            END IF
        END IF

        ' Player bullet off top
        IF pbulY < PLAY_TOP THEN
            EraseSprite pbulX, pbulY, PBUL_W, PBUL_H
            pbulActive = 0
        END IF
    END IF
    DonePBulHit:

    ' --- Invader bullets vs player ---
    FOR i = 1 TO MAX_IBULLETS
        IF ibul(i).active THEN
            IF BoxHit(ibul(i).x, ibul(i).y, IBUL_W, IBUL_H, _
                      shipX, shipY, SHIP_W, SHIP_H) THEN
                ibul(i).active = 0
                EraseSprite ibul(i).x, ibul(i).y, IBUL_W, IBUL_H
                CALL PlayerDie
                EXIT FOR
            END IF
            ' Off screen
            IF ibul(i).y > PLAY_BOT THEN
                EraseSprite ibul(i).x, ibul(i).y, IBUL_W, IBUL_H
                ibul(i).active = 0
            END IF
        END IF
    NEXT i

    ' --- Invader bottom vs player Y (instant game over) ---
    DIM botY AS INTEGER
    botY = frmY + (INV_ROWS - 1) * INV_SPACEY + INVC_H
    IF botY >= shipY THEN
        lives = 0
        gameState = STATE_GAMEOVER
    END IF
END SUB

' ════════════════════════════════
' SUB DrawBunkers
' Full redraw of all bunker pixels
' ════════════════════════════════
SUB DrawBunkers ()
    DIM b AS INTEGER, col AS INTEGER, row AS INTEGER
    FOR b = 1 TO NUM_BUNKERS
        FOR col = 0 TO BUNKER_W - 1
            FOR row = 0 TO BUNKER_H - 1
                IF bnk(b, col, row) <> 0 THEN
                    PSET (bnkX(b) + col, bnkY + row), bnk(b, col, row)
                ELSE
                    PSET (bnkX(b) + col, bnkY + row), BLACK
                END IF
            NEXT row
        NEXT col
    NEXT b
END SUB

' ════════════════════════════════
' SUB DrawGame
' Master draw — only changed elements
' ════════════════════════════════
SUB DrawGame ()
    DIM i AS INTEGER, col AS INTEGER, row AS INTEGER
    DIM invX AS INTEGER, invY AS INTEGER

    ' Draw invader formation
    FOR col = 1 TO INV_COLS
        FOR row = 1 TO INV_ROWS
            IF alive(col, row) THEN
                invX = frmX + (col - 1) * INV_SPACEX
                invY = frmY + (row - 1) * INV_SPACEY
                IF row <= 2 THEN
                    DrawSpriteInvA invX, invY, invFrame
                ELSEIF row <= 4 THEN
                    DrawSpriteInvB invX, invY, invFrame
                ELSE
                    DrawSpriteInvC invX, invY, invFrame
                END IF
            END IF
        NEXT row
    NEXT col

    ' Draw player ship
    DrawSpriteShip shipX, shipY

    ' Draw player bullet
    IF pbulActive THEN
        ' Inline draw for 2x6 white bullet
        LINE (pbulX, pbulY)-(pbulX + PBUL_W - 1, pbulY + PBUL_H - 1), _
             WHITE, BF
    END IF

    ' Draw invader bullets
    DIM bt AS INTEGER
    FOR i = 1 TO MAX_IBULLETS
        IF ibul(i).active THEN
            bt = ibul(i).btype - 1
            DrawSpriteIBul ibul(i).x, ibul(i).y, bt
        END IF
    NEXT i

    ' Draw UFO
    IF ufoActive THEN
        DrawSpriteUFO ufoX, ufoY
    END IF

    ' Draw explosions
    FOR i = 1 TO MAX_EXPLS
        IF expl(i).active THEN
            DrawSpriteExpl expl(i).x, expl(i).y, expl(i).frame
        END IF
    NEXT i

    ' Draw HUD
    CALL DrawHUD
END SUB

' ════════════════════════════════
' SUB DrawHUD
' Draws score bars top and bottom
' ════════════════════════════════
SUB DrawHUD ()
    ' Top bar background
    LINE (0, 0)-(SCR_W - 1, 14), BLACK, BF
    ' Bottom bar background
    LINE (0, 185)-(SCR_W - 1, SCR_H - 1), BLACK, BF
    ' Divider line
    LINE (0, 185)-(SCR_W - 1, 185), DKGREEN

    ' Score label left
    PrintStr "SCORE<1>", 2, 1, BRTRED
    DIM sc AS STRING
    sc = RIGHT$("000000" + LTRIM$(STR$(score)), 6)
    PrintStr sc, 2, 8, WHITE

    ' Hi-score centre
    PrintStr "HI-SCORE", 120, 1, BRTCYAN
    DIM hs AS STRING
    hs = RIGHT$("000000" + LTRIM$(STR$(hiScore)), 6)
    PrintStr hs, 128, 8, WHITE

    ' Level right
    DIM lvlStr AS STRING
    lvlStr = "LEVEL:" + LTRIM$(STR$(level))
    PrintStr lvlStr, 260, 1, YELLOW

    ' Lives
    PrintStr "LIVES:", 2, 188, WHITE
    DIM lv AS INTEGER
    FOR lv = 1 TO lives - 1
        ' Mini ship 7x5 at bottom
        DrawMiniShip 46 + (lv - 1) * 10, 188
    NEXT lv

    ' Credit
    PrintStr "CREDIT 00", 245, 188, WHITE
END SUB

' ════════════════════════════════
' SUB DrawMiniShip(x%,y%)
' Tiny 7x5 player ship for lives display
' ════════════════════════════════
SUB DrawMiniShip (x AS INTEGER, y AS INTEGER)
    PSET (x + 3, y), BRTGREEN
    PSET (x + 2, y + 1), BRTGREEN
    PSET (x + 3, y + 1), BRTGREEN
    PSET (x + 4, y + 1), BRTGREEN
    LINE (x, y + 2)-(x + 6, y + 2), BRTGREEN
    LINE (x, y + 3)-(x + 6, y + 3), DKGREEN
    LINE (x, y + 4)-(x + 6, y + 4), DKGREEN
END SUB

' ════════════════════════════════
' SUB DrawSpriteArr (generic 2D flat pass)
' Used for player bullet only
' ════════════════════════════════
SUB DrawSpriteArr (spr() AS INTEGER, x AS INTEGER, y AS INTEGER, _
                   w AS INTEGER, h AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO h - 1
        FOR c = 0 TO w - 1
            v = spr(c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ════════════════════════════════
' SUB DrawSpriteExpl(x,y,frame)
' Draw explosion sprite frame
' ════════════════════════════════
SUB DrawSpriteExpl (x AS INTEGER, y AS INTEGER, frm AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO EXPL_H - 1
        FOR c = 0 TO EXPL_W - 1
            v = explSprite(frm, c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ════════════════════════════════
' SUB DrawSpriteIBul(x,y,bt)
' Draw invader bullet type bt (0-2)
' ════════════════════════════════
SUB DrawSpriteIBul (x AS INTEGER, y AS INTEGER, bt AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO IBUL_H - 1
        FOR c = 0 TO IBUL_W - 1
            v = ibulSprite(bt, c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ════════════════════════════════
' SUB DrawSpriteInvA(x,y,frame)
' Draw Squid invader
' ════════════════════════════════
SUB DrawSpriteInvA (x AS INTEGER, y AS INTEGER, frm AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO INVA_H - 1
        FOR c = 0 TO INVA_W - 1
            v = invASprite(frm, c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ════════════════════════════════
' SUB DrawSpriteInvB(x,y,frame)
' Draw Crab invader
' ════════════════════════════════
SUB DrawSpriteInvB (x AS INTEGER, y AS INTEGER, frm AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO INVB_H - 1
        FOR c = 0 TO INVB_W - 1
            v = invBSprite(frm, c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ════════════════════════════════
' SUB DrawSpriteInvC(x,y,frame)
' Draw Octopus invader
' ════════════════════════════════
SUB DrawSpriteInvC (x AS INTEGER, y AS INTEGER, frm AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO INVC_H - 1
        FOR c = 0 TO INVC_W - 1
            v = invCSprite(frm, c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ════════════════════════════════
' SUB DrawSpriteShip(x,y)
' Draw player ship
' ════════════════════════════════
SUB DrawSpriteShip (x AS INTEGER, y AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO SHIP_H - 1
        FOR c = 0 TO SHIP_W - 1
            v = shipSprite(c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ════════════════════════════════
' SUB DrawSpriteUFO(x,y)
' Draw UFO saucer
' ════════════════════════════════
SUB DrawSpriteUFO (x AS INTEGER, y AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO UFO_H - 1
        FOR c = 0 TO UFO_W - 1
            v = ufoSprite(c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ════════════════════════════════
' SUB DrawStars
' Draws the starfield (static)
' ════════════════════════════════
SUB DrawStars ()
    DIM i AS INTEGER
    FOR i = 1 TO 80
        PSET (starX(i), starY(i)), starC(i)
    NEXT i
END SUB

' ════════════════════════════════
' SUB EnterInitials
' Prompt player for 3-char initials
' ════════════════════════════════
SUB EnterInitials ()
    DIM pos AS INTEGER
    DIM letters(1 TO 3) AS INTEGER
    DIM i AS INTEGER, k AS STRING
    DIM dispStr AS STRING
    DIM t AS DOUBLE
    CLS
    CALL DrawStars
    FOR i = 1 TO 3 : letters(i) = 65 : NEXT i  ' default AAA
    pos = 1

    DO
        ' Flash header
        IF (tickCount MOD 30) < 15 THEN
            PrintStr "ENTER YOUR INITIALS", 72, 80, YELLOW
        ELSE
            LINE (72, 80)-(248, 87), BLACK, BF
        END IF

        ' Show placeholders
        dispStr = CHR$(letters(1)) + " " + CHR$(letters(2)) + " " + CHR$(letters(3))
        LINE (120, 100)-(200, 108), BLACK, BF
        PrintStr dispStr, 130, 100, WHITE

        ' Cursor underline
        LINE (130 + (pos - 1) * 14, 109)-(136 + (pos - 1) * 14, 109), YELLOW

        tickCount = tickCount + 1
        k = INKEY$
        IF k = CHR$(27) THEN gameState = STATE_TITLE : EXIT SUB
        IF k = CHR$(32) OR k = "z" OR k = "Z" THEN
            pos = pos + 1
            IF pos > 3 THEN
                initials = CHR$(letters(1)) + CHR$(letters(2)) + CHR$(letters(3))
                CALL InsertScore(initials, score, level)
                CALL SaveHiScores
                gameState = STATE_HISCORES
                EXIT SUB
            END IF
        END IF
        IF k = CHR$(0) + CHR$(72) THEN ' up arrow
            letters(pos) = letters(pos) + 1
            IF letters(pos) > 90 THEN letters(pos) = 65
        END IF
        IF k = CHR$(0) + CHR$(80) THEN ' down arrow
            letters(pos) = letters(pos) - 1
            IF letters(pos) < 65 THEN letters(pos) = 90
        END IF

        ' Delay
        t = TIMER
        DO WHILE TIMER - t < .05 : LOOP
    LOOP
END SUB

' ════════════════════════════════
' SUB EraseSprite(x,y,w,h)
' Black fill over sprite bounds
' ════════════════════════════════
SUB EraseSprite (x AS INTEGER, y AS INTEGER, _
                 w AS INTEGER, h AS INTEGER)
    LINE (x, y)-(x + w - 1, y + h - 1), BLACK, BF
END SUB

' ════════════════════════════════
' SUB FireInvaderBullet
' Random bottom invader fires
' ════════════════════════════════
SUB FireInvaderBullet ()
    DIM slot AS INTEGER, i AS INTEGER
    ' Find free slot
    slot = 0
    FOR i = 1 TO MAX_IBULLETS
        IF ibul(i).active = 0 THEN slot = i : EXIT FOR
    NEXT i
    IF slot = 0 THEN EXIT SUB

    ' Pick random column, find lowest alive in that col
    DIM attempts AS INTEGER, c AS INTEGER, r AS INTEGER
    DIM found AS INTEGER
    found = 0
    FOR attempts = 1 TO 20
        c = INT(RND * INV_COLS) + 1
        FOR r = INV_ROWS TO 1 STEP -1
            IF alive(c, r) THEN
                ibul(slot).x = frmX + (c - 1) * INV_SPACEX + 4
                ibul(slot).y = frmY + (r - 1) * INV_SPACEY + INVC_H
                ibulTypeIdx = (ibulTypeIdx MOD 3) + 1
                ibul(slot).btype = ibulTypeIdx
                ibul(slot).active = 1
                found = 1
                EXIT FOR
            END IF
        NEXT r
        IF found THEN EXIT FOR
    NEXT attempts
END SUB

' ════════════════════════════════
' SUB GenStars
' Generate 80 static stars for bg
' ════════════════════════════════
SUB GenStars ()
    DIM i AS INTEGER
    DIM layer AS INTEGER
    RANDOMIZE TIMER
    FOR i = 1 TO 80
        starX(i) = INT(RND * SCR_W)
        starY(i) = INT(RND * SCR_H)
        layer = INT(RND * 3)
        IF layer = 0 THEN starC(i) = DKGRAY
        IF layer = 1 THEN starC(i) = LTGRAY
        IF layer = 2 THEN starC(i) = WHITE
    NEXT i
END SUB

' ════════════════════════════════
' SUB InitGame
' Reset all game state for new game
' ════════════════════════════════
SUB InitGame ()
    DIM c AS INTEGER, r AS INTEGER, i AS INTEGER

    score = 0
    lives = 3
    level = 1
    aliveCount = INV_COLS * INV_ROWS
    frmX = INV_STARTX
    frmY = INV_STARTY
    frmDX = 1
    frmStep = 1
    invFrame = 0
    marchNote = 0
    pbulActive = 0
    ufoActive = 0
    ufoTimer = 0
    ibulTypeIdx = 0
    tickCount = 0

    FOR c = 1 TO INV_COLS
        FOR r = 1 TO INV_ROWS
            alive(c, r) = 1
        NEXT r
    NEXT c

    FOR i = 1 TO MAX_IBULLETS
        ibul(i).active = 0
    NEXT i
    FOR i = 1 TO MAX_EXPLS
        expl(i).active = 0
    NEXT i

    shipX = SCR_W \ 2 - SHIP_W \ 2
    shipY = PLAY_BOT - SHIP_H - 2
    shipOldX = shipX

    CALL BuildBunkers
END SUB

' ════════════════════════════════
' SUB InitLevel
' Reset formation for new level
' ════════════════════════════════
SUB InitLevel ()
    DIM c AS INTEGER, r AS INTEGER, i AS INTEGER

    aliveCount = INV_COLS * INV_ROWS
    frmX = INV_STARTX
    frmY = INV_STARTY + (level - 1) * 4
    IF frmY > 60 THEN frmY = 60
    frmDX = 1
    invFrame = 0
    marchNote = 0

    FOR c = 1 TO INV_COLS
        FOR r = 1 TO INV_ROWS
            alive(c, r) = 1
        NEXT r
    NEXT c

    FOR i = 1 TO MAX_IBULLETS
        IF ibul(i).active THEN
            EraseSprite ibul(i).x, ibul(i).y, IBUL_W, IBUL_H
        END IF
        ibul(i).active = 0
    NEXT i
    pbulActive = 0

    ' Rebuild bunkers every 3 levels
    IF (level MOD 3) = 1 THEN
        CALL BuildBunkers
        CALL DrawBunkers
    END IF
END SUB

' ════════════════════════════════
' SUB InsertScore(nm,sc,lv)
' Insert new score into sorted table
' ════════════════════════════════
SUB InsertScore (nm AS STRING, sc AS LONG, lv AS INTEGER)
    DIM i AS INTEGER, j AS INTEGER
    ' Find insertion point
    i = 11
    DO WHILE i > 1
        IF sc > scores(i - 1).score THEN
            i = i - 1
        ELSE
            EXIT DO
        END IF
    LOOP
    IF i > 10 THEN EXIT SUB
    ' Shift down
    FOR j = 10 TO i + 1 STEP -1
        scores(j) = scores(j - 1)
    NEXT j
    scores(i).playerName = LEFT$(nm + "   ", 3)
    scores(i).score = sc
    scores(i).level = lv
    newScoreRank = i
END SUB

' ════════════════════════════════
' SUB LoadHiScores
' Load or initialise HISCORES.DAT
' ════════════════════════════════
SUB LoadHiScores ()
    DIM i AS INTEGER
    DIM fNum AS INTEGER
    fNum = FREEFILE
    ON ERROR GOTO DefaultScores
    OPEN "HISCORES.DAT" FOR BINARY AS #fNum
    IF LOF(fNum) = 0 THEN CLOSE #fNum : GOTO DefaultScores
    FOR i = 1 TO 10
        GET #fNum, , scores(i)
    NEXT i
    CLOSE #fNum
    ON ERROR GOTO 0
    EXIT SUB
    DefaultScores:
    ON ERROR GOTO 0
    DIM defNames(1 TO 10) AS STRING
    DIM defPts(1 TO 10) AS LONG
    defNames(1) = "AAA" : defPts(1) = 5000
    defNames(2) = "BBB" : defPts(2) = 4000
    defNames(3) = "CCC" : defPts(3) = 3000
    defNames(4) = "DDD" : defPts(4) = 2500
    defNames(5) = "EEE" : defPts(5) = 2000
    defNames(6) = "FFF" : defPts(6) = 1500
    defNames(7) = "GGG" : defPts(7) = 1000
    defNames(8) = "HHH" : defPts(8) = 800
    defNames(9) = "III" : defPts(9)  = 600
    defNames(10) = "JJJ" : defPts(10) = 500
    FOR i = 1 TO 10
        scores(i).playerName = defNames(i)
        scores(i).score = defPts(i)
        scores(i).level = 1
    NEXT i
END SUB

' ════════════════════════════════
' SUB MoveFormation
' Advance invader formation one tick
' ════════════════════════════════
SUB MoveFormation ()
    DIM spd AS INTEGER
    DIM col AS INTEGER, row AS INTEGER
    DIM rightmost AS INTEGER, leftmost AS INTEGER
    DIM invX AS INTEGER, invY AS INTEGER
    DIM oldFrmX AS INTEGER, oldFrmY AS INTEGER
    DIM iw AS INTEGER, ih AS INTEGER

    spd = 1 + (55 - aliveCount) \ 5
    IF spd > 8 THEN spd = 8

    ' Erase formation at old positions
    oldFrmX = frmX
    oldFrmY = frmY
    FOR col = 1 TO INV_COLS
        FOR row = 1 TO INV_ROWS
            IF alive(col, row) THEN
                invX = oldFrmX + (col - 1) * INV_SPACEX
                invY = oldFrmY + (row - 1) * INV_SPACEY
                IF row <= 2 THEN
                    iw = INVA_W : ih = INVA_H
                ELSEIF row <= 4 THEN
                    iw = INVB_W : ih = INVB_H
                ELSE
                    iw = INVC_W : ih = INVC_H
                END IF
                EraseSprite invX, invY, iw, ih
            END IF
        NEXT row
    NEXT col

    frmX = frmX + frmDX * spd

    ' Find extents of living invaders
    rightmost = 0 : leftmost = SCR_W
    FOR col = 1 TO INV_COLS
        FOR row = 1 TO INV_ROWS
            IF alive(col, row) THEN
                invX = frmX + (col - 1) * INV_SPACEX
                IF invX + INV_SPACEX > rightmost THEN
                    rightmost = invX + INV_SPACEX
                END IF
                IF invX < leftmost THEN leftmost = invX
            END IF
        NEXT row
    NEXT col

    ' Boundary hit — drop and reverse
    IF frmDX = 1 AND rightmost >= 295 THEN
        frmDX = -1
        frmY = frmY + 8
        SOUND 160, 1
    ELSEIF frmDX = -1 AND leftmost <= 5 THEN
        frmDX = 1
        frmY = frmY + 8
        SOUND 160, 1
    END IF

    ' Toggle animation frame
    invFrame = 1 - invFrame

    ' March sound
    SELECT CASE marchNote
        CASE 0 : SOUND 160, 1
        CASE 1 : SOUND 140, 1
        CASE 2 : SOUND 120, 1
        CASE 3 : SOUND 100, 1
    END SELECT
    marchNote = (marchNote + 1) MOD 4
END SUB

' ════════════════════════════════
' SUB PlayerDie
' Handle player death sequence
' ════════════════════════════════
SUB PlayerDie ()
    DIM i AS INTEGER
    DIM t2 AS DOUBLE
    CALL SoundPlayerDeath
    CALL AddExplosion(shipX, shipY)

    ' Animate explosion
    FOR i = 1 TO 60
        t2 = TIMER
        DO WHILE TIMER - t2 < .016 : LOOP
        CALL UpdateExplosions
        CALL DrawGame
    NEXT i

    lives = lives - 1
    IF lives <= 0 THEN
        gameState = STATE_GAMEOVER
        EXIT SUB
    END IF

    ' Respawn
    EraseSprite shipX, shipY, SHIP_W, SHIP_H
    shipX = SCR_W \ 2 - SHIP_W \ 2
    shipY = PLAY_BOT - SHIP_H - 2
    shipOldX = shipX
    DrawSpriteShip shipX, shipY

    ' Short pause
    t2 = TIMER
    DO WHILE TIMER - t2 < 2 : LOOP
END SUB

' ════════════════════════════════
' SUB PrintStr(s$, x%, y%, col%)
' Draw text using PRINT at scaled pos
' Uses LOCATE for simple text output
' ════════════════════════════════
SUB PrintStr (s AS STRING, x AS INTEGER, y AS INTEGER, _
              col AS INTEGER)
    ' Map pixel coords to text rows/cols
    ' SCREEN 13: 40 cols x 25 rows (8x8 char cells)
    DIM txRow AS INTEGER, txCol AS INTEGER
    txRow = (y \ 8) + 1
    txCol = (x \ 8) + 1
    IF txRow < 1 THEN txRow = 1
    IF txRow > 25 THEN txRow = 25
    IF txCol < 1 THEN txCol = 1
    IF txCol > 40 THEN txCol = 40
    LOCATE txRow, txCol
    COLOR col
    PRINT s;
END SUB

' ════════════════════════════════
' SUB RunGame
' Main gameplay loop
' ════════════════════════════════
SUB RunGame ()
    DIM k AS STRING
    DIM frmTickCount AS INTEGER
    DIM curTime AS DOUBLE
    DIM frmDelay AS INTEGER
    DIM ib AS INTEGER
    DIM fireDelay AS INTEGER
    DIM ufoSpawnRate AS INTEGER
    DIM ufoDir AS INTEGER
    DIM ufoScores(0 TO 4) AS INTEGER
    DIM wt AS DOUBLE
    frmTickCount = 0
    ufoSpawnRate = 1500
    ufoScores(0) = 50
    ufoScores(1) = 100
    ufoScores(2) = 150
    ufoScores(3) = 200
    ufoScores(4) = 300
    CLS
    CALL InitGame
    CALL DrawBunkers
    CALL DrawGame
    lastTime = TIMER

    DO
        curTime = TIMER
        IF curTime - lastTime >= TICK_DELAY THEN
            lastTime = curTime
            tickCount = tickCount + 1

            ' Input
            k = INKEY$
            IF k = CHR$(27) THEN gameState = STATE_TITLE : EXIT SUB

            ' Move player
            shipOldX = shipX
            IF k = CHR$(0) + CHR$(75) THEN   ' left arrow
                shipX = shipX - 3
                IF shipX < 10 THEN shipX = 10
            END IF
            IF k = CHR$(0) + CHR$(77) THEN   ' right arrow
                shipX = shipX + 3
                IF shipX > SCR_W - SHIP_W - 10 THEN
                    shipX = SCR_W - SHIP_W - 10
                END IF
            END IF
            IF (k = " " OR k = "z" OR k = "Z") AND pbulActive = 0 THEN
                pbulActive = 1
                pbulX = shipX + SHIP_W \ 2 - 1
                pbulY = shipY - PBUL_H
                SOUND 800, 1 : SOUND 600, 1
            END IF

            ' Erase old ship pos if moved
            IF shipX <> shipOldX THEN
                EraseSprite shipOldX, shipY, SHIP_W, SHIP_H
            END IF

            ' Move player bullet
            IF pbulActive THEN
                EraseSprite pbulX, pbulY, PBUL_W, PBUL_H
                pbulY = pbulY - 4
            END IF

            ' Move invader formation every few ticks
            frmTickCount = frmTickCount + 1
            frmDelay = 6 - level
            IF frmDelay < 1 THEN frmDelay = 1
            IF frmTickCount >= frmDelay THEN
                frmTickCount = 0
                CALL MoveFormation
            END IF

            ' Move invader bullets
            FOR ib = 1 TO MAX_IBULLETS
                IF ibul(ib).active THEN
                    EraseSprite ibul(ib).x, ibul(ib).y, IBUL_W, IBUL_H
                    ibul(ib).y = ibul(ib).y + 2
                END IF
            NEXT ib

            ' Invader fires?
            fireDelay = 40 - level * 3
            IF fireDelay < 10 THEN fireDelay = 10
            IF (tickCount MOD fireDelay) = 0 THEN
                CALL FireInvaderBullet
            END IF

            ' UFO logic
            ufoTimer = ufoTimer + 1
            IF ufoActive = 0 AND ufoTimer > 100 AND (ufoTimer MOD ufoSpawnRate) = 0 THEN
                ufoActive = 1
                ufoDir = INT(RND * 2)
                IF ufoDir = 0 THEN
                    ufoX = -UFO_W : ufoDX = 2
                ELSE
                    ufoX = SCR_W : ufoDX = -2
                END IF
                ufoY = PLAY_TOP + 4
                ' Random score
                ufoScore = ufoScores(INT(RND * 5))
            END IF
            IF ufoActive THEN
                EraseSprite ufoX, ufoY, UFO_W, UFO_H
                ufoX = ufoX + ufoDX
                IF ufoX < -UFO_W OR ufoX > SCR_W THEN
                    ufoActive = 0
                END IF
                ' UFO sound
                IF (tickCount MOD 4) < 2 THEN
                    SOUND 800, 1
                ELSE
                    SOUND 600, 1
                END IF
            END IF

            ' Collisions
            CALL CheckCollisions

            ' Erase bunker pixels hit by bullets
            CALL UpdateBunkerErosion

            ' Update explosion animations
            CALL UpdateExplosions

            ' Draw everything
            CALL DrawGame

            ' Check win condition
            IF aliveCount <= 0 THEN
                CALL SoundLevelClear
                wt = TIMER
                DO WHILE TIMER - wt < 2 : LOOP
                level = level + 1
                IF level > 10 THEN level = 10
                CALL InitLevel
                ' Redraw field
                CLS
                CALL DrawBunkers
                CALL DrawGame
            END IF
        END IF
    LOOP UNTIL gameState <> STATE_PLAYING
END SUB

' ════════════════════════════════
' SUB SaveHiScores
' Write HISCORES.DAT
' ════════════════════════════════
SUB SaveHiScores ()
    DIM fNum AS INTEGER, i AS INTEGER
    fNum = FREEFILE
    OPEN "HISCORES.DAT" FOR BINARY AS #fNum
    FOR i = 1 TO 10
        PUT #fNum, , scores(i)
    NEXT i
    CLOSE #fNum
END SUB

' ════════════════════════════════
' SUB ShowGameOver
' Game over screen then route
' ════════════════════════════════
SUB ShowGameOver ()
    CLS
    CALL DrawStars
    PrintStr "GAME  OVER", 100, 88, BRTRED
    PrintStr RIGHT$("000000" + LTRIM$(STR$(score)), 6), 128, 100, WHITE

    DIM t AS DOUBLE
    t = TIMER
    DO WHILE TIMER - t < 3 : LOOP

    ' Check if score qualifies
    IF score > scores(10).score THEN
        gameState = STATE_INITIALS
    ELSE
        gameState = STATE_TITLE
    END IF
END SUB

' ════════════════════════════════
' SUB ShowHiScores
' Full hi-score display screen
' ════════════════════════════════
SUB ShowHiScores ()
    CLS
    CALL DrawStars

    ' Header
    PrintStr "HIGH SCORES", 96, 10, BRTRED

    ' Subheader
    PrintStr "RANK  SCORE  NAME  LVL", 48, 28, DKGRAY
    LINE (8, 36)-(312, 36), DKCYAN

    DIM rankStr(1 TO 10) AS STRING
    rankStr(1) = "1ST"
    rankStr(2) = "2ND"
    rankStr(3) = "3RD"
    rankStr(4) = "4TH"
    rankStr(5) = "5TH"
    rankStr(6) = "6TH"
    rankStr(7) = "7TH"
    rankStr(8) = "8TH"
    rankStr(9) = "9TH"
    rankStr(10) = "10TH"

    DIM i AS INTEGER
    DIM ycur AS INTEGER
    ycur = 40

    ' Flash toggle for #1
    DIM flashCol AS INTEGER
    IF (tickCount MOD 40) < 20 THEN
        flashCol = BRTRED
    ELSE
        flashCol = YELLOW
    END IF
    tickCount = tickCount + 1

    DIM entryCol AS INTEGER
    DIM scStr AS STRING
    FOR i = 1 TO 10
        IF i = 1 THEN
            entryCol = flashCol
        ELSEIF i = newScoreRank THEN
            entryCol = BRTGREEN
        ELSE
            entryCol = WHITE
        END IF

        ' Rank
        PrintStr rankStr(i), 16, ycur, YELLOW
        ' Score
        scStr = RIGHT$("000000" + LTRIM$(STR$(scores(i).score)), 6)
        PrintStr scStr, 56, ycur, entryCol
        ' Name
        PrintStr scores(i).playerName, 136, ycur, BRTCYAN
        ' Level
        PrintStr LTRIM$(STR$(scores(i).level)), 184, ycur, LTGRAY

        ycur = ycur + 12
    NEXT i

    LINE (8, 168)-(312, 168), DKCYAN

    ' Prompt
    IF (tickCount MOD 30) < 15 THEN
        PrintStr "PRESS FIRE TO CONTINUE", 56, 176, YELLOW
    ELSE
        LINE (56, 176)-(264, 184), BLACK, BF
    END IF

    DIM k AS STRING
    k = INKEY$
    IF k = " " OR k = "z" OR k = "Z" THEN
        newScoreRank = 0
        gameState = STATE_TITLE
    END IF
    IF k = CHR$(27) THEN
        gameState = STATE_TITLE
    END IF

    DIM dt AS DOUBLE
    dt = TIMER
    DO WHILE TIMER - dt < .033 : LOOP
END SUB

' ════════════════════════════════
' SUB ShowTitle
' Title screen with all elements
' ════════════════════════════════
SUB ShowTitle ()
    CLS
    CALL DrawStars

    ' Hi-score at top
    PrintStr "HI-SCORE", 120, 2, BRTCYAN
    DIM hsStr AS STRING
    hsStr = RIGHT$("000000" + LTRIM$(STR$(hiScore)), 6)
    PrintStr hsStr, 128, 10, WHITE

    ' Drop shadow for title
    PrintStr "SPACE", 88 + 1, 34 + 1, BLACK
    PrintStr "INVADERS", 72 + 1, 50 + 1, BLACK
    ' Title text
    PrintStr "SPACE", 88, 34, BRTCYAN
    PrintStr "INVADERS", 72, 50, BRTRED

    ' Invader parade
    DIM paradeY AS INTEGER
    paradeY = 78

    ' Type C = 10 pts
    DrawSpriteInvC 40, paradeY, invFrame
    PrintStr "= 10 PTS", 64, paradeY + 1, WHITE

    ' Type B = 20 pts
    DrawSpriteInvB 40, paradeY + 18, invFrame
    PrintStr "= 20 PTS", 64, paradeY + 19, WHITE

    ' Type A = 30 pts
    DrawSpriteInvA 40, paradeY + 36, invFrame
    PrintStr "= 30 PTS", 64, paradeY + 37, WHITE

    ' UFO = ???
    DrawSpriteUFO 36, paradeY + 54
    PrintStr "= ??? PTS", 64, paradeY + 55, WHITE

    ' Flash prompt
    IF (tickCount MOD 30) < 15 THEN
        PrintStr "PRESS FIRE TO START", 72, 152, YELLOW
    ELSE
        LINE (72, 152)-(248, 159), BLACK, BF
    END IF

    ' Copyright
    PrintStr "ORIGINALLY  1978 TAITO  QB PORT 2026", 24, 192, DKGRAY

    tickCount = tickCount + 1

    ' Animate parade invaders
    DIM t AS DOUBLE
    t = TIMER
    DO WHILE TIMER - t < .25 : LOOP
    ' Erase and redraw parade invaders with toggled frame
    invFrame = 1 - invFrame

    ' Input
    DIM k AS STRING
    k = INKEY$
    IF k = CHR$(27) THEN gameState = STATE_QUIT : EXIT SUB
    IF k = " " OR k = "z" OR k = "Z" THEN
        gameState = STATE_PLAYING
    END IF
    IF k = "h" OR k = "H" THEN
        gameState = STATE_HISCORES
    END IF
END SUB

' ════════════════════════════════
' SUB SoundInvaderDeath
' ════════════════════════════════
SUB SoundInvaderDeath ()
    SOUND 400, 1 : SOUND 300, 1 : SOUND 200, 1
END SUB

' ════════════════════════════════
' SUB SoundLevelClear
' ════════════════════════════════
SUB SoundLevelClear ()
    SOUND 262, 2 : SOUND 330, 2
    SOUND 392, 2 : SOUND 523, 3
END SUB

' ════════════════════════════════
' SUB SoundPlayerDeath
' ════════════════════════════════
SUB SoundPlayerDeath ()
    SOUND 300, 2 : SOUND 200, 2
    SOUND 150, 2 : SOUND 100, 3
END SUB

' ════════════════════════════════
' SUB UpdateBunkerErosion
' Erode bunker pixels hit by bullets
' ════════════════════════════════
SUB UpdateBunkerErosion ()
    DIM b AS INTEGER, col AS INTEGER, row AS INTEGER
    DIM bx AS INTEGER, by AS INTEGER
    DIM px AS INTEGER, py AS INTEGER
    DIM ib AS INTEGER

    ' Check invader bullets vs bunkers
    DIM ibHit AS INTEGER
    FOR ib = 1 TO MAX_IBULLETS
        ibHit = 0
        IF ibul(ib).active THEN
            FOR b = 1 TO NUM_BUNKERS
            IF ibHit = 0 THEN
                FOR col = 0 TO BUNKER_W - 1
                IF ibHit = 0 THEN
                    FOR row = 0 TO BUNKER_H - 1
                        IF ibHit = 0 AND bnk(b, col, row) <> 0 THEN
                            bx = bnkX(b) + col
                            by = bnkY + row
                            IF BoxHit(ibul(ib).x, ibul(ib).y, _
                                      IBUL_W, IBUL_H, _
                                      bx, by, 1, 1) THEN
                                bnk(b, col, row) = 0
                                PSET (bx, by), BLACK
                                EraseSprite ibul(ib).x, ibul(ib).y, _
                                            IBUL_W, IBUL_H
                                ibul(ib).active = 0
                                ibHit = 1
                            END IF
                        END IF
                    NEXT row
                END IF
                NEXT col
            END IF
            NEXT b
        END IF
    NEXT ib

    ' Check player bullet vs bunkers
    DIM pbHit AS INTEGER
    IF pbulActive THEN
        pbHit = 0
        FOR b = 1 TO NUM_BUNKERS
        IF pbHit = 0 THEN
            FOR col = 0 TO BUNKER_W - 1
            IF pbHit = 0 THEN
                FOR row = 0 TO BUNKER_H - 1
                    IF pbHit = 0 AND bnk(b, col, row) <> 0 THEN
                        bx = bnkX(b) + col
                        by = bnkY + row
                        IF BoxHit(pbulX, pbulY, PBUL_W, PBUL_H, _
                                  bx, by, 1, 1) THEN
                            bnk(b, col, row) = 0
                            PSET (bx, by), BLACK
                            EraseSprite pbulX, pbulY, PBUL_W, PBUL_H
                            pbulActive = 0
                            pbHit = 1
                        END IF
                    END IF
                NEXT row
            END IF
            NEXT col
        END IF
        NEXT b
    END IF
END SUB

' ════════════════════════════════
' SUB UpdateExplosions
' Advance explosion animation frames
' ════════════════════════════════
SUB UpdateExplosions ()
    DIM i AS INTEGER
    FOR i = 1 TO MAX_EXPLS
        IF expl(i).active THEN
            expl(i).timer = expl(i).timer + 1
            IF expl(i).timer >= 8 THEN
                expl(i).timer = 0
                EraseSprite expl(i).x, expl(i).y, EXPL_W, EXPL_H
                expl(i).frame = expl(i).frame + 1
                IF expl(i).frame > 2 THEN
                    expl(i).active = 0
                END IF
            END IF
        END IF
    NEXT i
END SUB

' END OF INVADERS.BAS


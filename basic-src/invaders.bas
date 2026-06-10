' ====================================================================
' SPACE INVADERS  -  QBasic 4.5 Port  (c) 1993 QB Port
' Single-file, SCREEN 13 (320x200 256-color), no external libs.
' ====================================================================

' --------------------------------------------------------------------
' 1. CONSTANTS
' --------------------------------------------------------------------
CONST SCRW = 320
CONST SCRH = 200
CONST PLAYTOP = 16
CONST PLAYBOT = 184

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
CONST STATETITLE = 0
CONST STATEPLAYING = 1
CONST STATEHISCORES = 2
CONST STATEGAMEOVER = 3
CONST STATEINITIALS = 4
CONST STATEQUIT = 5

' Formation constants
CONST INVCOLS = 11
CONST INVROWS = 5
CONST INVSPACEX = 16
CONST INVSPACEY = 14
CONST INVSTARTX = 16
CONST INVSTARTY = 32

' Sprite dimensions
CONST SHIPW = 13
CONST SHIPH = 8
CONST INVAW = 11
CONST INVAH = 8
CONST INVBW = 11
CONST INVBH = 8
CONST INVCW = 12
CONST INVCH = 8
CONST UFOW = 16
CONST UFOH = 7
CONST EXPLW = 13
CONST EXPLH = 8
CONST PBULW = 2
CONST PBULH = 6
CONST IBULW = 3
CONST IBULH = 7

CONST BUNKERW = 22
CONST BUNKERH = 16
CONST NUMBUNKERS = 4

CONST MAXIBULLETS = 3
CONST MAXEXPLS = 6

CONST TICKDELAY = .016

' --------------------------------------------------------------------
' 2. TYPE DEFINITIONS
' --------------------------------------------------------------------
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
    tmr AS INTEGER
    active AS INTEGER
END TYPE

' --------------------------------------------------------------------
' 3. DIM SHARED (global game state)
' --------------------------------------------------------------------
DIM SHARED gameState AS INTEGER
DIM SHARED score AS LONG
DIM SHARED hiScore AS LONG
DIM SHARED lives AS INTEGER
DIM SHARED level AS INTEGER
DIM SHARED aliveCount AS INTEGER

' Formation
DIM SHARED alive(1 TO INVCOLS, 1 TO INVROWS) AS INTEGER
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
DIM SHARED ibul(1 TO MAXIBULLETS) AS BulletType

' UFO
DIM SHARED ufoX AS INTEGER
DIM SHARED ufoY AS INTEGER
DIM SHARED ufoActive AS INTEGER
DIM SHARED ufoDX AS INTEGER
DIM SHARED ufoTimer AS INTEGER
DIM SHARED ufoScore AS INTEGER

' Explosions
DIM SHARED expl(1 TO MAXEXPLS) AS ExplType

' Bunkers - pixel arrays 22x16
DIM SHARED bnk(1 TO NUMBUNKERS, 0 TO BUNKERW - 1, 0 TO BUNKERH - 1) AS INTEGER
DIM SHARED bnkX(1 TO NUMBUNKERS) AS INTEGER
DIM SHARED bnkY AS INTEGER

' Sprite data arrays
DIM SHARED shipSprite(0 TO SHIPW - 1, 0 TO SHIPH - 1) AS INTEGER
DIM SHARED invASprite(0 TO 1, 0 TO INVAW - 1, 0 TO INVAH - 1) AS INTEGER
DIM SHARED invBSprite(0 TO 1, 0 TO INVBW - 1, 0 TO INVBH - 1) AS INTEGER
DIM SHARED invCSprite(0 TO 1, 0 TO INVCW - 1, 0 TO INVCH - 1) AS INTEGER
DIM SHARED ufoSprite(0 TO UFOW - 1, 0 TO UFOH - 1) AS INTEGER
DIM SHARED explSprite(0 TO 2, 0 TO EXPLW - 1, 0 TO EXPLH - 1) AS INTEGER
DIM SHARED pbulSprite(0 TO PBULW - 1, 0 TO PBULH - 1) AS INTEGER
DIM SHARED ibulSprite(0 TO 2, 0 TO IBULW - 1, 0 TO IBULH - 1) AS INTEGER

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
DIM SHARED lastHudScore AS LONG
DIM SHARED lastHudLives AS INTEGER
DIM SHARED lastHudLevel AS INTEGER
DIM SHARED initials AS STRING
DIM SHARED colAlive(1 TO INVCOLS) AS INTEGER
DIM SHARED shipDirty AS INTEGER

' --------------------------------------------------------------------
' 4. SPRITE DATA BLOCKS
' --------------------------------------------------------------------

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

' === SPRITE: EXPLOSION FRAME 0 (13x8) - small starburst ===
DATA  0, 0, 0, 0, 0,14, 0, 0, 0, 0, 0, 0, 0
DATA  0, 0, 0,14, 0, 0, 0,14, 0, 0, 0, 0, 0
DATA  0, 0, 0, 0,14, 0,14, 0, 0, 0, 0, 0, 0
DATA  0,14, 0,14, 0,14, 0,14, 0,14, 0, 0, 0
DATA  0, 0,14, 0,14, 0,14, 0,14, 0, 0, 0, 0
DATA  0, 0, 0,14, 0, 0, 0,14, 0, 0, 0, 0, 0
DATA  0, 0, 0, 0, 0,14, 0, 0, 0, 0, 0, 0, 0
DATA  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0

' === SPRITE: EXPLOSION FRAME 1 (13x8) - medium starburst ===
DATA  0, 0,14, 0, 0,14, 0, 0,14, 0, 0, 0, 0
DATA  0,14, 0,12, 0, 0, 0,12, 0,14, 0, 0, 0
DATA 14, 0,12,14,12,14,12,14,12, 0,14, 0, 0
DATA  0,12,14,12,14,14,14,12,14,12, 0, 0, 0
DATA 14, 0,12,14,14,14,14,14,12, 0,14, 0, 0
DATA  0,14, 0,12, 0,14, 0,12, 0,14, 0, 0, 0
DATA  0, 0,14, 0, 0,14, 0, 0,14, 0, 0, 0, 0
DATA  0, 0, 0, 0,14, 0,14, 0, 0, 0, 0, 0, 0

' === SPRITE: EXPLOSION FRAME 2 (13x8) - fading sparks ===
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

' --------------------------------------------------------------------
' 5. LOAD SPRITES GOSUB
' --------------------------------------------------------------------
GOSUB LoadSprites

' --------------------------------------------------------------------
' 6. MAIN PROGRAM FLOW
' --------------------------------------------------------------------
SCREEN 13

' Generate stars
CALL GenStars

' Load high scores
CALL LoadHiScores

hiScore = scores(1).score
gameState = STATETITLE

DO
    SELECT CASE gameState
        CASE STATETITLE
            CALL ShowTitle
        CASE STATEPLAYING
            CALL RunGame
        CASE STATEHISCORES
            CALL ShowHiScores
        CASE STATEINITIALS
            CALL EnterInitials
        CASE STATEGAMEOVER
            CALL ShowGameOver
    END SELECT
LOOP UNTIL gameState = STATEQUIT

SCREEN 0
COLOR 7, 0
CLS
PRINT "Thanks for playing SPACE INVADERS - QB45 Port 1993"
END
' ====================================================================
' SUBs AND FUNCTIONS (alphabetical)
' ====================================================================

' ================================
' SUB AddExplosion(px%, py%)
' Spawns explosion animation at px,py
' ================================
' -----------------------------------------------------
' LoadSprites - reads DATA into sprite arrays
' Must appear between END and first SUB in QB45
' -----------------------------------------------------
LoadSprites:
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER

    ' Player ship
    FOR r = 0 TO SHIPH - 1
        FOR c = 0 TO SHIPW - 1
            READ v
            shipSprite(c, r) = v
        NEXT c
    NEXT r

    ' Invader A frames 0 and 1
    DIM f AS INTEGER
    FOR f = 0 TO 1
        FOR r = 0 TO INVAH - 1
            FOR c = 0 TO INVAW - 1
                READ v
                invASprite(f, c, r) = v
            NEXT c
        NEXT r
    NEXT f

    ' Invader B frames 0 and 1
    FOR f = 0 TO 1
        FOR r = 0 TO INVBH - 1
            FOR c = 0 TO INVBW - 1
                READ v
                invBSprite(f, c, r) = v
            NEXT c
        NEXT r
    NEXT f

    ' Invader C frames 0 and 1
    FOR f = 0 TO 1
        FOR r = 0 TO INVCH - 1
            FOR c = 0 TO INVCW - 1
                READ v
                invCSprite(f, c, r) = v
            NEXT c
        NEXT r
    NEXT f

    ' UFO
    FOR r = 0 TO UFOH - 1
        FOR c = 0 TO UFOW - 1
            READ v
            ufoSprite(c, r) = v
        NEXT c
    NEXT r

    ' Explosion frames 0,1,2
    FOR f = 0 TO 2
        FOR r = 0 TO EXPLH - 1
            FOR c = 0 TO EXPLW - 1
                READ v
                explSprite(f, c, r) = v
            NEXT c
        NEXT r
    NEXT f

    ' Player bullet
    FOR r = 0 TO PBULH - 1
        FOR c = 0 TO PBULW - 1
            READ v
            pbulSprite(c, r) = v
        NEXT c
    NEXT r

    ' Invader bullet types 0,1,2
    FOR f = 0 TO 2
        FOR r = 0 TO IBULH - 1
            FOR c = 0 TO IBULW - 1
                READ v
                ibulSprite(f, c, r) = v
            NEXT c
        NEXT r
    NEXT f

RETURN

SUB AddExplosion (px AS INTEGER, py AS INTEGER)
    DIM i AS INTEGER
    FOR i = 1 TO MAXEXPLS
        IF expl(i).active = 0 THEN
            expl(i).x = px
            expl(i).y = py
            expl(i).frame = 0
            expl(i).tmr = 0
            expl(i).active = 1
            EXIT FOR
        END IF
    NEXT i
END SUB

' ================================
' FUNCTION BoxHit%(ax,ay,aw,ah,bx,by,bw,bh)
' AABB collision test
' ================================
FUNCTION BoxHit% (ax AS INTEGER, ay AS INTEGER, aw AS INTEGER, ah AS INTEGER, bx AS INTEGER, by AS INTEGER, bw AS INTEGER, bh AS INTEGER)
    IF (ax < bx + bw) AND (ax + aw > bx) AND (ay < by + bh) AND (ay + ah > by) THEN
        BoxHit% = 1
    ELSE
        BoxHit% = 0
    END IF
END FUNCTION

' ================================
' SUB BuildBunkers
' Initialises 4 bunkers with arch shape
' ================================
SUB BuildBunkers ()
    DIM b AS INTEGER, bx AS INTEGER, col AS INTEGER, row AS INTEGER
    bnkY = 155
    FOR b = 1 TO NUMBUNKERS
        bnkX(b) = 20 + (b - 1) * 72
    NEXT b

    ' Arch shape: fill all, then cut top-centre notch
    FOR b = 1 TO NUMBUNKERS
        FOR col = 0 TO BUNKERW - 1
            FOR row = 0 TO BUNKERH - 1
                ' Cut the arch notch (bottom-centre opening)
                IF row >= BUNKERH - 6 AND col >= 7 AND col <= 14 THEN
                    bnk(b, col, row) = 0
                ELSE
                    bnk(b, col, row) = DKGREEN
                END IF
            NEXT row
        NEXT col
    NEXT b
END SUB

' ================================
' SUB CheckCollisions
' All AABB checks for a game tick
' ================================
SUB CheckCollisions ()
    DIM col AS INTEGER, row AS INTEGER
    DIM invX AS INTEGER, invY AS INTEGER
    DIM i AS INTEGER, b AS INTEGER
    DIM iw AS INTEGER, ih AS INTEGER
    DIM pts AS INTEGER
    DIM hitCol AS INTEGER
    DIM stillAlive AS INTEGER, cr AS INTEGER

    ' --- Player bullet vs invaders ---
    IF pbulActive THEN
        ' Bullet can only be in one column - compute it directly
        hitCol = (pbulX - frmX) \ INVSPACEX + 1
        IF hitCol >= 1 AND hitCol <= INVCOLS THEN
            FOR row = 1 TO INVROWS
                IF alive(hitCol, row) THEN
                    invX = frmX + (hitCol - 1) * INVSPACEX
                    invY = frmY + (row - 1) * INVSPACEY
                    IF row <= 2 THEN
                        iw = INVAW : ih = INVAH
                    ELSEIF row <= 4 THEN
                        iw = INVBW : ih = INVBH
                    ELSE
                        iw = INVCW : ih = INVCH
                    END IF
                    IF BoxHit%(pbulX, pbulY, PBULW, PBULH, invX, invY, iw, ih) THEN
                        ' Kill invader
                        alive(hitCol, row) = 0
                        aliveCount = aliveCount - 1
                        ' Update column-alive flag
                        stillAlive = 0
                        FOR cr = 1 TO INVROWS
                            IF alive(hitCol, cr) THEN stillAlive = 1 : EXIT FOR
                        NEXT cr
                        colAlive(hitCol) = stillAlive
                        ' Score
                        IF row <= 2 THEN pts = 30
                        IF row = 3 OR row = 4 THEN pts = 20
                        IF row = 5 THEN pts = 10
                        score = score + pts
                        IF score > hiScore THEN hiScore = score
                        ' Erase bullet
                        EraseSprite pbulX, pbulY, PBULW, PBULH
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
        END IF

        ' Player bullet vs UFO
        IF ufoActive THEN
            IF BoxHit%(pbulX, pbulY, PBULW, PBULH, ufoX, ufoY, UFOW, UFOH) THEN
                score = score + ufoScore
                IF score > hiScore THEN hiScore = score
                EraseSprite ufoX, ufoY, UFOW, UFOH
                ufoActive = 0
                EraseSprite pbulX, pbulY, PBULW, PBULH
                pbulActive = 0
                CALL AddExplosion(ufoX, ufoY)
                CALL SoundInvaderDeath
                GOTO DonePBulHit
            END IF
        END IF

        ' Player bullet off top
        IF pbulY < PLAYTOP THEN
            EraseSprite pbulX, pbulY, PBULW, PBULH
            pbulActive = 0
        END IF
    END IF
    DonePBulHit:

    ' --- Invader bullets vs player ---
    FOR i = 1 TO MAXIBULLETS
        IF ibul(i).active THEN
            IF BoxHit%(ibul(i).x, ibul(i).y, IBULW, IBULH, shipX, shipY, SHIPW, SHIPH) THEN
                ibul(i).active = 0
                EraseSprite ibul(i).x, ibul(i).y, IBULW, IBULH
                CALL PlayerDie
                EXIT FOR
            END IF
            ' Off screen
            IF ibul(i).y > PLAYBOT THEN
                EraseSprite ibul(i).x, ibul(i).y, IBULW, IBULH
                ibul(i).active = 0
            END IF
        END IF
    NEXT i

    ' --- Invader bottom vs player Y (instant game over) ---
    DIM botY AS INTEGER
    botY = frmY + (INVROWS - 1) * INVSPACEY + INVCH
    IF botY >= shipY THEN
        lives = 0
        gameState = STATEGAMEOVER
    END IF
END SUB

' ================================
' SUB DrawBunkers
' Full redraw of all bunker pixels
' ================================
SUB DrawBunkers ()
    DIM b AS INTEGER, col AS INTEGER, row AS INTEGER
    FOR b = 1 TO NUMBUNKERS
        FOR col = 0 TO BUNKERW - 1
            FOR row = 0 TO BUNKERH - 1
                IF bnk(b, col, row) <> 0 THEN
                    PSET (bnkX(b) + col, bnkY + row), bnk(b, col, row)
                ELSE
                    PSET (bnkX(b) + col, bnkY + row), BLACK
                END IF
            NEXT row
        NEXT col
    NEXT b
END SUB

' ================================
' SUB DrawGame
' Master draw - only changed elements
' ================================
SUB DrawGame ()
    DIM i AS INTEGER

    ' Draw player ship only when it moved or respawned
    IF shipDirty THEN
        DrawSpriteShip shipX, shipY
        shipDirty = 0
    END IF

    ' Draw player bullet
    IF pbulActive THEN
        ' Inline draw for 2x6 white bullet
        LINE (pbulX, pbulY)-(pbulX + PBULW - 1, pbulY + PBULH - 1), WHITE, BF
    END IF

    ' Draw invader bullets
    DIM bt AS INTEGER
    FOR i = 1 TO MAXIBULLETS
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
    FOR i = 1 TO MAXEXPLS
        IF expl(i).active THEN
            DrawSpriteExpl expl(i).x, expl(i).y, expl(i).frame
        END IF
    NEXT i

    ' Draw HUD
    CALL DrawHUD
END SUB

' ================================
' SUB DrawFormation
' Draw all living invaders at current formation position
' ================================
SUB DrawFormation ()
    DIM col AS INTEGER, row AS INTEGER
    DIM invX AS INTEGER, invY AS INTEGER
    FOR col = 1 TO INVCOLS
        IF colAlive(col) THEN
            FOR row = 1 TO INVROWS
                IF alive(col, row) THEN
                    invX = frmX + (col - 1) * INVSPACEX
                    invY = frmY + (row - 1) * INVSPACEY
                    IF row <= 2 THEN
                        DrawSpriteInvA invX, invY, invFrame
                    ELSEIF row <= 4 THEN
                        DrawSpriteInvB invX, invY, invFrame
                    ELSE
                        DrawSpriteInvC invX, invY, invFrame
                    END IF
                END IF
            NEXT row
        END IF
    NEXT col
END SUB

' ================================
' SUB DrawHUD
' Draws score bars top and bottom
' ================================
SUB DrawHUD ()
    IF score = lastHudScore AND lives = lastHudLives AND level = lastHudLevel THEN EXIT SUB
    lastHudScore = score
    lastHudLives = lives
    lastHudLevel = level
    ' Top bar background
    LINE (0, 0)-(SCRW - 1, 14), BLACK, BF
    ' Bottom bar background
    LINE (0, 185)-(SCRW - 1, SCRH - 1), BLACK, BF
    ' Divider line
    LINE (0, 185)-(SCRW - 1, 185), DKGREEN

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
    PrintStr "LIVES:", 2, 192, WHITE
    DIM lv AS INTEGER
    FOR lv = 1 TO lives - 1
        ' Mini ship 7x5 at bottom
        DrawMiniShip 46 + (lv - 1) * 10, 193
    NEXT lv

    ' Credit
    PrintStr "CREDIT 00", 245, 192, WHITE
END SUB

' ================================
' SUB DrawMiniShip(x%,y%)
' Tiny 7x5 player ship for lives display
' ================================
SUB DrawMiniShip (x AS INTEGER, y AS INTEGER)
    PSET (x + 3, y), BRTGREEN
    PSET (x + 2, y + 1), BRTGREEN
    PSET (x + 3, y + 1), BRTGREEN
    PSET (x + 4, y + 1), BRTGREEN
    LINE (x, y + 2)-(x + 6, y + 2), BRTGREEN
    LINE (x, y + 3)-(x + 6, y + 3), DKGREEN
    LINE (x, y + 4)-(x + 6, y + 4), DKGREEN
END SUB

' ================================
' SUB DrawSpriteArr (generic 2D flat pass)
' Used for player bullet only
' ================================
SUB DrawSpriteArr (spr() AS INTEGER, x AS INTEGER, y AS INTEGER, w AS INTEGER, h AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO h - 1
        FOR c = 0 TO w - 1
            v = spr(c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ================================
' SUB DrawSpriteExpl(x,y,frame)
' Draw explosion sprite frame
' ================================
SUB DrawSpriteExpl (x AS INTEGER, y AS INTEGER, frm AS INTEGER)
    SELECT CASE frm
        CASE 0  ' Small starburst - tight center block
            LINE (x + 4, y + 2)-(x + 8, y + 5), YELLOW, BF
            PSET (x + 6, y + 1), YELLOW
            PSET (x + 6, y + 6), YELLOW
            PSET (x + 2, y + 3), YELLOW
            PSET (x + 10, y + 4), YELLOW
        CASE 1  ' Medium burst - bright outer, white-hot center
            LINE (x + 1, y)-(x + 11, y + 7), YELLOW, BF
            LINE (x + 4, y + 2)-(x + 8, y + 5), WHITE, BF
        CASE 2  ' Fading sparks - dim remnants
            LINE (x + 3, y + 1)-(x + 9, y + 6), DKRED, BF
            LINE (x + 5, y + 3)-(x + 7, y + 4), DKGRAY, BF
    END SELECT
END SUB

' ================================
' SUB DrawSpriteIBul(x,y,bt)
' Draw invader bullet type bt (0-2)
' ================================
SUB DrawSpriteIBul (x AS INTEGER, y AS INTEGER, bt AS INTEGER)
    ' Type 2 (bt=1) is a straight cyan column - fast path
    IF bt = 1 THEN
        LINE (x + 1, y)-(x + 1, y + IBULH - 1), BRTCYAN
        EXIT SUB
    END IF
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO IBULH - 1
        FOR c = 0 TO IBULW - 1
            v = ibulSprite(bt, c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ================================
' SUB DrawSpriteInvA(x,y,frame)
' Draw Squid invader
' ================================
SUB DrawSpriteInvA (x AS INTEGER, y AS INTEGER, frm AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO INVAH - 1
        FOR c = 0 TO INVAW - 1
            v = invASprite(frm, c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ================================
' SUB DrawSpriteInvB(x,y,frame)
' Draw Crab invader
' ================================
SUB DrawSpriteInvB (x AS INTEGER, y AS INTEGER, frm AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO INVBH - 1
        FOR c = 0 TO INVBW - 1
            v = invBSprite(frm, c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ================================
' SUB DrawSpriteInvC(x,y,frame)
' Draw Octopus invader
' ================================
SUB DrawSpriteInvC (x AS INTEGER, y AS INTEGER, frm AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO INVCH - 1
        FOR c = 0 TO INVCW - 1
            v = invCSprite(frm, c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ================================
' SUB DrawSpriteShip(x,y)
' Draw player ship
' ================================
SUB DrawSpriteShip (x AS INTEGER, y AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO SHIPH - 1
        FOR c = 0 TO SHIPW - 1
            v = shipSprite(c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ================================
' SUB DrawSpriteUFO(x,y)
' Draw UFO saucer
' ================================
SUB DrawSpriteUFO (x AS INTEGER, y AS INTEGER)
    DIM r AS INTEGER, c AS INTEGER, v AS INTEGER
    FOR r = 0 TO UFOH - 1
        FOR c = 0 TO UFOW - 1
            v = ufoSprite(c, r)
            IF v <> 0 THEN PSET (x + c, y + r), v
        NEXT c
    NEXT r
END SUB

' ================================
' SUB DrawStars
' Draws the starfield (static)
' ================================
SUB DrawStars ()
    DIM i AS INTEGER
    FOR i = 1 TO 80
        PSET (starX(i), starY(i)), starC(i)
    NEXT i
END SUB

' ================================
' SUB EnterInitials
' Prompt player for 3-char initials
' ================================
SUB EnterInitials ()
    DIM charPos AS INTEGER
    DIM letters(1 TO 3) AS INTEGER
    DIM i AS INTEGER, k AS STRING
    DIM dispStr AS STRING
    DIM t AS DOUBLE
    CLS
    CALL DrawStars
    FOR i = 1 TO 3 : letters(i) = 65 : NEXT i  ' default AAA
    charPos = 1

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
        LINE (130 + (charPos - 1) * 14, 109)-(136 + (charPos - 1) * 14, 109), YELLOW

        tickCount = tickCount + 1
        k = INKEY$
        IF k = CHR$(27) THEN gameState = STATETITLE : EXIT SUB
        IF k = CHR$(32) OR k = "z" OR k = "Z" THEN
            charPos = charPos + 1
            IF charPos > 3 THEN
                initials = CHR$(letters(1)) + CHR$(letters(2)) + CHR$(letters(3))
                CALL InsertScore(initials, score, level)
                CALL SaveHiScores
                gameState = STATEHISCORES
                EXIT SUB
            END IF
        END IF
        IF k = CHR$(0) + CHR$(72) THEN ' up arrow
            letters(charPos) = letters(charPos) + 1
            IF letters(charPos) > 90 THEN letters(charPos) = 65
        END IF
        IF k = CHR$(0) + CHR$(80) THEN ' down arrow
            letters(charPos) = letters(charPos) - 1
            IF letters(charPos) < 65 THEN letters(charPos) = 90
        END IF

        ' Delay
        t = TIMER
        DO WHILE TIMER - t < .05 : LOOP
    LOOP
END SUB

' ================================
' SUB EraseSprite(x,y,w,h)
' Black fill over sprite bounds
' ================================
SUB EraseSprite (x AS INTEGER, y AS INTEGER, w AS INTEGER, h AS INTEGER)
    LINE (x, y)-(x + w - 1, y + h - 1), BLACK, BF
END SUB

' ================================
' SUB FireInvaderBullet
' Random bottom invader fires
' ================================
SUB FireInvaderBullet ()
    DIM slot AS INTEGER, i AS INTEGER
    ' Find free slot
    slot = 0
    FOR i = 1 TO MAXIBULLETS
        IF ibul(i).active = 0 THEN slot = i : EXIT FOR
    NEXT i
    IF slot = 0 THEN EXIT SUB

    ' Pick random column, find lowest alive in that col
    DIM attempts AS INTEGER, c AS INTEGER, r AS INTEGER
    DIM found AS INTEGER
    found = 0
    FOR attempts = 1 TO 20
        c = INT(RND * INVCOLS) + 1
        FOR r = INVROWS TO 1 STEP -1
            IF alive(c, r) THEN
                ibul(slot).x = frmX + (c - 1) * INVSPACEX + 4
                ibul(slot).y = frmY + (r - 1) * INVSPACEY + INVCH
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

' ================================
' SUB GenStars
' Generate 80 static stars for bg
' ================================
SUB GenStars ()
    DIM i AS INTEGER
    DIM layer AS INTEGER
    RANDOMIZE TIMER
    FOR i = 1 TO 80
        starX(i) = INT(RND * SCRW)
        starY(i) = INT(RND * SCRH)
        layer = INT(RND * 3)
        IF layer = 0 THEN starC(i) = DKGRAY
        IF layer = 1 THEN starC(i) = LTGRAY
        IF layer = 2 THEN starC(i) = WHITE
    NEXT i
END SUB

' ================================
' SUB InitGame
' Reset all game state for new game
' ================================
SUB InitGame ()
    DIM c AS INTEGER, r AS INTEGER, i AS INTEGER

    score = 0
    lives = 3
    level = 1
    lastHudScore = -1 : lastHudLives = -1 : lastHudLevel = -1
    aliveCount = INVCOLS * INVROWS
    frmX = INVSTARTX
    frmY = INVSTARTY
    frmDX = 1
    frmStep = 1
    invFrame = 0
    marchNote = 0
    pbulActive = 0
    ufoActive = 0
    ufoTimer = 0
    ibulTypeIdx = 0
    tickCount = 0

    FOR c = 1 TO INVCOLS
        FOR r = 1 TO INVROWS
            alive(c, r) = 1
        NEXT r
    NEXT c

    FOR i = 1 TO MAXIBULLETS
        ibul(i).active = 0
    NEXT i
    FOR i = 1 TO MAXEXPLS
        expl(i).active = 0
    NEXT i

    shipX = SCRW \ 2 - SHIPW \ 2
    shipY = PLAYBOT - SHIPH - 2
    shipOldX = shipX
    shipDirty = 1

    FOR c = 1 TO INVCOLS
        colAlive(c) = 1
    NEXT c

    CALL BuildBunkers
END SUB

' ================================
' SUB InitLevel
' Reset formation for new level
' ================================
SUB InitLevel ()
    DIM c AS INTEGER, r AS INTEGER, i AS INTEGER

    aliveCount = INVCOLS * INVROWS
    frmX = INVSTARTX
    frmY = INVSTARTY + (level - 1) * 4
    IF frmY > 60 THEN frmY = 60
    frmDX = 1
    invFrame = 0
    marchNote = 0

    FOR c = 1 TO INVCOLS
        FOR r = 1 TO INVROWS
            alive(c, r) = 1
        NEXT r
        colAlive(c) = 1
    NEXT c

    FOR i = 1 TO MAXIBULLETS
        IF ibul(i).active THEN
            EraseSprite ibul(i).x, ibul(i).y, IBULW, IBULH
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

' ================================
' SUB InsertScore(nm,sc,lv)
' Insert new score into sorted table
' ================================
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

' ================================
' SUB LoadHiScores
' Load or initialise HISCORES.DAT
' ================================
SUB LoadHiScores ()
    DIM i AS INTEGER
    DIM fileN AS INTEGER
    fileN = FREEFILE
    OPEN "HISCORES.DAT" FOR BINARY AS #fileN
    IF LOF(fileN) = 0 THEN CLOSE #fileN : GOTO DefaultScores
    FOR i = 1 TO 10
        GET #fileN, , scores(i)
    NEXT i
    CLOSE #fileN
    EXIT SUB
    DefaultScores:
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

' ================================
' SUB MoveFormation
' Advance invader formation one tick
' ================================
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
    FOR col = 1 TO INVCOLS
        IF colAlive(col) THEN
            FOR row = 1 TO INVROWS
                IF alive(col, row) THEN
                    invX = oldFrmX + (col - 1) * INVSPACEX
                    invY = oldFrmY + (row - 1) * INVSPACEY
                    IF row <= 2 THEN
                        iw = INVAW : ih = INVAH
                    ELSEIF row <= 4 THEN
                        iw = INVBW : ih = INVBH
                    ELSE
                        iw = INVCW : ih = INVCH
                    END IF
                    EraseSprite invX, invY, iw, ih
                END IF
            NEXT row
        END IF
    NEXT col

    frmX = frmX + frmDX * spd

    ' Find extents of living invaders
    rightmost = 0 : leftmost = SCRW
    FOR col = 1 TO INVCOLS
        IF colAlive(col) THEN
            FOR row = 1 TO INVROWS
                IF alive(col, row) THEN
                    invX = frmX + (col - 1) * INVSPACEX
                    IF invX + INVSPACEX > rightmost THEN
                        rightmost = invX + INVSPACEX
                    END IF
                    IF invX < leftmost THEN leftmost = invX
                END IF
            NEXT row
        END IF
    NEXT col

    ' Boundary hit - drop and reverse
    IF frmDX = 1 AND rightmost >= 295 THEN
        frmDX = -1
        frmY = frmY + 8
        SOUND 160, 1
    ELSEIF frmDX = -1 AND leftmost <= 5 THEN
        frmDX = 1
        frmY = frmY + 8
        SOUND 160, 1
    END IF

    ' Toggle animation frame and redraw at new position
    invFrame = 1 - invFrame
    CALL DrawFormation

    ' March sound
    SELECT CASE marchNote
        CASE 0 : SOUND 160, 1
        CASE 1 : SOUND 140, 1
        CASE 2 : SOUND 120, 1
        CASE 3 : SOUND 100, 1
    END SELECT
    marchNote = (marchNote + 1) MOD 4
END SUB

' ================================
' SUB PlayerDie
' Handle player death sequence
' ================================
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
        gameState = STATEGAMEOVER
        EXIT SUB
    END IF

    ' Respawn
    EraseSprite shipX, shipY, SHIPW, SHIPH
    shipX = SCRW \ 2 - SHIPW \ 2
    shipY = PLAYBOT - SHIPH - 2
    shipOldX = shipX
    shipDirty = 1
    DrawSpriteShip shipX, shipY

    ' Short pause
    t2 = TIMER
    DO WHILE TIMER - t2 < 2 : LOOP
END SUB

' ================================
' SUB PrintStr(s$, x%, y%, col%)
' Draw text using PRINT at scaled pos
' Uses LOCATE for simple text output
' ================================
SUB PrintStr (s AS STRING, x AS INTEGER, y AS INTEGER, col AS INTEGER)
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

' ================================
' SUB RunGame
' Main gameplay loop
' ================================
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
    CALL DrawFormation
    CALL DrawGame
    lastTime = TIMER

    DO
        curTime = TIMER
        IF curTime - lastTime >= TICKDELAY THEN
            lastTime = curTime
            tickCount = tickCount + 1

            ' Input
            k = INKEY$
            IF k = CHR$(27) THEN gameState = STATETITLE : EXIT SUB

            ' Move player
            shipOldX = shipX
            IF k = CHR$(0) + CHR$(75) THEN   ' left arrow
                shipX = shipX - 3
                IF shipX < 10 THEN shipX = 10
            END IF
            IF k = CHR$(0) + CHR$(77) THEN   ' right arrow
                shipX = shipX + 3
                IF shipX > SCRW - SHIPW - 10 THEN
                    shipX = SCRW - SHIPW - 10
                END IF
            END IF
            IF (k = " " OR k = "z" OR k = "Z") AND pbulActive = 0 THEN
                pbulActive = 1
                pbulX = shipX + SHIPW \ 2 - 1
                pbulY = shipY - PBULH
                SOUND 800, 1 : SOUND 600, 1
            END IF

            ' Erase old ship pos if moved
            IF shipX <> shipOldX THEN
                EraseSprite shipOldX, shipY, SHIPW, SHIPH
                shipDirty = 1
            END IF

            ' Move player bullet
            IF pbulActive THEN
                EraseSprite pbulX, pbulY, PBULW, PBULH
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
            FOR ib = 1 TO MAXIBULLETS
                IF ibul(ib).active THEN
                    EraseSprite ibul(ib).x, ibul(ib).y, IBULW, IBULH
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
                    ufoX = -UFOW : ufoDX = 2
                ELSE
                    ufoX = SCRW : ufoDX = -2
                END IF
                ufoY = PLAYTOP + 4
                ' Random score
                ufoScore = ufoScores(INT(RND * 5))
            END IF
            IF ufoActive THEN
                EraseSprite ufoX, ufoY, UFOW, UFOH
                ufoX = ufoX + ufoDX
                IF ufoX < -UFOW OR ufoX > SCRW THEN
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
                CALL DrawFormation
                CALL DrawGame
            END IF
        END IF
    LOOP UNTIL gameState <> STATEPLAYING
END SUB

' ================================
' SUB SaveHiScores
' Write HISCORES.DAT
' ================================
SUB SaveHiScores ()
    DIM fileN AS INTEGER, i AS INTEGER
    fileN = FREEFILE
    OPEN "HISCORES.DAT" FOR BINARY AS #fileN
    FOR i = 1 TO 10
        PUT #fileN, , scores(i)
    NEXT i
    CLOSE #fileN
END SUB

' ================================
' SUB ShowGameOver
' Game over screen then route
' ================================
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
        gameState = STATEINITIALS
    ELSE
        gameState = STATETITLE
    END IF
END SUB

' ================================
' SUB ShowHiScores
' Full hi-score display screen
' ================================
SUB ShowHiScores ()
    ' -- Draw static elements once -------------------------------------
    CLS
    CALL DrawStars

    ' Header
    PrintStr "HIGH SCORES", 96, 10, BRTRED

    ' Subheader
    PrintStr "RANK  SCORE  NAME  LVL", 48, 28, DKGRAY
    LINE (8, 36)-(312, 36), DKCYAN

    DIM rankStr(1 TO 10) AS STRING
    rankStr(1) = "1ST"  : rankStr(2) = "2ND"  : rankStr(3) = "3RD"
    rankStr(4) = "4TH"  : rankStr(5) = "5TH"  : rankStr(6) = "6TH"
    rankStr(7) = "7TH"  : rankStr(8) = "8TH"  : rankStr(9) = "9TH"
    rankStr(10) = "10TH"

    DIM i AS INTEGER
    DIM ycur AS INTEGER
    ycur = 40

    DIM scStr AS STRING
    FOR i = 1 TO 10
        ' Rank
        PrintStr rankStr(i), 16, ycur, YELLOW
        ' Score (static - not #1 flash, drawn once in neutral color)
        scStr = RIGHT$("000000" + LTRIM$(STR$(scores(i).score)), 6)
        IF i = newScoreRank THEN
            PrintStr scStr, 56, ycur, BRTGREEN
        ELSE
            PrintStr scStr, 56, ycur, WHITE
        END IF
        ' Name
        PrintStr scores(i).playerName, 136, ycur, BRTCYAN
        ' Level
        PrintStr LTRIM$(STR$(scores(i).level)), 184, ycur, LTGRAY
        ycur = ycur + 12
    NEXT i

    LINE (8, 168)-(312, 168), DKCYAN

    ' -- Animation loop - only blink #1 score and prompt --------------
    DIM flashCol AS INTEGER
    DIM lastFlash AS INTEGER
    DIM lastBlink AS INTEGER
    DIM k AS STRING
    DIM dt AS DOUBLE
    DIM flashPhase AS INTEGER
    DIM blinkPhase AS INTEGER
    lastFlash = -1 : lastBlink = -1

    DO
        ' Tick-based blink state
        tickCount = tickCount + 1
        flashPhase = (tickCount MOD 40) \ 20
        blinkPhase = (tickCount MOD 30) \ 15

        ' Redraw #1 score only when color changes
        IF flashPhase <> lastFlash THEN
            IF flashPhase = 0 THEN flashCol = BRTRED ELSE flashCol = YELLOW
            scStr = RIGHT$("000000" + LTRIM$(STR$(scores(1).score)), 6)
            PrintStr scStr, 56, 40, flashCol
            lastFlash = flashPhase
        END IF

        ' Blink prompt
        IF blinkPhase <> lastBlink THEN
            IF blinkPhase = 0 THEN
                PrintStr "PRESS FIRE TO CONTINUE", 56, 176, YELLOW
            ELSE
                LINE (56, 176)-(264, 184), BLACK, BF
            END IF
            lastBlink = blinkPhase
        END IF

        k = INKEY$
        IF k = " " OR k = "z" OR k = "Z" THEN
            newScoreRank = 0
            gameState = STATETITLE
            EXIT DO
        END IF
        IF k = CHR$(27) THEN
            gameState = STATETITLE
            EXIT DO
        END IF

        dt = TIMER
        DO WHILE TIMER - dt < .033 : LOOP
    LOOP
END SUB

' ================================
' SUB ShowTitle
' Title screen with all elements
' ================================
SUB ShowTitle ()
    ' -- Draw static elements once ------------------------------------------
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

    ' Invader parade - static labels
    DIM paradeY AS INTEGER
    paradeY = 78
    PrintStr "= 10 PTS", 64, paradeY + 1, WHITE
    PrintStr "= 20 PTS", 64, paradeY + 19, WHITE
    PrintStr "= 30 PTS", 64, paradeY + 37, WHITE
    PrintStr "= ??? PTS", 64, paradeY + 55, WHITE

    ' UFO never animates
    DrawSpriteUFO 36, paradeY + 54

    ' Copyright
    PrintStr "ORIGINALLY  1978 TAITO  QB PORT 2026", 24, 192, DKGRAY

    ' -- Animation + input loop ---------------------------------------------
    DIM t AS DOUBLE
    DIM k AS STRING
    DO
        ' Erase previous parade invader frames and draw new frame
        LINE (40, paradeY)-(40 + INVCW - 1, paradeY + INVCH - 1), BLACK, BF
        DrawSpriteInvC 40, paradeY, invFrame
        LINE (40, paradeY + 18)-(40 + INVBW - 1, paradeY + 18 + INVBH - 1), BLACK, BF
        DrawSpriteInvB 40, paradeY + 18, invFrame
        LINE (40, paradeY + 36)-(40 + INVAW - 1, paradeY + 36 + INVAH - 1), BLACK, BF
        DrawSpriteInvA 40, paradeY + 36, invFrame

        ' Blink only the prompt
        IF (tickCount MOD 30) < 15 THEN
            PrintStr "PRESS FIRE TO START", 72, 152, YELLOW
        ELSE
            LINE (72, 152)-(247, 159), BLACK, BF
        END IF

        tickCount = tickCount + 1
        invFrame = 1 - invFrame

        t = TIMER
        DO WHILE TIMER - t < .25 : LOOP

        k = INKEY$
        IF k = CHR$(27) THEN gameState = STATEQUIT : EXIT SUB
        IF k = " " OR k = "z" OR k = "Z" THEN
            gameState = STATEPLAYING : EXIT DO
        END IF
        IF k = "h" OR k = "H" THEN
            gameState = STATEHISCORES : EXIT DO
        END IF
    LOOP
END SUB

' ================================
' SUB SoundInvaderDeath
' ================================
SUB SoundInvaderDeath ()
    SOUND 400, 1 : SOUND 300, 1 : SOUND 200, 1
END SUB

' ================================
' SUB SoundLevelClear
' ================================
SUB SoundLevelClear ()
    SOUND 262, 2 : SOUND 330, 2
    SOUND 392, 2 : SOUND 523, 3
END SUB

' ================================
' SUB SoundPlayerDeath
' ================================
SUB SoundPlayerDeath ()
    SOUND 300, 2 : SOUND 200, 2
    SOUND 150, 2 : SOUND 100, 3
END SUB

' ================================
' SUB UpdateBunkerErosion
' Erode bunker pixels hit by bullets
' ================================
SUB UpdateBunkerErosion ()
    DIM b AS INTEGER, col AS INTEGER, row AS INTEGER
    DIM bx AS INTEGER, by AS INTEGER
    DIM px AS INTEGER, py AS INTEGER
    DIM ib AS INTEGER

    ' Check invader bullets vs bunkers
    DIM ibHit AS INTEGER
    FOR ib = 1 TO MAXIBULLETS
        ibHit = 0
        IF ibul(ib).active THEN
            ' Quick Y-range check - skip if bullet not near bunker row
            IF ibul(ib).y + IBULH >= bnkY AND ibul(ib).y <= bnkY + BUNKERH THEN
                FOR b = 1 TO NUMBUNKERS
                IF ibHit = 0 THEN
                    ' Quick X-range check for this bunker
                    IF ibul(ib).x + IBULW >= bnkX(b) AND ibul(ib).x <= bnkX(b) + BUNKERW THEN
                        FOR col = 0 TO BUNKERW - 1
                        IF ibHit = 0 THEN
                            FOR row = 0 TO BUNKERH - 1
                                IF ibHit = 0 AND bnk(b, col, row) <> 0 THEN
                                    bx = bnkX(b) + col
                                    by = bnkY + row
                                    IF BoxHit%(ibul(ib).x, ibul(ib).y, IBULW, IBULH, bx, by, 1, 1) THEN
                                        bnk(b, col, row) = 0
                                        PSET (bx, by), BLACK
                                        EraseSprite ibul(ib).x, ibul(ib).y, IBULW, IBULH
                                        ibul(ib).active = 0
                                        ibHit = 1
                                    END IF
                                END IF
                            NEXT row
                        END IF
                        NEXT col
                    END IF
                END IF
                NEXT b
            END IF
        END IF
    NEXT ib

    ' Check player bullet vs bunkers
    DIM pbHit AS INTEGER
    IF pbulActive THEN
        pbHit = 0
        ' Quick Y-range check
        IF pbulY + PBULH >= bnkY AND pbulY <= bnkY + BUNKERH THEN
            FOR b = 1 TO NUMBUNKERS
            IF pbHit = 0 THEN
                ' Quick X-range check for this bunker
                IF pbulX + PBULW >= bnkX(b) AND pbulX <= bnkX(b) + BUNKERW THEN
                    FOR col = 0 TO BUNKERW - 1
                    IF pbHit = 0 THEN
                        FOR row = 0 TO BUNKERH - 1
                            IF pbHit = 0 AND bnk(b, col, row) <> 0 THEN
                                bx = bnkX(b) + col
                                by = bnkY + row
                                IF BoxHit%(pbulX, pbulY, PBULW, PBULH, bx, by, 1, 1) THEN
                                    bnk(b, col, row) = 0
                                    PSET (bx, by), BLACK
                                    EraseSprite pbulX, pbulY, PBULW, PBULH
                                    pbulActive = 0
                                    pbHit = 1
                                END IF
                            END IF
                        NEXT row
                    END IF
                    NEXT col
                END IF
            END IF
            NEXT b
        END IF
    END IF
END SUB

' ================================
' SUB UpdateExplosions
' Advance explosion animation frames
' ================================
SUB UpdateExplosions ()
    DIM i AS INTEGER
    FOR i = 1 TO MAXEXPLS
        IF expl(i).active THEN
            expl(i).tmr = expl(i).tmr + 1
            IF expl(i).tmr >= 8 THEN
                expl(i).tmr = 0
                EraseSprite expl(i).x, expl(i).y, EXPLW, EXPLH
                expl(i).frame = expl(i).frame + 1
                IF expl(i).frame > 2 THEN
                    expl(i).active = 0
                END IF
            END IF
        END IF
    NEXT i
END SUB

' END OF INVADERS.BAS


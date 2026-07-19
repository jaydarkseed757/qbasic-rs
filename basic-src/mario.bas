REM  MARIO.BAS -- playable platformer, e
REM  Author: jaydarkseed757 


DEFINT A-Z

CONST SCESC = 1
CONST SCLEFT = 75
CONST SCRIGHT = 77
CONST SCUP = 72
CONST SCSPACE = 57
CONST SCP = 25                 ' P = pause
CONST SC1 = 2                  ' 1/2/3 = DEBUG world-skip from the game-over
CONST SC2 = 3                  ' screen (TEMPORARY -- remove before shipping)
CONST SC3 = 4

CONST MAXPLAT = 7              ' platform list capacity (incl. blocks/movers); nplat live count
CONST MAXEN = 3                ' enemy list capacity; nen is the live count
CONST MAXCOIN = 7              ' coin list capacity; ncoin is the live count
CONST MAXBLK = 3               ' ? block list capacity; nblk is the live count
CONST MAXMOV = 2                ' moving-platform capacity; nmov is the live count
CONST NSCREENS = 4             ' rooms per world (flip-screen chain)
CONST NWORLDS = 3              ' worlds; each clears via its last room's flagpole
CONST POLEX = 292              ' flagpole x (last room only); contact = win

DIM SHARED kd(127) AS INTEGER  ' PollKeys held-key state (see SUB PollKeys)

DIM SHARED nplat, platL(MAXPLAT), platR(MAXPLAT), platY(MAXPLAT)   ' 0=ground
DIM SHARED platBlk(MAXPLAT)    ' -1 = normal platform, else index into blk* (? block)
DIM SHARED platMov(MAXPLAT)    ' -1 = static, else index into mov* (moving platform)

' Moving platforms: one platform-list slot (movPlat) whose L/R (axis=0,
' horizontal) or Y (axis=1, vertical) is advanced every frame between
' movMin/movMax at movSpd px/frame, bouncing at each bound like enemy
' patrol. movOldL/R/Y remembers last frame's box so PlayGame can erase it
' (a plain sky fill, not the generic EraseRect -- see PlayGame for why).
DIM SHARED nmov
DIM SHARED movPlat(MAXMOV), movAxis(MAXMOV), movW(MAXMOV)
DIM SHARED movMin(MAXMOV), movMax(MAXMOV), movSpd(MAXMOV), movDir(MAXMOV)
DIM SHARED movOldL(MAXMOV), movOldR(MAXMOV), movOldY(MAXMOV)

' Enemy roster: patrol span epL..epR at box-top row eyTop, type
' (0=goomba stompable, 1=spiny unstompable), alive/squish state, and
' last-drawn bookkeeping for the eraser.
DIM SHARED nen
DIM SHARED ex(MAXEN), edir(MAXEN), elive(MAXEN), esquiT(MAXEN)
DIM SHARED epL(MAXEN), epR(MAXEN), eyTop(MAXEN), eType(MAXEN)
DIM SHARED oex(MAXEN), oedrawn(MAXEN)

' Coins (static until collected) and ? blocks (solid tops, bumpable
' from below while unspent).
DIM SHARED ncoin, coinX(MAXCOIN), coinY(MAXCOIN), coinLive(MAXCOIN)
DIM SHARED nblk, blkX(MAXBLK), blkY(MAXBLK), blkUsed(MAXBLK)

DIM SHARED curScr              ' current screen index, 0..NSCREENS-1
DIM SHARED curWorld            ' current world index, 0..NWORLDS-1
DIM SHARED hudForce            ' 1 = DrawHUD re-prints both fields next call
DIM SHARED flagY               ' flagpole flag top-y (slides down on the win)

' Per-world room cache: LoadWorld(w) parses all NSCREENS rooms of world w
' (from WORLD<n>.TXT if present, else the matching DATA fallback) into
' these arrays ONCE per world entry. LoadScreen(s) then just copies room s
' out of the cache into the active arrays below -- an in-memory flip with
' no file/DATA access, so it stays cheap on every screen edge crossed.
' Sized by the same MAXPLAT/MAXEN/MAXCOIN/MAXBLK per-room caps as before.
DIM SHARED wNplat(NSCREENS)
DIM SHARED wPlatL(NSCREENS, MAXPLAT), wPlatR(NSCREENS, MAXPLAT), wPlatY(NSCREENS, MAXPLAT)
DIM SHARED wNen(NSCREENS)
DIM SHARED wEpL(NSCREENS, MAXEN), wEpR(NSCREENS, MAXEN), wEyTop(NSCREENS, MAXEN)
DIM SHARED wEdir(NSCREENS, MAXEN), wEType(NSCREENS, MAXEN)
DIM SHARED wNcoin(NSCREENS)
DIM SHARED wCoinX(NSCREENS, MAXCOIN), wCoinY(NSCREENS, MAXCOIN)
DIM SHARED wNblk(NSCREENS)
DIM SHARED wBlkX(NSCREENS, MAXBLK), wBlkY(NSCREENS, MAXBLK)
DIM SHARED wNmov(NSCREENS)
DIM SHARED wMovAxis(NSCREENS, MAXMOV), wMovFixed(NSCREENS, MAXMOV)
DIM SHARED wMovMin(NSCREENS, MAXMOV), wMovMax(NSCREENS, MAXMOV)
DIM SHARED wMovWidth(NSCREENS, MAXMOV), wMovSpd(NSCREENS, MAXMOV)

DIM SHARED lives, score, coinCt   ' HUD state (DrawHUD needs these)
DIM SHARED hiscore             ' best score, persisted in MARIOHI.DAT
DIM SHARED playAgain           ' set by Win/LoseScreen: 1 = run another game
DIM SHARED titleQuit           ' set by StartScreen: 1 = ESC pressed on title
DIM SHARED debugSkipWorld      ' TEMPORARY: set by LoseScreen (1/2/3 key) to
                               ' jump the next game straight to that world;
                               ' -1 = no skip. Remove along with SC1/SC2/SC3.

DIM SHARED pR(12), pG(12), pB(12)                ' palette fade targets

sprN = 2340                    ' 9 sprites x (draw 130 + mask 130) ints
DIM SHARED spr(sprN)           ' variable bound -> dynamic array, off DGROUP

' Boss (MEGA GOOMBA, hardcoded room NSCREENS of the last world): 3 HP,
' shrinks a size per stomp (48px -> 32px -> 16px; size IS the health bar).
' bspr() holds the scale-3 and scale-2 goomba frames (A/B, color+mask GETs,
' built by BuildBossSprites); the 16px phase reuses spr()'s normal frames.
' Layout: s3 Acol@0 Amsk@1154 Bcol@2308 Bmsk@3462 (48x48 = 1154 ints each),
' then s2 Acol@4616 Amsk@5130 Bcol@5644 Bmsk@6158 (32x32 = 514 ints each).
DIM SHARED bossActive          ' 1 only inside the boss room
DIM SHARED bossHP, bossX, bossDir, bossSpd, bossHurtT
DIM SHARED obossX, obossTop, obossW, obossDrawn  ' last drawn box (erase)
DIM SHARED bsprN
bsprN = 6672
DIM SHARED bspr(bsprN)         ' variable bound -> dynamic, off DGROUP

CALL LoadHigh
debugSkipWorld = -1
DO
    CALL PlayGame
LOOP WHILE playAgain = 1
END

' ---- Scene15 sprite art: 9 sprites x 16 rows, 16 chars per row ----
' frames: 0=run1  1=run2  2=jump  3=goomba A  4=squished  5=goomba B
'         6=coin  7=spiny A  8=spiny B
' (append new frames at the end -- indices 3..8 are hardcoded in code)
' legend: . transparent  R red  S skin  B brown  O overalls
'         G goomba body  W white  D goomba feet  C coin gold
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
' squished goomba (stomp result)
DATA "................"
DATA "................"
DATA "................"
DATA "................"
DATA "................"
DATA "................"
DATA "................"
DATA "................"
DATA "................"
DATA "................"
DATA "..GGGGGGGGGGGG.."
DATA ".GGGGGGGGGGGGGG."
DATA "GGGGGGGGGGGGGGGG"
DATA "GGGGGGGGGGGGGGGG"
DATA "GGGGGGGGGGGGGGGG"
DATA ".GWBBWGGGGWBBWG."
' mushroom man, walk frame B (feet tucked mid-stride)
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
DATA "...DDDD..DDDD..."
DATA "..DDDDD..DDDDD.."
DATA "...DDDD..DDDD..."
DATA "................"
' coin (gold with a glint)
DATA "................"
DATA "................"
DATA "......CCCC......"
DATA ".....CCCCCC....."
DATA "....CCCCCCCC...."
DATA "....CWCCCCCC...."
DATA "....CWCCCCCC...."
DATA "....CWCCCCCC...."
DATA "....CCWCCCCC...."
DATA "....CCCWWCCC...."
DATA "....CCCCCCCC...."
DATA ".....CCCCCC....."
DATA "......CCCC......"
DATA "................"
DATA "................"
DATA "................"
' spiny, walk frame A (unstompable: white spikes)
DATA "................"
DATA "...W...WW...W..."
DATA "...WW..WW..WW..."
DATA "....WWWWWWWW...."
DATA "...RRRRRRRRRR..."
DATA "..RRRRRRRRRRRR.."
DATA ".RRWWRRRRRRWWRR."
DATA ".RWBBWRRRRWBBWR."
DATA ".RRWWRRRRRRWWRR."
DATA ".RRRRRRRRRRRRRR."
DATA "..RRRRRRRRRRRR.."
DATA "...RRRRRRRRRR..."
DATA "..DDDD....DDDD.."
DATA ".DDDDD....DDDDD."
DATA ".DDDDDD..DDDDDD."
DATA "................"
' spiny, walk frame B (feet tucked mid-stride)
DATA "................"
DATA "...W...WW...W..."
DATA "...WW..WW..WW..."
DATA "....WWWWWWWW...."
DATA "...RRRRRRRRRR..."
DATA "..RRRRRRRRRRRR.."
DATA ".RRWWRRRRRRWWRR."
DATA ".RWBBWRRRRWBBWR."
DATA ".RRWWRRRRRRWWRR."
DATA ".RRRRRRRRRRRRRR."
DATA "..RRRRRRRRRRRR.."
DATA "...RRRRRRRRRR..."
DATA "...DDDD..DDDD..."
DATA "..DDDDD..DDDDD.."
DATA "...DDDD..DDDD..."
DATA "................"

' ---- LevelData: NSCREENS rooms. Per room, five blocks in order:
'   platform count, then one  L, R, topY  triple per platform
'     (topY 176 entries are ground segments -- gaps between them are
'      pits; keep solid ground at BOTH screen edges so entering and
'      pit-death respawn are always safe)
'   enemy count, then  patrolL, patrolR, boxTopRow, dir, type  per enemy
'     (type 0 = goomba/stompable, 1 = spiny/unstompable; spawn is at the
'      dir-side patrol bound -- keep spawns >20px clear of both screen
'      edges so room entry can't collide instantly)
'   coin count, then one  x, y  pair per coin (16x16 box; coin trails
'     should trace the intended jump arc)
'   ? block count, then one  x, y  pair per block (16x16, bump from below)
'   moving-platform count, then one  axis, fixed, tMin, tMax, width, speed
'     per platform (axis 0=horizontal (fixed=topY row, travel=L),
'     1=vertical (fixed=left X, travel=topY); width must be a multiple
'     of 16; starts at tMin moving toward tMax, bounces at each bound.
'     Keep its whole travel box clear of ground/other platforms/coins --
'     its own erase is a plain sky fill, not the grazed-scenery repaint
'     everything else gets, so it doesn't know how to restore anything
'     else it might overlap)
' Geometry rules of thumb (vyQ=-18 impulse, gravity 1 qpx/f^2):
'   max reliable rise ~30px, max pit/gap ~48px, box-top row = topY-16,
'   a ground jump's head-top reaches ~y122 (so block bottoms need y+15
'   >= ~124 to be bumpable from the floor).
Scr0Data:
' room 1-1 intro: teach jump (? block), platforms, one goomba to stomp;
' a vertical lift off to the side is an optional bonus (rides up to a
' coin, doesn't block the main ground->A->B->goomba path)
DATA 3
DATA 0,319,176
DATA 76,155,146
DATA 160,239,120
DATA 1
DATA 236,272,160,1,0
DATA 4
DATA 104,126
DATA 188,100
DATA 272,150
DATA 272,44
DATA 1
DATA 40,124
DATA 1
DATA 1,264,60,130,32,1

Scr1Data:
' room 1-2 precision: two 48px pits, helper ledge, island goomba;
' coins trace the pit-2 jump arc
DATA 4
DATA 0,79,176
DATA 128,207,176
DATA 256,319,176
DATA 96,143,146
DATA 1
DATA 132,188,160,1,0
DATA 3
DATA 112,122
DATA 224,130
DATA 244,136
DATA 0
DATA 0

Scr2Data:
' room 1-3 climb: steps to a high ledge (coin payout up top), ground
' spiny guards the low route -- climb over or brave it
DATA 5
DATA 0,319,176
DATA 32,95,146
DATA 128,191,120
DATA 224,287,94
DATA 96,191,72
DATA 3
DATA 40,120,160,1,0
DATA 132,172,104,-1,0
DATA 216,284,160,-1,1
DATA 3
DATA 120,48
DATA 152,48
DATA 240,70
DATA 0
DATA 0

Scr3Data:
' room 1-4 finale: centre pit, goomba + platform goomba + ground spiny;
' flagpole at x292 wins (spiny patrol kept clear of the pole column)
DATA 4
DATA 0,159,176
DATA 208,319,176
DATA 120,199,132
DATA 240,287,146
DATA 3
DATA 40,130,160,1,0
DATA 216,256,160,-1,1
DATA 124,180,116,1,0
DATA 3
DATA 60,132
DATA 176,140
DATA 256,122
DATA 1
DATA 96,120
DATA 0

' ---- World2Data: built-in fallback for world 2 (used only if
' WORLD2.TXT is missing -- see LoadWorld). Same 4-rooms-in-order layout
' as world 1's DATA above, just one contiguous label since LoadWorld
' reads all NSCREENS rooms in a single pass, not room-by-room RESTOREs.
' Content matches the shipped WORLD2.TXT so both sources agree.
World2Data:
' room 2-1: intro to the new world -- platform pair, one goomba, a block
DATA 3
DATA 0,319,176
DATA 80,159,146
DATA 168,247,116
DATA 1
DATA 252,288,160,1,0
DATA 3
DATA 108,130
DATA 192,104
DATA 276,150
DATA 1
DATA 40,120
DATA 0

' room 2-2: tighter pits than 1-2, and the first spiny guards the ledge
DATA 4
DATA 0,72,176
DATA 120,199,176
DATA 247,319,176
DATA 88,135,146
DATA 1
DATA 124,180,160,1,1
DATA 3
DATA 104,122
DATA 216,130
DATA 236,136
DATA 0
DATA 0

' room 2-3: vertical climb, spiny mid-way forces a real route choice
DATA 5
DATA 0,319,176
DATA 24,87,146
DATA 120,183,120
DATA 216,279,94
DATA 88,183,72
DATA 3
DATA 32,112,160,1,0
DATA 124,164,104,-1,0
DATA 208,276,160,-1,0
DATA 3
DATA 112,48
DATA 144,48
DATA 232,70
DATA 0
DATA 0

' room 2-4: finale -- two spinies + a goomba around a centre pit,
' flagpole (drawn automatically: last room of any world) wins the game
DATA 4
DATA 0,152,176
DATA 200,319,176
DATA 112,191,132
DATA 232,279,146
DATA 3
DATA 32,122,160,1,1
DATA 208,248,160,-1,0
DATA 116,172,116,1,1
DATA 3
DATA 52,132
DATA 168,140
DATA 248,122
DATA 1
DATA 88,120
DATA 0

' ---- World3Data: built-in fallback for world 3 (used only if
' WORLD3.TXT is missing -- see LoadWorld). Castle/lava theme, hardest
' world: 3-1 debuts the horizontal mover as an optional bonus route
' (mirrors 1-1's vertical-lift bonus), 3-2 makes a horizontal mover
' MANDATORY across a 136px pit no jump can clear (the axis-0 code path
' was built with #15 but never exercised by real, stakes-having content
' until now), 3-3/3-4 reuse proven 1-3/2-3 and 1-4/2-4 geometry with a
' harsher enemy mix (spinies where earlier worlds had goombas).
World3Data:
' room 3-1: castle debut -- goomba, ? block, bonus horizontal mover
' (parks at x240-303,y60; doesn't block the main ground->A->B path)
DATA 3
DATA 0,319,176
DATA 72,151,148
DATA 176,255,120
DATA 1
DATA 216,252,160,1,0
DATA 4
DATA 40,150
DATA 100,124
DATA 200,96
DATA 264,40
DATA 1
DATA 16,124
DATA 1
DATA 0,60,240,272,32,1

' room 3-2: a 136px pit (80-215) -- no jump clears it, so the
' horizontal mover (fixed row 160, travels x80-215) is the only way
' across; first spiny of the world guards the landing
DATA 2
DATA 0,79,176
DATA 216,319,176
DATA 1
DATA 248,296,160,1,1
DATA 3
DATA 100,140
DATA 160,140
DATA 200,140
DATA 0
DATA 1
DATA 0,160,80,184,32,1

' room 3-3: vertical climb (geometry proven in 1-3/2-3) -- ground-level
' spiny replaces the usual goomba for a harsher first hazard
DATA 5
DATA 0,319,176
DATA 24,87,146
DATA 120,183,120
DATA 216,279,94
DATA 88,183,72
DATA 3
DATA 32,112,160,1,1
DATA 124,164,104,-1,0
DATA 208,276,160,-1,0
DATA 3
DATA 112,48
DATA 144,48
DATA 232,70
DATA 0
DATA 0

' room 3-4: finale -- two spinies + a platform goomba around the centre
' pit (geometry proven in 1-4/2-4), flagpole ends the game (last world)
DATA 4
DATA 0,159,176
DATA 208,319,176
DATA 120,199,132
DATA 240,287,146
DATA 3
DATA 40,130,160,1,1
DATA 216,256,160,-1,0
DATA 124,180,116,1,0
DATA 3
DATA 60,132
DATA 176,140
DATA 256,122
DATA 1
DATA 96,120
DATA 0

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
' PollKeys -- true held-key state from port &H60 (make/break scancodes),
' reused verbatim from PIN.BAS (proven for real-time flipper controls in
' this exact QBASIC.EXE 1.1 + DOSBox-X setup). Make codes are < 128 (key
' down); break codes are the same + 128 (key up).
' ======================================================
SUB PollKeys
    DEFINT A-Z
    DIM sc
    sc = INP(&H60)
    IF sc < 128 THEN
        kd(sc) = 1
    ELSE
        kd(sc - 128) = 0
    END IF
END SUB

' ======================================================
' ReadInput -- one PollKeys/frame, plus drain INKEY$ so the BIOS keyboard
' buffer never overflows/beeps. Reused verbatim from PIN.BAS.
'
' Note: an earlier draft of this SUB tried to be "more robust" by only
' calling PollKeys when the 8042 status port (&H64 bit 0, "output buffer
' full") was set. That's wrong and was left broken during testing: BIOS's
' own IRQ1 handler (which QBasic never disables) consumes that bit within
' microseconds of a keypress, so by the time our ~60Hz loop polls &H64
' the bit has almost always already been cleared -- PollKeys never fires
' and kd() never updates. Port &H60 (the data register) has no such race:
' it holds the last scancode persistently regardless of whether the BIOS
' already "consumed" it via its own status-gated read, so an unconditional
' read every frame is what actually works.
' ======================================================
SUB ReadInput
    DEFINT A-Z
    DIM kp$
    CALL PollKeys
    kp$ = INKEY$
END SUB

' ======================================================
' Blip -- non-blocking short jingle/stinger ("MB" = music-background: PLAY
' queues the notes to the speaker interrupt and returns immediately,
' unlike a bare blocking PLAY, which would stall the 60fps loop). Reused
' verbatim from PIN.BAS.
' ======================================================
SUB Blip (seq$)
    PLAY "MB" + seq$
END SUB

' ======================================================
' AddScore -- capped add: score is a 16-bit INTEGER, and screen-hopping
' respawn farming could otherwise overflow it (runtime error at 32767).
' ======================================================
SUB AddScore (n)
    DEFINT A-Z
    score = score + n
    IF score > 30000 THEN score = 30000
END SUB

' ======================================================
' CollectCoin -- shared coin credit for pickups and ? blocks: counter,
' 1-up at 100 (counter rolls over, classic), matching Blip.
' ======================================================
SUB CollectCoin
    DEFINT A-Z
    coinCt = coinCt + 1
    IF coinCt >= 100 THEN
        coinCt = coinCt - 100
        lives = lives + 1
        CALL Blip("T200L16O5CEGO6C")   ' 1-up fanfare
    ELSE
        CALL Blip("T255L64O6E")        ' coin ping
    END IF
END SUB

' ======================================================
' LoadHigh / SaveHigh -- one INTEGER high score in MARIOHI.DAT (game
' dir = C:\). QBasic 1.1 has no DIR$: probe by OPEN FOR BINARY (which
' creates the file if missing) and KILL the empty file we may have just
' made -- the PIN.BAS idiom.
' ======================================================
SUB LoadHigh
    DEFINT A-Z
    DIM f
    DIM nbytes AS LONG
    f = FREEFILE
    OPEN "MARIOHI.DAT" FOR BINARY AS f
    nbytes = LOF(f)
    IF nbytes >= 2 THEN GET #f, 1, hiscore
    CLOSE f
    IF nbytes = 0 THEN KILL "MARIOHI.DAT"
END SUB

SUB SaveHigh
    DEFINT A-Z
    DIM f
    f = FREEFILE
    OPEN "MARIOHI.DAT" FOR BINARY AS f
    PUT #f, 1, hiscore
    CLOSE f
END SUB

' ======================================================
' WorldFileExists -- true (1) if fileName$ already exists with real content.
' QBasic 1.1 has no DIR$: probe by OPEN FOR BINARY (creates the file if
' missing) and KILL the empty stub we may have just created -- same idiom
' as LoadHigh's MARIOHI.DAT probe (itself from PIN.BAS's TITLE.BIN check).
' ======================================================
FUNCTION WorldFileExists (fileName$)
    DEFINT A-Z
    DIM f
    DIM nbytes AS LONG
    f = FREEFILE
    OPEN fileName$ FOR BINARY AS f
    nbytes = LOF(f)
    CLOSE f
    IF nbytes = 0 THEN
        KILL fileName$
        WorldFileExists = 0
    ELSE
        WorldFileExists = 1
    END IF
END FUNCTION

' ======================================================
' LoadWorld -- parse ALL NSCREENS rooms of world w into the wNplat/wPlatL/
' etc. cache, once per world entry. Source is WORLDn.TXT (n = w+1) if it
' exists, else the matching built-in DATA fallback (Scr0Data for world 0,
' World2Data for world 1) -- so the game stays runnable with zero extra
' files. The text format mirrors the DATA layout field-for-field (see the
' LevelData comment above Scr0Data).
'
' WORLDn.TXT AUTHORING RULES (learned the hard way):
'   * Every line is data -- no comments, no BLANK lines. QBasic 1.1's
'     INPUT# reads a blank line as a null numeric field (=0), which
'     silently shifts every following count and count-driven loop out of
'     alignment (overflow / "input past end of file"). Keep it dense.
'   * DOS line endings (CRLF). A bare LF lets INPUT# glue digits across
'     lines into one giant number (overflow). Save as CRLF.
'   * Field order per room: nplat, then nplat "L,R,topY" lines; nen, then
'     nen "pL,pR,boxTop,dir,type" lines; ncoin, then ncoin "x,y" lines;
'     nblk, then nblk "x,y" lines. Commas within a line are fine.
' Both READ (DATA) and INPUT# (file) walk the same field order, so this
' is the only place that duplicates itself for the two sources.
' ======================================================
SUB LoadWorld (w)
    DEFINT A-Z
    DIM s, i, useFile, f, fileName$

    fileName$ = "WORLD" + LTRIM$(STR$(w + 1)) + ".TXT"
    useFile = WorldFileExists(fileName$)

    IF useFile THEN
        f = FREEFILE
        OPEN fileName$ FOR INPUT AS f
    ELSEIF w = 0 THEN
        RESTORE Scr0Data
    ELSEIF w = 1 THEN
        RESTORE World2Data
    ELSE
        RESTORE World3Data
    END IF

    FOR s = 0 TO NSCREENS - 1
        IF useFile THEN
            INPUT #f, wNplat(s)
            FOR i = 0 TO wNplat(s) - 1
                INPUT #f, wPlatL(s, i), wPlatR(s, i), wPlatY(s, i)
            NEXT i
            INPUT #f, wNen(s)
            FOR i = 0 TO wNen(s) - 1
                INPUT #f, wEpL(s, i), wEpR(s, i), wEyTop(s, i), wEdir(s, i), wEType(s, i)
            NEXT i
            INPUT #f, wNcoin(s)
            FOR i = 0 TO wNcoin(s) - 1
                INPUT #f, wCoinX(s, i), wCoinY(s, i)
            NEXT i
            INPUT #f, wNblk(s)
            FOR i = 0 TO wNblk(s) - 1
                INPUT #f, wBlkX(s, i), wBlkY(s, i)
            NEXT i
            INPUT #f, wNmov(s)
            FOR i = 0 TO wNmov(s) - 1
                INPUT #f, wMovAxis(s, i), wMovFixed(s, i), wMovMin(s, i), wMovMax(s, i), wMovWidth(s, i), wMovSpd(s, i)
            NEXT i
        ELSE
            READ wNplat(s)
            FOR i = 0 TO wNplat(s) - 1
                READ wPlatL(s, i), wPlatR(s, i), wPlatY(s, i)
            NEXT i
            READ wNen(s)
            FOR i = 0 TO wNen(s) - 1
                READ wEpL(s, i), wEpR(s, i), wEyTop(s, i), wEdir(s, i), wEType(s, i)
            NEXT i
            READ wNcoin(s)
            FOR i = 0 TO wNcoin(s) - 1
                READ wCoinX(s, i), wCoinY(s, i)
            NEXT i
            READ wNblk(s)
            FOR i = 0 TO wNblk(s) - 1
                READ wBlkX(s, i), wBlkY(s, i)
            NEXT i
            READ wNmov(s)
            FOR i = 0 TO wNmov(s) - 1
                READ wMovAxis(s, i), wMovFixed(s, i), wMovMin(s, i), wMovMax(s, i), wMovWidth(s, i), wMovSpd(s, i)
            NEXT i
        END IF
    NEXT s

    IF useFile THEN CLOSE f
    curWorld = w
END SUB

' ======================================================
' LoadScreen -- copy room s out of the current world's cache (populated
' by LoadWorld) into the active platform/enemy/coin/block arrays. Pure
' in-memory work, no file or DATA access -- cheap on every screen edge.
' Enemies respawn on every entry (NES-style: only the current room is
' simulated, nothing persists).
' ======================================================
SUB LoadScreen (s)
    DEFINT A-Z
    DIM i

    ' -- Boss arena: a hardcoded 5th room (index NSCREENS), reachable
    '    only off the last world's room 4 right edge. Not data-driven:
    '    the fight logic is bespoke, so there's nothing for the level
    '    editor to say about it. Flat ground, two low stepping-stone
    '    platforms, one high stomping vantage; both edges clamp. --
    IF s = NSCREENS THEN
        nplat = 4
        platL(0) = 0: platR(0) = 319: platY(0) = 176
        platL(1) = 24: platR(1) = 87: platY(1) = 146
        platL(2) = 232: platR(2) = 295: platY(2) = 146
        platL(3) = 120: platR(3) = 199: platY(3) = 116
        FOR i = 0 TO nplat - 1
            platBlk(i) = -1: platMov(i) = -1
        NEXT i
        nen = 0: ncoin = 0: nblk = 0: nmov = 0
        bossActive = 1
        bossHP = 3: bossX = 248: bossDir = -1: bossSpd = 1: bossHurtT = 0
        obossDrawn = 0
        curScr = s
        hudForce = 1
        EXIT SUB
    END IF
    bossActive = 0

    nplat = wNplat(s)
    FOR i = 0 TO nplat - 1
        platL(i) = wPlatL(s, i): platR(i) = wPlatR(s, i): platY(i) = wPlatY(s, i)
        platBlk(i) = -1: platMov(i) = -1
    NEXT i

    nen = wNen(s)
    FOR i = 0 TO nen - 1
        epL(i) = wEpL(s, i): epR(i) = wEpR(s, i): eyTop(i) = wEyTop(s, i)
        edir(i) = wEdir(s, i): eType(i) = wEType(s, i)
        IF edir(i) > 0 THEN ex(i) = epL(i) ELSE ex(i) = epR(i)
        elive(i) = 1: esquiT(i) = 0
        oex(i) = ex(i): oedrawn(i) = 0
    NEXT i

    ncoin = wNcoin(s)
    FOR i = 0 TO ncoin - 1
        coinX(i) = wCoinX(s, i): coinY(i) = wCoinY(s, i)
        coinLive(i) = 1
    NEXT i

    ' ? blocks join the platform list (their tops are landable); platBlk
    ' maps the platform entry back to its block so repaints use block art
    nblk = wNblk(s)
    FOR i = 0 TO nblk - 1
        blkX(i) = wBlkX(s, i): blkY(i) = wBlkY(s, i)
        blkUsed(i) = 0
        platL(nplat) = blkX(i): platR(nplat) = blkX(i) + 15
        platY(nplat) = blkY(i): platBlk(nplat) = i: platMov(nplat) = -1
        nplat = nplat + 1
    NEXT i

    ' moving platforms also join the platform list (so landing/one-way
    ' collision is the same code path as everything else); platMov maps
    ' the entry back to its motion state. Always starts at movMin, moving
    ' toward movMax first (deterministic; no separate start-dir field).
    nmov = wNmov(s)
    FOR i = 0 TO nmov - 1
        movAxis(i) = wMovAxis(s, i): movW(i) = wMovWidth(s, i)
        movMin(i) = wMovMin(s, i): movMax(i) = wMovMax(s, i)
        movSpd(i) = wMovSpd(s, i): movDir(i) = 1
        IF movAxis(i) = 0 THEN
            platL(nplat) = movMin(i): platR(nplat) = movMin(i) + movW(i) - 1
            platY(nplat) = wMovFixed(s, i)
        ELSE
            platL(nplat) = wMovFixed(s, i): platR(nplat) = wMovFixed(s, i) + movW(i) - 1
            platY(nplat) = movMin(i)
        END IF
        platBlk(nplat) = -1: platMov(nplat) = i
        movPlat(i) = nplat
        movOldL(i) = platL(nplat): movOldR(i) = platR(nplat): movOldY(i) = platY(nplat)
        nplat = nplat + 1
    NEXT i

    curScr = s
    hudForce = 1               ' the world redraw wipes the HUD row
END SUB

' ======================================================
' SprColor -- sprite-legend character to palette index (-1 transparent).
' Shared by BuildSprites and DrawBigSprite.
' ======================================================
FUNCTION SprColor (c$)
    DEFINT A-Z
    SELECT CASE c$
        CASE "R": SprColor = 6
        CASE "S": SprColor = 7
        CASE "B": SprColor = 8
        CASE "O": SprColor = 9
        CASE "G": SprColor = 10
        CASE "D": SprColor = 11
        CASE "W": SprColor = 5
        CASE "C": SprColor = 12
        CASE ELSE: SprColor = -1
    END SELECT
END FUNCTION

' ======================================================
' BuildSprites -- draw each SpriteData shape at the screen's top-left
' corner (invisible: palette is still blacked out), GET it as the colour
' sprite, redraw as a 255/0 mask, GET again. Same technique as the
' original scene, now 9 frames (see the SpriteData frame list).
' ======================================================
SUB BuildSprites
    DEFINT A-Z
    DIM rw$(15)
    RESTORE SpriteData
    FOR f = 0 TO 8
        FOR ry = 0 TO 15
            READ rw$(ry)
        NEXT ry
        LINE (0, 0)-(15, 15), 0, BF
        FOR ry = 0 TO 15
            FOR rx = 1 TO 16
                c = SprColor(MID$(rw$(ry), rx, 1))
                IF c >= 0 THEN PSET (rx - 1, ry), c
            NEXT rx
        NEXT ry
        GET (0, 0)-(15, 15), spr(f * 260)
        LINE (0, 0)-(15, 15), 255, BF
        FOR ry = 0 TO 15
            FOR rx = 1 TO 16
                IF MID$(rw$(ry), rx, 1) <> "." THEN PSET (rx - 1, ry), 0
            NEXT rx
        NEXT ry
        GET (0, 0)-(15, 15), spr(f * 260 + 130)
    NEXT f
END SUB

' ======================================================
' BuildBossSprites -- rasterize the goomba walk frames (3 and 5) at
' scale 3 (48x48) and scale 2 (32x32) into bspr() as color+mask GET
' pairs, exactly the BuildSprites idiom (mask: 255 = transparent,
' 0 = opaque). Called right after BuildSprites while the palette is
' still blacked out, so the staging draws never show. The boss's 16px
' final phase reuses spr()'s normal goomba frames, so only two scales
' are built here.
' ======================================================
SUB BuildBossSprites
    DEFINT A-Z
    DIM rw$(15)
    DIM sc, f, fi, ry, rx, c, x1, y1, sz, bof, imgInts
    bof = 0
    FOR sc = 3 TO 2 STEP -1
        sz = 16 * sc
        imgInts = 2 + sz * sz \ 2       ' GET size: 4-byte header + sz*sz
        FOR fi = 0 TO 1
            f = 3 + fi * 2                        ' frames 3 (A) and 5 (B)
            RESTORE SpriteData
            FOR ry = 1 TO f * 16                  ' skip to frame f's rows
                READ rw$(0)
            NEXT ry
            FOR ry = 0 TO 15
                READ rw$(ry)
            NEXT ry
            ' color image
            LINE (0, 0)-(sz - 1, sz - 1), 0, BF
            FOR ry = 0 TO 15
                FOR rx = 1 TO 16
                    c = SprColor(MID$(rw$(ry), rx, 1))
                    IF c >= 0 THEN
                        x1 = (rx - 1) * sc: y1 = ry * sc
                        LINE (x1, y1)-(x1 + sc - 1, y1 + sc - 1), c, BF
                    END IF
                NEXT rx
            NEXT ry
            GET (0, 0)-(sz - 1, sz - 1), bspr(bof)
            ' mask image (255 transparent, 0 opaque)
            LINE (0, 0)-(sz - 1, sz - 1), 255, BF
            FOR ry = 0 TO 15
                FOR rx = 1 TO 16
                    IF MID$(rw$(ry), rx, 1) <> "." THEN
                        x1 = (rx - 1) * sc: y1 = ry * sc
                        LINE (x1, y1)-(x1 + sc - 1, y1 + sc - 1), 0, BF
                    END IF
                NEXT rx
            NEXT ry
            GET (0, 0)-(sz - 1, sz - 1), bspr(bof + imgInts)
            bof = bof + imgInts * 2
        NEXT fi
    NEXT sc
END SUB

' ======================================================
' DrawBigSprite -- render one SpriteData frame scaled up, one LINE BF
' block per set pixel (the DrawText technique applied to sprite art).
' Title/menu use only: ~200 LINE BFs per call, far too slow for the
' 60fps game loop. Transparent pixels are skipped, so clear the target
' box to sky before redrawing a different frame in the same spot.
' ======================================================
SUB DrawBigSprite (f, x, y, scale)
    DEFINT A-Z
    DIM i, ry, rx, c, x1, y1, r$
    RESTORE SpriteData
    FOR i = 1 TO f * 16
        READ r$
    NEXT i
    FOR ry = 0 TO 15
        READ r$
        FOR rx = 1 TO 16
            c = SprColor(MID$(r$, rx, 1))
            IF c >= 0 THEN
                x1 = x + (rx - 1) * scale
                y1 = y + ry * scale
                LINE (x1, y1)-(x1 + scale - 1, y1 + scale - 1), c, BF
            END IF
        NEXT rx
    NEXT ry
END SUB

' ======================================================
' DrawCloud -- one cloud shape. Shared by DrawWorld and EraseRect (high
' jump arcs on rooms with tall ledges can graze the cloud boxes).
' ======================================================
SUB DrawCloud (cx, cy)
    DEFINT A-Z
    LINE (cx + 6, cy)-(cx + 21, cy + 3), 5, BF
    LINE (cx + 2, cy + 4)-(cx + 27, cy + 9), 5, BF
    LINE (cx, cy + 6)-(cx + 29, cy + 11), 5, BF
END SUB

' ======================================================
' DrawGround -- brick ground-band art (rows 176-199) for one ground
' segment, clipped to x1..x2. The mortar-joint pattern is aligned to
' global x, so adjacent segments and partial EraseRect repaints stay
' seamless. Gaps between ground segments read as pits.
' ======================================================
SUB DrawGround (x1, x2)
    DEFINT A-Z
    DIM x, y
    LINE (x1, 176)-(x2, 199), 2, BF
    LINE (x1, 176)-(x2, 176), 4                   ' lit top edge
    FOR y = 183 TO 199 STEP 8
        LINE (x1, y)-(x2, y), 3
    NEXT y
    FOR x = (x1 \ 16) * 16 TO x2 STEP 16
        IF x >= x1 AND x <= x2 THEN
            LINE (x, 177)-(x, 182), 3
            LINE (x, 192)-(x, 198), 3
        END IF
        IF x + 8 >= x1 AND x + 8 <= x2 THEN
            LINE (x + 8, 184)-(x + 8, 191), 3
        END IF
    NEXT x
END SUB

' ======================================================
' DrawPlatform -- one platform's block art (lit top / shaded right+bottom
' edges), clipped to the block(s) covering x1..x2 so an EraseRect repaint
' touches only the grazed blocks, not the whole platform (keeps worst-case
' repaint cost flat as platforms get longer). DrawWorld passes the full
' span. Driven by the platL/platR/platY tables.
' ======================================================
SUB DrawPlatform (p, x1, x2)
    DEFINT A-Z
    DIM bx, by, b1, b2
    by = platY(p)
    b1 = platL(p) + ((x1 - platL(p)) \ 16) * 16   ' block-aligned start
    IF b1 < platL(p) THEN b1 = platL(p)
    b2 = x2
    IF b2 > platR(p) THEN b2 = platR(p)
    FOR bx = b1 TO b2 STEP 16
        LINE (bx, by)-(bx + 15, by + 11), 2, BF
        LINE (bx, by)-(bx + 15, by), 4
        LINE (bx + 15, by)-(bx + 15, by + 11), 3
        LINE (bx, by + 11)-(bx + 15, by + 11), 3
    NEXT bx
END SUB

' ======================================================
' DrawMovingPlatform -- same block art as DrawPlatform (lit top / shaded
' right+bottom edges), but filled blue (index 9) instead of brick-orange
' so a moving platform reads as visually distinct at a glance. Same
' (p, x1, x2) clipped-repaint signature as DrawPlatform, used by both
' DrawWorld and EraseRect's generic grazed-scenery repaint.
' ======================================================
SUB DrawMovingPlatform (p, x1, x2)
    DEFINT A-Z
    DIM bx, by, b1, b2
    by = platY(p)
    b1 = platL(p) + ((x1 - platL(p)) \ 16) * 16
    IF b1 < platL(p) THEN b1 = platL(p)
    b2 = x2
    IF b2 > platR(p) THEN b2 = platR(p)
    FOR bx = b1 TO b2 STEP 16
        LINE (bx, by)-(bx + 15, by + 11), 9, BF
        LINE (bx, by)-(bx + 15, by), 4
        LINE (bx + 15, by)-(bx + 15, by + 11), 3
        LINE (bx, by + 11)-(bx + 15, by + 11), 3
    NEXT bx
END SUB

' ======================================================
' DrawQBlock -- one ? block: gold with a ? while unspent, dull brick
' once bumped. In the platform list via platBlk so landing and repaint
' come for free.
' ======================================================
SUB DrawQBlock (i)
    DEFINT A-Z
    DIM x, y
    x = blkX(i): y = blkY(i)
    IF blkUsed(i) = 0 THEN
        LINE (x, y)-(x + 15, y + 15), 12, BF
        LINE (x, y)-(x + 15, y), 5
        LINE (x, y)-(x, y + 15), 5
        LINE (x + 15, y)-(x + 15, y + 15), 3
        LINE (x, y + 15)-(x + 15, y + 15), 3
        CALL DrawText("?", x + 4, y + 4, 1, 3)
    ELSE
        LINE (x, y)-(x + 15, y + 15), 2, BF
        LINE (x, y)-(x + 15, y), 4
        LINE (x + 15, y)-(x + 15, y + 15), 3
        LINE (x, y + 15)-(x + 15, y + 15), 3
    END IF
END SUB

' ======================================================
' DrawFlagpole -- goal marker on the last room: ball finial, 2px pole to
' the ground, and the red pennant at the current flagY. Repainted whole
' each time (a handful of LINEs), so EraseRect and the win slide can both
' just call it. The castle backdrop is separate (static, in DrawWorld).
' ======================================================
SUB DrawFlagpole
    DEFINT A-Z
    LINE (POLEX - 2, 52)-(POLEX + 2, 56), 5, BF   ' ball finial
    LINE (POLEX, 56)-(POLEX, 175), 5              ' pole
    LINE (POLEX + 1, 56)-(POLEX + 1, 175), 3      ' pole shaded edge
    LINE (POLEX - 13, flagY)-(POLEX - 1, flagY + 9), 6, BF   ' pennant
    LINE (POLEX - 1, flagY)-(POLEX - 1, flagY + 9), 4        ' pennant hoist
END SUB

' ======================================================
' DrawWorld -- the current room's background, drawn once per room entry
' and only patched by EraseRect during play. Sky, 2 clouds, every
' platform-list entry (ground segment / block platform / ? block), the
' room's live coins, and (last room) the flagpole + castle. Title shows
' the room number.
' ======================================================
SUB DrawWorld
    DEFINT A-Z
    DIM p, i, t$

    LINE (0, 0)-(319, 199), 1, BF                 ' sky (pits show sky too)

    CALL DrawCloud(44, 28)
    CALL DrawCloud(180, 44)

    FOR p = 0 TO nplat - 1
        IF platBlk(p) >= 0 THEN
            CALL DrawQBlock(platBlk(p))
        ELSEIF platMov(p) >= 0 THEN
            CALL DrawMovingPlatform(p, platL(p), platR(p))
        ELSEIF platY(p) = 176 THEN
            CALL DrawGround(platL(p), platR(p))
        ELSE
            CALL DrawPlatform(p, platL(p), platR(p))
        END IF
    NEXT p

    FOR i = 0 TO ncoin - 1
        IF coinLive(i) = 1 THEN
            PUT (coinX(i), coinY(i)), spr(6 * 260 + 130), AND
            PUT (coinX(i), coinY(i)), spr(6 * 260), OR
        END IF
    NEXT i

    IF curScr = NSCREENS - 1 AND curWorld < NWORLDS - 1 THEN
        flagY = 58                                ' flag starts at the top
        ' castle backdrop (static; nothing overlaps it, so no repaint)
        LINE (298, 150)-(319, 175), 2, BF
        LINE (298, 150)-(319, 150), 4
        LINE (298, 146)-(301, 150), 2, BF
        LINE (305, 146)-(308, 150), 2, BF
        LINE (312, 146)-(315, 150), 2, BF
        LINE (306, 162)-(311, 175), 3, BF         ' doorway
        CALL DrawFlagpole
    END IF

    t$ = "MEGA WORLD " + LTRIM$(STR$(curWorld + 1)) + "-" + CHR$(49 + curScr)
    CALL DrawText(t$, 104, 6, 1, 5)
END SUB

' ======================================================
' EraseRect -- procedural erase: repaint sky, then, only if the erased box
' overlaps a platform's drawn art, repaint that whole platform. By this
' level's geometry a grounded actor's box always sits entirely above its
' own surface's art (art starts at surfaceY, box bottom is surfaceY-1),
' so the platform branch only fires on near-miss jumps that graze a
' platform edge. This replaces a flat-colour LINE-BF erase (which only
' worked because actor boxes were guaranteed to cover pure sky) and a
' whole-screen GET/PUT save-under buffer (ruled out: QBasic's GET/PUT
' format bakes in a fixed row-stride per capture, so a sub-rectangle
' can't be sliced back out of one big capture; a per-actor save-under
' buffer was also ruled out since it corrupts permanently the moment two
' sprites' capture/restore cycles overlap, which happens on every stomp).
' ======================================================
SUB EraseRect (x1, y1, x2, y2)
    DEFINT A-Z
    DIM p, i, py1, py2, cx1, cx2

    LINE (x1, y1)-(x2, y2), 1, BF

    ' clouds: reachable by jump arcs from the tallest ledges
    IF x2 >= 44 AND x1 <= 73 AND y2 >= 28 AND y1 <= 39 THEN CALL DrawCloud(44, 28)
    IF x2 >= 180 AND x1 <= 209 AND y2 >= 44 AND y1 <= 55 THEN CALL DrawCloud(180, 44)

    ' ground segments (topY 176) can be grazed by a player falling into a
    ' pit and drifting against its wall; floating platforms and ? blocks
    ' by near-miss jumps. Repaint only the overlapped entry.
    FOR p = 0 TO nplat - 1
        py1 = platY(p)
        IF platBlk(p) >= 0 THEN
            py2 = py1 + 15
        ELSEIF py1 = 176 THEN
            py2 = 199
        ELSE
            py2 = py1 + 11
        END IF
        IF x2 >= platL(p) AND x1 <= platR(p) AND y2 >= py1 AND y1 <= py2 THEN
            IF platBlk(p) >= 0 THEN
                CALL DrawQBlock(platBlk(p))
            ELSEIF platMov(p) >= 0 THEN
                CALL DrawMovingPlatform(p, x1, x2)
            ELSEIF py1 = 176 THEN
                cx1 = x1: IF cx1 < platL(p) THEN cx1 = platL(p)
                cx2 = x2: IF cx2 > platR(p) THEN cx2 = platR(p)
                CALL DrawGround(cx1, cx2)
                hudForce = 1   ' ground repaint can cross the HUD text row
            ELSE
                CALL DrawPlatform(p, x1, x2)
            END IF
        END IF
    NEXT p

    ' live coins are static art: repaint any the erase box grazed
    ' (enemies can patrol through coin boxes; the player can't -- an
    ' overlap collects the coin first)
    FOR i = 0 TO ncoin - 1
        IF coinLive(i) = 1 THEN
            IF x2 >= coinX(i) AND x1 <= coinX(i) + 15 AND y2 >= coinY(i) AND y1 <= coinY(i) + 15 THEN
                PUT (coinX(i), coinY(i)), spr(6 * 260 + 130), AND
                PUT (coinX(i), coinY(i)), spr(6 * 260), OR
            END IF
        END IF
    NEXT i

    ' flagpole (last room, non-final worlds): repaint if the erase box
    ' grazes the pole/flag column -- the player's box reaches the flag one
    ' frame before the win trigger fires, and this keeps that frame clean
    IF curScr = NSCREENS - 1 AND curWorld < NWORLDS - 1 THEN
        IF x2 >= POLEX - 13 AND x1 <= POLEX + 2 AND y2 >= 52 AND y1 <= 175 THEN
            CALL DrawFlagpole
        END IF
    END IF
END SUB

' ======================================================
' Overlap -- AABB test between box A and box B (inclusive coordinates).
' ======================================================
FUNCTION Overlap (ax1, ay1, ax2, ay2, bx1, by1, bx2, by2)
    DEFINT A-Z
    IF ax2 < bx1 OR ax1 > bx2 OR ay2 < by1 OR ay1 > by2 THEN
        Overlap = 0
    ELSE
        Overlap = 1
    END IF
END FUNCTION

' ======================================================
' DrawEnemy -- draw enemy i at its current position in its current pose
' (position-keyed walk frame / squish). Used by scripted beats (death
' animation) that bypass the main loop's draw section; the main loop
' keeps its own inline version for oex/oedrawn bookkeeping.
' ======================================================
SUB DrawEnemy (i)
    DEFINT A-Z
    DIM g
    IF elive(i) = 1 THEN
        IF eType(i) = 1 THEN
            IF (ex(i) \ 8) AND 1 THEN g = 8 ELSE g = 7
        ELSE
            IF (ex(i) \ 8) AND 1 THEN g = 5 ELSE g = 3
        END IF
    ELSEIF esquiT(i) > 0 THEN
        g = 4
    ELSE
        EXIT SUB
    END IF
    PUT (ex(i), eyTop(i)), spr(g * 260 + 130), AND
    PUT (ex(i), eyTop(i)), spr(g * 260), OR
END SUB

' ======================================================
' DrawBoss -- masked blit of the boss at its current size/position.
' Size is 16*bossHP (the boss shrinks per stomp; size IS the health
' display): 48/32px phases come from bspr() (see its layout comment),
' the 16px phase reuses spr()'s normal goomba frames. Walk frame A/B
' from position, same as regular enemies.
' ======================================================
SUB DrawBoss
    DEFINT A-Z
    DIM sz, top, fb, bof
    sz = 16 * bossHP: top = 176 - sz
    fb = (bossX \ 8) AND 1
    IF bossHP = 1 THEN
        IF fb = 0 THEN bof = 3 * 260 ELSE bof = 5 * 260
        PUT (bossX, top), spr(bof + 130), AND
        PUT (bossX, top), spr(bof), OR
    ELSE
        IF bossHP = 3 THEN bof = fb * 2308 ELSE bof = 4616 + fb * 1028
        PUT (bossX, top), bspr(bof + 2 + sz * sz \ 2), AND
        PUT (bossX, top), bspr(bof), OR
    END IF
END SUB

' ======================================================
' DrawHUD -- lives/score readout on the bottom text row (y 192-199, a row
' actor boxes never reach by this level's geometry), redrawn only when
' the backing value changed. Same idiom as PIN.BAS's SUB DrawScore.
' ======================================================
SUB DrawHUD
    DEFINT A-Z
    STATIC oldlives, oldscore, oldcoins

    IF hudForce = 1 THEN
        oldlives = -1: oldscore = -1: oldcoins = -1: hudForce = 0
    END IF

    IF lives <> oldlives THEN
        COLOR 5: LOCATE 25, 1: PRINT "LIVES:"; lives; " ";
        oldlives = lives
    END IF
    IF score <> oldscore THEN
        COLOR 5: LOCATE 25, 20: PRINT "SCORE:"; score; "  ";
        oldscore = score
    END IF
    IF coinCt <> oldcoins THEN
        ' col 34 + max "C: 99 " = ends col 39; never touches col 40
        ' (writing there scrolls SCREEN 13's text rows)
        COLOR 5: LOCATE 25, 34: PRINT "C:"; coinCt; " ";
        oldcoins = coinCt
    END IF
END SUB

' ======================================================
' FadeIn / FadeOut -- 64-step palette fade over entries 1-11, double-
' vblank paced. Extracted from the original scene's inline fade so start/
' win/lose screens can all reuse it.
' ======================================================
SUB FadeIn
    DEFINT A-Z
    DIM v, i
    FOR v = 0 TO 63
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 1
        FOR i = 1 TO 12
            OUT &H3C9, pR(i) * v \ 63
            OUT &H3C9, pG(i) * v \ 63
            OUT &H3C9, pB(i) * v \ 63
        NEXT i
    NEXT v
END SUB

SUB FadeOut
    DEFINT A-Z
    DIM v, i
    FOR v = 63 TO 0 STEP -1
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        OUT &H3C8, 1
        FOR i = 1 TO 12
            OUT &H3C9, pR(i) * v \ 63
            OUT &H3C9, pG(i) * v \ 63
            OUT &H3C9, pB(i) * v \ 63
        NEXT i
    NEXT v
END SUB

' ======================================================
' StartScreen -- title card: big running Mario (5x) + goomba (3x) via
' DrawBigSprite, controls, blinking PRESS SPACE. Space/up starts;
' ESC sets titleQuit for the caller. The armed latch requires a fresh
' keypress so a held space from a previous game can't skip the title.
' ======================================================
SUB StartScreen
    DEFINT A-Z
    DIM t, mf, blink, armed, t$

    titleQuit = 0
    LINE (0, 0)-(319, 199), 1, BF
    CALL DrawText("MEGA WORLD", 80, 16, 2, 5)
    CALL DrawBigSprite(0, 48, 48, 5)
    CALL DrawBigSprite(3, 224, 80, 3)
    IF hiscore > 0 THEN
        t$ = "HI" + STR$(hiscore)
        CALL DrawText(t$, (320 - LEN(t$) * 8) \ 2, 90, 1, 12)
    END IF
    CALL DrawText("ARROWS MOVE  SPACE JUMP", 68, 146, 1, 4)
    CALL DrawText("ESC QUIT  P PAUSE", 92, 158, 1, 4)
    CALL DrawText("PRESS SPACE", 116, 182, 1, 5)
    CALL FadeIn

    t = 0: mf = 0: blink = 1: armed = 0
    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        CALL ReadInput
        t = t + 1
        IF t MOD 20 = 0 THEN                 ' Mario runs in place
            mf = 1 - mf
            LINE (48, 48)-(127, 127), 1, BF
            CALL DrawBigSprite(mf, 48, 48, 5)
        END IF
        IF t MOD 30 = 0 THEN                 ' prompt blink
            blink = 1 - blink
            IF blink = 1 THEN
                CALL DrawText("PRESS SPACE", 116, 182, 1, 5)
            ELSE
                LINE (116, 182)-(203, 189), 1, BF
            END IF
        END IF
        IF kd(SCSPACE) = 0 AND kd(SCUP) = 0 AND kd(SCESC) = 0 THEN armed = 1
    LOOP UNTIL armed = 1 AND (kd(SCSPACE) = 1 OR kd(SCUP) = 1 OR kd(SCESC) = 1)
    IF kd(SCESC) = 1 THEN titleQuit = 1
    CALL FadeOut
END SUB

' ======================================================
' SetPalette -- palette fade targets for world w. Drawing code always
' uses the fixed indices 1-4 (sky/brick/mortar/highlight); only the RGB
' those indices resolve to changes per world, so a "night sky" or
' "underground" theme is just different pR/pG/pB values here -- no
' drawing-code changes needed. Indices 5-12 (characters, enemies, coins)
' are shared across worlds so actors read the same regardless of theme.
' ======================================================
SUB SetPalette (w)
    DEFINT A-Z
    IF w = 0 THEN
        pR(1) = 23: pG(1) = 37: pB(1) = 63     ' overworld sky (blue)
        pR(2) = 50: pG(2) = 19: pB(2) = 3      ' brick (orange-brown)
        pR(3) = 12: pG(3) = 5: pB(3) = 0       ' mortar (dark)
        pR(4) = 63: pG(4) = 42: pB(4) = 20     ' highlight
    ELSEIF w = 1 THEN
        pR(1) = 2: pG(1) = 2: pB(1) = 8        ' underground (near-black)
        pR(2) = 26: pG(2) = 27: pB(2) = 31     ' stone (grey-blue)
        pR(3) = 8: pG(3) = 8: pB(3) = 11       ' mortar (dark)
        pR(4) = 38: pG(4) = 40: pB(4) = 46     ' highlight
    ELSE
        pR(1) = 20: pG(1) = 2: pB(1) = 2       ' castle (dark red-black)
        pR(2) = 45: pG(2) = 10: pB(2) = 2      ' scorched brick (deep red-orange)
        pR(3) = 15: pG(3) = 2: pB(3) = 0       ' mortar (near-black)
        pR(4) = 63: pG(4) = 28: pB(4) = 4      ' highlight (lava glow)
    END IF
    pR(5) = 63: pG(5) = 63: pB(5) = 63
    pR(6) = 58: pG(6) = 0: pB(6) = 0
    pR(7) = 63: pG(7) = 40: pB(7) = 26
    pR(8) = 26: pG(8) = 11: pB(8) = 0
    pR(9) = 8: pG(9) = 16: pB(9) = 55
    pR(10) = 42: pG(10) = 22: pB(10) = 8
    pR(11) = 15: pG(11) = 6: pB(11) = 0
    pR(12) = 63: pG(12) = 52: pB(12) = 8
END SUB

' ======================================================
' WorldClearScreen -- brief transition beat between worlds (not shown
' after the LAST world's flagpole -- that goes straight to WinScreen).
' No key-wait; just a fade/hold/fade so momentum carries into the next
' world. Still drains input each wait tick to avoid a buffer-overflow
' beep on resume.
' ======================================================
SUB WorldClearScreen (w)
    DEFINT A-Z
    DIM t$, i
    LINE (0, 0)-(319, 199), 1, BF
    t$ = "WORLD " + LTRIM$(STR$(w + 1)) + " CLEAR!"
    CALL DrawText(t$, (320 - LEN(t$) * 16) \ 2, 90, 2, 5)
    CALL FadeIn
    CALL Blip("T200L8O4CEGO5CL4C")
    FOR i = 1 TO 90
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        CALL ReadInput
    NEXT i
    CALL FadeOut
END SUB

' ======================================================
' WinScreen / LoseScreen -- message + final score, fade in, jingle, then
' space/up = play again (playAgain=1) or ESC = quit (playAgain=0). The
' armed latch requires a fresh press (space is often still held from
' gameplay). Caller (PlayGame) handles SCREEN 0 when quitting for good.
' ======================================================
SUB WinScreen
    DEFINT A-Z
    DIM t$, armed
    LINE (0, 0)-(319, 199), 1, BF
    CALL DrawText("YOU WIN!", 96, 70, 2, 5)
    t$ = "SCORE" + STR$(score)
    CALL DrawText(t$, (320 - LEN(t$) * 8) \ 2, 110, 1, 5)
    IF score > hiscore THEN
        hiscore = score
        CALL SaveHigh
        CALL DrawText("NEW HIGH SCORE!", 100, 128, 1, 12)
    END IF
    CALL DrawText("SPACE PLAY AGAIN   ESC QUIT", 52, 150, 1, 4)
    CALL FadeIn
    CALL Blip("T200L8O4CEGO5C")
    armed = 0
    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        CALL ReadInput
        IF kd(SCSPACE) = 0 AND kd(SCUP) = 0 AND kd(SCESC) = 0 THEN armed = 1
    LOOP UNTIL armed = 1 AND (kd(SCSPACE) = 1 OR kd(SCUP) = 1 OR kd(SCESC) = 1)
    IF kd(SCESC) = 1 THEN playAgain = 0 ELSE playAgain = 1
    CALL FadeOut
END SUB

SUB LoseScreen
    DEFINT A-Z
    DIM t$, armed
    LINE (0, 0)-(319, 199), 1, BF
    CALL DrawText("GAME OVER", 88, 70, 2, 6)
    t$ = "SCORE" + STR$(score)
    CALL DrawText(t$, (320 - LEN(t$) * 8) \ 2, 110, 1, 5)
    IF score > hiscore THEN
        hiscore = score
        CALL SaveHigh
        CALL DrawText("NEW HIGH SCORE!", 100, 128, 1, 12)
    END IF
    CALL DrawText("SPACE PLAY AGAIN   ESC QUIT", 52, 150, 1, 4)
    CALL DrawText("1/2/3: SKIP TO WORLD (DEBUG)", 48, 166, 1, 4)
    CALL FadeIn
    CALL Blip("T120L4O3CO2GO2EO2C")
    armed = 0
    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        CALL ReadInput
        IF kd(SCSPACE) = 0 AND kd(SCUP) = 0 AND kd(SCESC) = 0 AND kd(SC1) = 0 AND kd(SC2) = 0 AND kd(SC3) = 0 THEN armed = 1
    LOOP UNTIL armed = 1 AND (kd(SCSPACE) = 1 OR kd(SCUP) = 1 OR kd(SCESC) = 1 OR kd(SC1) = 1 OR kd(SC2) = 1 OR kd(SC3) = 1)
    IF kd(SCESC) = 1 THEN
        playAgain = 0
    ELSEIF kd(SC1) = 1 THEN
        debugSkipWorld = 0: playAgain = 1
    ELSEIF kd(SC2) = 1 THEN
        debugSkipWorld = 1: playAgain = 1
    ELSEIF kd(SC3) = 1 THEN
        debugSkipWorld = 2: playAgain = 1
    ELSE
        playAgain = 1
    END IF
    CALL FadeOut
END SUB

' ======================================================
' PLAYGAME -- the platformer. A 16x16 pixel-art runner (red cap,
' overalls) is player-controlled: left/right to walk, space/up to jump
' (full air control), against a generalized one-way platform list and a
' goomba that can be stomped from above or hurts on side contact. Gravity
' stays in quarter-pixel fixed point exactly as the original scene.
' ======================================================
SUB PlayGame
    DEFINT A-Z
    DIM i

    SCREEN 13

    ' Black out the palette so sprite prep + world draw stay invisible
    OUT &H3C8, 0
    FOR i = 0 TO 255
        OUT &H3C9, 0: OUT &H3C9, 0: OUT &H3C9, 0
    NEXT i

    ' Palette targets (indices: 1 sky, 2 brick, 3 dark lines, 4 highlight,
    ' 5 white, 6 red, 7 skin, 8 brown, 9 overalls, 10 goomba body,
    ' 11 goomba feet, 12 coin/block gold) -- title always shows world 1's
    ' colours regardless of any previous playthrough's leftover state.
    CALL SetPalette(0)
    CALL LoadWorld(0)

    CALL BuildSprites
    CALL BuildBossSprites
    CALL StartScreen
    IF titleQuit = 1 THEN
        playAgain = 0
        SCREEN 0
        EXIT SUB
    END IF

    ' TEMPORARY: game-over-screen world skip (see debugSkipWorld) -- remove
    ' this block, SC1/SC2/SC3, and debugSkipWorld itself when done debugging.
    IF debugSkipWorld >= 0 THEN
        curWorld = debugSkipWorld
        CALL SetPalette(curWorld)
        CALL LoadWorld(curWorld)
        debugSkipWorld = -1
        CALL LoadScreen(0)
    ELSE
        curWorld = 0
        CALL LoadScreen(0)
    END IF
    CALL DrawWorld

    ' -- Player state --
    DIM px, feetQ, vyQ, grounded, standingOn
    DIM runF, animT, prevFeetQ, invulnT, prevJump
    DIM omx, omy, oPlayerDrawn
    px = 0: feetQ = 176 * 4: vyQ = 0: grounded = 1: standingOn = 0
    runF = 0: animT = 0: invulnT = 0
    prevJump = 1        ' require a fresh press (space may still be held from StartScreen)
    omx = px: omy = feetQ \ 4 - 16: oPlayerDrawn = 0

    lives = 10: score = 0: coinCt = 0  ' TEMPORARY debug value, revert to 3

    DIM dx, p, bestP, gFrame, f, my, playerDrawn, edrawn, exitReason, jumpHeld
    DIM flipTo, entryPx, stompBonus
    DIM startDeath, dmy, ddrawn, popX, popY, popT
    DIM dxMov, dyMov
    DIM bw, bdrawn
    exitReason = 0
    entryPx = 0                ' pit-death respawn x for the current room
    stompBonus = 100           ' consecutive-stomp chain: 100,200,400,800
    startDeath = 0: popT = 0

    CALL DrawHUD
    CALL FadeIn

    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        CALL ReadInput

        IF kd(SCESC) = 1 THEN exitReason = 3: EXIT DO

        ' -- Pause (P): freeze with actors drawn; P resumes, ESC quits.
        '    Three release/press waits give a clean edge in both
        '    directions with no latch variable. --
        IF kd(SCP) = 1 THEN
            COLOR 5: LOCATE 25, 12: PRINT "PAUSED";
            CALL Blip("T255L64O4C")
            DO
                WAIT &H3DA, 8, 8
                WAIT &H3DA, 8
                CALL ReadInput
            LOOP UNTIL kd(SCP) = 0
            DO
                WAIT &H3DA, 8, 8
                WAIT &H3DA, 8
                CALL ReadInput
            LOOP UNTIL kd(SCP) = 1 OR kd(SCESC) = 1
            DO
                WAIT &H3DA, 8, 8
                WAIT &H3DA, 8
                CALL ReadInput
            LOOP UNTIL kd(SCP) = 0
            CALL EraseRect(88, 192, 135, 199)    ' clear "PAUSED"
            hudForce = 1
            CALL DrawHUD
            ' Drop movement/jump state: PollKeys sees one scancode per
            ' frame, so a key released in the same frame P was pressed
            ' loses its break code and reads stuck. Pause is where that
            ' bites; re-pressing after resume re-sets kd() naturally.
            kd(SCLEFT) = 0: kd(SCRIGHT) = 0
            kd(SCSPACE) = 0: kd(SCUP) = 0
        END IF

        ' -- Erase all actors at their previous drawn position. Moving
        '    platforms get a plain sky fill, NOT the generic EraseRect:
        '    they're still in the platform list at their OLD box right
        '    now (advance hasn't run yet this frame), so routing through
        '    EraseRect's grazed-platform scan would see the erase box
        '    exactly overlapping this same entry and redraw it immediately
        '    -- an instant self-undo that leaves a ghost at the old spot
        '    once the platform moves on. Their travel path is kept clear
        '    of scenery by authoring convention, so a plain fill is safe. --
        IF oPlayerDrawn THEN CALL EraseRect(omx, omy, omx + 15, omy + 15)
        FOR i = 0 TO nen - 1
            IF oedrawn(i) THEN CALL EraseRect(oex(i), eyTop(i), oex(i) + 15, eyTop(i) + 15)
        NEXT i
        FOR i = 0 TO nmov - 1
            LINE (movOldL(i), movOldY(i))-(movOldR(i), movOldY(i) + 11), 1, BF
        NEXT i
        IF bossActive = 1 AND obossDrawn = 1 THEN
            CALL EraseRect(obossX, obossTop, obossX + obossW - 1, 175)
        END IF

        ' -- Enemies: patrol while alive, count down the squish pose --
        FOR i = 0 TO nen - 1
            IF elive(i) = 1 THEN
                ex(i) = ex(i) + edir(i)
                IF ex(i) >= epR(i) THEN edir(i) = -1
                IF ex(i) <= epL(i) THEN edir(i) = 1
            ELSEIF esquiT(i) > 0 THEN
                esquiT(i) = esquiT(i) - 1
            END IF
        NEXT i

        ' -- Boss: patrol the arena floor (bounds track the current
        '    size), tick down the post-stomp flicker window. Bounds stay
        '    ~24px clear of both walls so the corners are safe pockets:
        '    the player spawns/respawns at a wall, and a boss that camps
        '    it re-hits the moment invulnerability expires --
        IF bossActive = 1 THEN
            bossX = bossX + bossDir * bossSpd
            IF bossX >= 280 - 16 * bossHP THEN bossX = 280 - 16 * bossHP: bossDir = -1
            IF bossX <= 40 THEN bossX = 40: bossDir = 1
            IF bossHurtT > 0 THEN bossHurtT = bossHurtT - 1
        END IF

        ' -- Horizontal input; screen edges flip to the neighbouring room --
        prevFeetQ = feetQ

        ' -- Moving platforms: advance, bounce at bounds (same pattern as
        '    enemy patrol), then carry the player if standingOn this one.
        '    Uses the ACTUAL applied delta (new minus pre-advance old),
        '    not dir*speed recomputed after a possible bounce -- if the
        '    platform clamped at a bound this frame, dir may have just
        '    flipped, and dir*speed post-flip would carry the wrong way. --
        FOR i = 0 TO nmov - 1
            p = movPlat(i)
            IF movAxis(i) = 0 THEN
                platL(p) = platL(p) + movDir(i) * movSpd(i)
                IF platL(p) >= movMax(i) THEN platL(p) = movMax(i): movDir(i) = -1
                IF platL(p) <= movMin(i) THEN platL(p) = movMin(i): movDir(i) = 1
                platR(p) = platL(p) + movW(i) - 1
                dxMov = platL(p) - movOldL(i)
                IF grounded = 1 AND standingOn = p THEN px = px + dxMov
            ELSE
                platY(p) = platY(p) + movDir(i) * movSpd(i)
                IF platY(p) >= movMax(i) THEN platY(p) = movMax(i): movDir(i) = -1
                IF platY(p) <= movMin(i) THEN platY(p) = movMin(i): movDir(i) = 1
                dyMov = platY(p) - movOldY(i)
                IF grounded = 1 AND standingOn = p THEN feetQ = feetQ + dyMov * 4
            END IF
            movOldL(i) = platL(p): movOldR(i) = platR(p): movOldY(i) = platY(p)
        NEXT i

        dx = 0
        IF kd(SCLEFT) = 1 THEN dx = dx - 2
        IF kd(SCRIGHT) = 1 THEN dx = dx + 2
        px = px + dx
        IF px < 0 THEN
            IF curScr = NSCREENS THEN
                px = 0                    ' boss arena: locked in
            ELSEIF curScr > 0 THEN
                flipTo = curScr - 1: px = 304
                GOSUB FlipScreen
            ELSE
                px = 0
            END IF
        ELSEIF px > 304 THEN
            IF curScr = NSCREENS THEN
                px = 304                  ' boss arena: locked in
            ELSEIF curScr < NSCREENS - 1 THEN
                flipTo = curScr + 1: px = 0
                GOSUB FlipScreen
            ELSEIF curWorld = NWORLDS - 1 THEN
                flipTo = NSCREENS: px = 0 ' last world: room 4 leads to the boss
                GOSUB FlipScreen
            ELSE
                exitReason = 1: EXIT DO   ' fallback if the pole is ever removed
            END IF
        END IF

        ' -- Flagpole (last room): contact grabs the pole, runs the slide.
        '    Fires at px>=277, before px can reach the edge. Flagpole sets
        '    exitReason=1 only when this was the LAST world's pole; for an
        '    earlier world it advances curWorld/room state itself and
        '    leaves exitReason=0 so the loop just continues into world+1. --
        ' (the LAST world has no pole -- its room 4 flows into the boss room)
        IF curScr = NSCREENS - 1 AND curWorld < NWORLDS - 1 AND px + 15 >= POLEX THEN
            GOSUB Flagpole
            IF exitReason = 1 THEN EXIT DO
        END IF

        ' -- Jump (edge-triggered: a fresh press, not held-down auto-hop) --
        jumpHeld = kd(SCSPACE) OR kd(SCUP)
        IF grounded = 1 AND jumpHeld AND prevJump = 0 THEN
            grounded = 0: vyQ = -18
            CALL Blip("T255L32O3C")
        END IF
        prevJump = jumpHeld

        ' -- Walked off the edge of the platform we were standing on? --
        IF grounded = 1 THEN
            IF px + 15 < platL(standingOn) OR px > platR(standingOn) THEN
                grounded = 0: vyQ = 0
            END IF
        END IF

        ' -- Gravity + one-way landing (box-overlap, crossed-this-frame);
        '    rising instead scans for ? block head-bumps --
        IF grounded = 0 THEN
            vyQ = vyQ + 1
            feetQ = feetQ + vyQ
            IF vyQ < 0 THEN
                FOR i = 0 TO nblk - 1
                    IF blkUsed(i) = 0 THEN
                        IF px + 15 >= blkX(i) AND px <= blkX(i) + 15 THEN
                            IF prevFeetQ \ 4 - 16 >= blkY(i) + 16 AND feetQ \ 4 - 16 <= blkY(i) + 15 THEN
                                blkUsed(i) = 1
                                CALL DrawQBlock(i)
                                vyQ = 2              ' bonk: head stops, fall back
                                CALL AddScore(100)
                                CALL CollectCoin
                                popX = blkX(i): popY = blkY(i) - 16: popT = 12
                            END IF
                        END IF
                    END IF
                NEXT i
            END IF
            IF vyQ >= 0 THEN
                bestP = -1
                FOR p = 0 TO nplat - 1
                    IF px + 15 >= platL(p) AND px <= platR(p) THEN
                        IF prevFeetQ <= platY(p) * 4 AND feetQ >= platY(p) * 4 THEN
                            IF bestP = -1 THEN
                                bestP = p
                            ELSEIF platY(p) < platY(bestP) THEN
                                bestP = p
                            END IF
                        END IF
                    END IF
                NEXT p
                IF bestP >= 0 THEN
                    feetQ = platY(bestP) * 4
                    grounded = 1: vyQ = 0: standingOn = bestP
                    stompBonus = 100         ' stomp chain resets on landing
                END IF
            END IF
        END IF

        ' -- Pit check: fell past the bottom of the screen (must run
        '    before the draw section; PUT cannot clip below row 199) --
        IF feetQ \ 4 > 200 THEN
            lives = lives - 1
            CALL Blip("T120L16O2C")
            IF lives <= 0 THEN exitReason = 2: EXIT DO
            px = entryPx: feetQ = 176 * 4: vyQ = 0: grounded = 0
            invulnT = 90       ' respawn grace (room edges always have ground)
        END IF

        ' -- Enemy collision: stomp from above (goombas only -- spinies
        '    hurt from any direction), hurt on side/bottom --
        FOR i = 0 TO nen - 1
            IF elive(i) = 1 THEN
                IF Overlap(px, feetQ \ 4 - 16, px + 15, feetQ \ 4 - 1, ex(i), eyTop(i), ex(i) + 15, eyTop(i) + 15) = 1 THEN
                    IF vyQ > 0 AND prevFeetQ \ 4 <= eyTop(i) AND eType(i) = 0 THEN
                        elive(i) = 0: esquiT(i) = 15: vyQ = -10
                        CALL AddScore(stompBonus)
                        SELECT CASE stompBonus   ' pitch climbs with the chain
                            CASE 100: CALL Blip("T255L64O5C")
                            CASE 200: CALL Blip("T255L64O5E")
                            CASE 400: CALL Blip("T255L64O5G")
                            CASE ELSE: CALL Blip("T255L64O6C")
                        END SELECT
                        IF stompBonus < 800 THEN stompBonus = stompBonus + stompBonus
                    ELSEIF invulnT = 0 THEN
                        lives = lives - 1
                        IF lives <= 0 THEN
                            startDeath = 1       ' death beat runs below
                        ELSE
                            invulnT = 90
                            IF px < ex(i) THEN px = px - 10 ELSE px = px + 10
                            IF px < 0 THEN px = 0
                            IF px > 304 THEN px = 304
                            CALL Blip("T120L16O2C")
                        END IF
                    END IF
                END IF
            END IF
        NEXT i

        ' -- Boss collision: a stomp shrinks it one size (48->32->16px;
        '    the third stomp kills), then a flicker window disables boss
        '    collision BOTH ways -- without it the stomp bounce drops the
        '    player straight back into the (still overlapping) boss box
        '    for an unfair hit. Any other contact is a normal hurt. --
        IF bossActive = 1 AND bossHurtT = 0 THEN
            bw = 16 * bossHP
            IF Overlap(px, feetQ \ 4 - 16, px + 15, feetQ \ 4 - 1, bossX, 176 - bw, bossX + bw - 1, 175) = 1 THEN
                IF vyQ > 0 AND prevFeetQ \ 4 <= 176 - bw THEN
                    bossHP = bossHP - 1
                    vyQ = -10
                    IF bossHP = 0 THEN
                        ' kill: squish where it stands (frame-top erase
                        ' already cleared the old box), redraw the player
                        ' mid-bounce, fanfare, hold, then the win screen
                        PUT (bossX, 160), spr(4 * 260 + 130), AND
                        PUT (bossX, 160), spr(4 * 260), OR
                        my = feetQ \ 4 - 16
                        PUT (px, my), spr(2 * 260 + 130), AND
                        PUT (px, my), spr(2 * 260), OR
                        CALL AddScore(1000)
                        CALL DrawHUD
                        CALL Blip("T160L8O4CEGO5CL4EGO5C")
                        FOR i = 1 TO 60
                            WAIT &H3DA, 8, 8
                            WAIT &H3DA, 8
                        NEXT i
                        exitReason = 1
                        EXIT DO
                    END IF
                    CALL AddScore(500)
                    CALL Blip("T255L32O2CO3C")
                    bossHurtT = 60
                    bossSpd = bossSpd + 1
                ELSEIF invulnT = 0 THEN
                    lives = lives - 1
                    IF lives <= 0 THEN
                        startDeath = 1       ' death beat runs below
                    ELSE
                        invulnT = 90
                        IF px < bossX THEN px = px - 10 ELSE px = px + 10
                        IF px < 0 THEN px = 0
                        IF px > 304 THEN px = 304
                        CALL Blip("T120L16O2C")
                    END IF
                END IF
            END IF
        END IF
        IF invulnT > 0 THEN invulnT = invulnT - 1

        ' -- Death beat: last life lost to a hit. Mario soars in the
        '    jump pose and falls off the screen (world frozen), then
        '    game over. Pit deaths skip this -- the fall already played. --
        IF startDeath = 1 THEN
            CALL DrawHUD
            CALL Blip("T140L8O3EO3CO2GL2O2C")
            ' the frame-top erase removed every actor and the draw section
            ' won't run again: put the frozen enemies (and boss) back
            FOR i = 0 TO nen - 1
                CALL DrawEnemy(i)
            NEXT i
            IF bossActive = 1 THEN CALL DrawBoss
            vyQ = -14: ddrawn = 0: dmy = 0
            DO
                WAIT &H3DA, 8, 8
                WAIT &H3DA, 8
                IF ddrawn = 1 THEN
                    CALL EraseRect(px, dmy, px + 15, dmy + 15)
                    ' repaint any frozen enemy (or boss) the erase grazed
                    FOR i = 0 TO nen - 1
                        IF Overlap(px, dmy, px + 15, dmy + 15, ex(i), eyTop(i), ex(i) + 15, eyTop(i) + 15) = 1 THEN
                            CALL DrawEnemy(i)
                        END IF
                    NEXT i
                    IF bossActive = 1 THEN
                        bw = 16 * bossHP
                        IF Overlap(px, dmy, px + 15, dmy + 15, bossX, 176 - bw, bossX + bw - 1, 175) = 1 THEN
                            CALL DrawBoss
                        END IF
                    END IF
                END IF
                vyQ = vyQ + 1
                feetQ = feetQ + vyQ
                my = feetQ \ 4 - 16
                IF my > 199 THEN EXIT DO
                IF my <= 184 THEN                ' PUT can't clip below 199
                    PUT (px, my), spr(2 * 260 + 130), AND
                    PUT (px, my), spr(2 * 260), OR
                    dmy = my: ddrawn = 1
                ELSE
                    ddrawn = 0
                END IF
            LOOP
            exitReason = 2
            EXIT DO
        END IF

        ' -- Coin pickup (boxes overlap = collected; kill the coin before
        '    EraseRect or the eraser would faithfully repaint it) --
        FOR i = 0 TO ncoin - 1
            IF coinLive(i) = 1 THEN
                IF Overlap(px, feetQ \ 4 - 16, px + 15, feetQ \ 4 - 1, coinX(i), coinY(i), coinX(i) + 15, coinY(i) + 15) = 1 THEN
                    coinLive(i) = 0
                    CALL EraseRect(coinX(i), coinY(i), coinX(i) + 15, coinY(i) + 15)
                    CALL AddScore(50)
                    CALL CollectCoin
                END IF
            END IF
        NEXT i

        ' -- Animation frame selection (run cycle only advances while
        '    moving; standing still holds the legs-together pose) --
        IF grounded = 0 THEN
            f = 2
        ELSEIF dx <> 0 THEN
            animT = animT + 1
            IF animT >= 6 THEN animT = 0: runF = 1 - runF
            f = runF
        ELSE
            f = 1
        END IF
        my = feetQ \ 4 - 16

        playerDrawn = 1
        IF invulnT > 0 THEN
            IF (invulnT \ 3) MOD 2 = 0 THEN playerDrawn = 0
        END IF

        ' -- Bumped-coin pop effect (visual only, already credited):
        '    rises 2px/frame above its block for 12 frames --
        IF popT > 0 THEN
            CALL EraseRect(popX, popY, popX + 15, popY + 15)
            popT = popT - 1
            IF popT > 0 THEN
                popY = popY - 2
                PUT (popX, popY), spr(6 * 260 + 130), AND
                PUT (popX, popY), spr(6 * 260), OR
            END IF
        END IF

        ' -- Draw: AND-mask carves the hole, OR stamps the colours.
        '    Moving platforms, boss, enemies, player on top --
        FOR i = 0 TO nmov - 1
            CALL DrawMovingPlatform(movPlat(i), platL(movPlat(i)), platR(movPlat(i)))
        NEXT i
        IF bossActive = 1 THEN
            bdrawn = 1                     ' post-stomp flicker, 3-of-6
            IF bossHurtT > 0 THEN
                IF (bossHurtT \ 3) MOD 2 = 0 THEN bdrawn = 0
            END IF
            IF bdrawn = 1 THEN
                CALL DrawBoss
                obossX = bossX: obossW = 16 * bossHP
                obossTop = 176 - obossW: obossDrawn = 1
            ELSE
                obossDrawn = 0
            END IF
        END IF
        FOR i = 0 TO nen - 1
            IF elive(i) = 1 THEN
                ' walk animation keyed to position: flips every 8 patrol
                ' px, so enemies only animate while they actually move
                IF eType(i) = 1 THEN
                    IF (ex(i) \ 8) AND 1 THEN gFrame = 8 ELSE gFrame = 7
                ELSE
                    IF (ex(i) \ 8) AND 1 THEN gFrame = 5 ELSE gFrame = 3
                END IF
                edrawn = 1
            ELSEIF esquiT(i) > 0 THEN
                gFrame = 4: edrawn = 1
            ELSE
                edrawn = 0
            END IF
            IF edrawn THEN
                PUT (ex(i), eyTop(i)), spr(gFrame * 260 + 130), AND
                PUT (ex(i), eyTop(i)), spr(gFrame * 260), OR
            END IF
            oex(i) = ex(i): oedrawn(i) = edrawn
        NEXT i
        IF playerDrawn THEN
            PUT (px, my), spr(f * 260 + 130), AND
            PUT (px, my), spr(f * 260), OR
        END IF

        omx = px: omy = my: oPlayerDrawn = playerDrawn

        CALL DrawHUD

        IF lives <= 0 THEN exitReason = 2: EXIT DO
    LOOP

    CALL FadeOut
    playAgain = 0                        ' ESC quit (exitReason 3) stays quit
    IF exitReason = 1 THEN CALL WinScreen
    IF exitReason = 2 THEN CALL LoseScreen

    IF playAgain = 0 THEN SCREEN 0       ' replay skips the mode flip
    EXIT SUB

' -- GOSUB target: flip to room flipTo; px is already at the entry edge.
'    A GOSUB (not a SUB) because it rewrites PlayGame's loop locals. --
FlipScreen:
    CALL LoadScreen(flipTo)
    CALL DrawWorld
    CALL DrawHUD
    CALL Blip("T255L64O4E")
    oPlayerDrawn = 0           ' fresh world: nothing drawn on it yet
    popT = 0                   ' cancel any in-flight coin pop
    grounded = 0: vyQ = 0      ' re-land on the new room's surfaces
    entryPx = px               ' pit-death respawn point for this room
    FOR i = 1 TO 12            ' brief beat so the cut reads as a room change
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
    NEXT i
    RETURN

' -- GOSUB target: flagpole reached. Lock Mario to the pole, slide him
'    down while the flag lowers, land, victory jingle. Then: if this was
'    the last world, set exitReason=1 and let the caller EXIT DO (falls
'    through to FadeOut -> WinScreen); otherwise show WORLD CLEAR and
'    advance world/room state in place, leaving exitReason=0 so the main
'    loop just continues -- same DO loop, same lives/score, new world. --
Flagpole:
    px = POLEX - 10
    IF oPlayerDrawn = 1 THEN CALL EraseRect(omx, omy, omx + 15, omy + 15)
    oPlayerDrawn = 0
    CALL Blip("T160L16O5C")             ' grab
    feetQ = 96 * 4                      ' start the slide up high
    flagY = 58
    DO
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
        ' advance first, then repaint the whole pole column fresh (a flat
        ' sky strip clears last frame's flag so it doesn't smear into a
        ' vertical bar; Mario's right overhang is cleared separately)
        feetQ = feetQ + 16
        IF feetQ > 176 * 4 THEN feetQ = 176 * 4
        flagY = flagY + 5
        IF flagY > 150 THEN flagY = 150
        LINE (POLEX - 13, 52)-(POLEX + 2, 175), 1, BF
        IF oPlayerDrawn = 1 THEN CALL EraseRect(omx, omy, omx + 15, omy + 15)
        CALL DrawFlagpole
        my = feetQ \ 4 - 16
        PUT (px, my), spr(2 * 260 + 130), AND
        PUT (px, my), spr(2 * 260), OR
        omx = px: omy = my: oPlayerDrawn = 1
    LOOP UNTIL feetQ >= 176 * 4
    CALL Blip("T160L8O4CEGO5CL4EGO5C")  ' victory fanfare
    FOR i = 1 TO 40                     ' hold the tableau a beat
        WAIT &H3DA, 8, 8
        WAIT &H3DA, 8
    NEXT i

    IF curWorld >= NWORLDS - 1 THEN
        exitReason = 1
    ELSE
        CALL WorldClearScreen(curWorld)
        curWorld = curWorld + 1
        CALL SetPalette(curWorld)
        CALL LoadWorld(curWorld)
        CALL LoadScreen(0)
        CALL DrawWorld
        hudForce = 1
        CALL DrawHUD
        CALL FadeIn
        px = 0: feetQ = 176 * 4: vyQ = 0: grounded = 1: standingOn = 0
        prevJump = 1
        oPlayerDrawn = 0
        entryPx = 0
        stompBonus = 100
    END IF
    RETURN
END SUB

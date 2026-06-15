'' ============================================================
'' FARKLE.BAS - A QBasic Farkle Dice Game
'' SCREEN 13: 320x200, 256 colors
'' Two players, hot seat. First to 10000 wins!
'' ============================================================

RANDOMIZE TIMER
SCREEN 13

'' --- COLOR CONSTANTS ---
CONST BLACK = 0
CONST DKBLUE = 1
CONST DKGREEN = 2
CONST DKCYAN = 3
CONST DKRED = 4
CONST DKMAGENTA = 5
CONST BROWN = 6
CONST LTGRAY = 7
CONST DKGRAY = 8
CONST BLUE = 9
CONST GREEN = 10
CONST CYAN = 11
CONST RED = 12
CONST MAGENTA = 13
CONST YELLOW = 14
CONST WHITE = 15
CONST FELTGREEN = 34
CONST GOLD = 43
CONST CREAM = 63

'' --- GAME VARIABLES ---
DIM dice(6) AS INTEGER        '' current dice values
DIM kept(6) AS INTEGER        '' 1 = kept this roll
DIM keptAll(6) AS INTEGER     '' 1 = permanently kept this turn
DIM scores(2) AS LONG         '' total scores: scores(1), scores(2)
DIM turnScore AS LONG         '' points banked this turn
DIM rollScore AS LONG         '' points from current roll selection
DIM numDiceLeft AS INTEGER    '' dice remaining to roll
DIM currentPlayer AS INTEGER
DIM farkleCount AS INTEGER
DIM gameOver AS INTEGER
DIM winner AS INTEGER
DIM rolling AS INTEGER
DIM i AS INTEGER, j AS INTEGER
DIM k AS STRING
DIM msg AS STRING

'' Locals hoisted to module scope.  In QB1.1, GOSUB routines share the module
'' variable scope, so a DIM inside a routine that is GOSUB'd more than once would
'' re-execute and raise "Duplicate definition".  Declare them once here instead.
DIM dx AS INTEGER, dy AS INTEGER
DIM adx AS INTEGER
DIM frames AS INTEGER
DIM diceRemaining AS INTEGER
DIM cnt(6) AS INTEGER
DIM isStraight AS INTEGER
DIM pairs AS INTEGER
DIM sc(6) AS INTEGER
DIM selCount AS INTEGER
DIM allDiff AS INTEGER
DIM prs AS INTEGER
DIM flashC AS INTEGER
DIM wt AS SINGLE

'' ============================================================
'' TITLE SCREEN
'' ============================================================
GOSUB DrawTitle
SLEEP 1
k = INPUT$(1)

'' ============================================================
'' MAIN GAME LOOP
'' ============================================================
currentPlayer = 1
gameOver = 0

DO WHILE gameOver = 0

    turnScore = 0
    numDiceLeft = 6
    FOR i = 1 TO 6: keptAll(i) = 0: NEXT i

    GOSUB DrawTable
    GOSUB ShowScores
    GOSUB AnimateNewTurn

    '' --- TURN LOOP ---
    DO
        '' Reset kept flags for this roll
        FOR i = 1 TO 6: kept(i) = 0: NEXT i

        '' Roll remaining dice
        GOSUB RollDice
        GOSUB DrawTable
        GOSUB ShowScores
        GOSUB DrawDice
        GOSUB ShowTurnInfo

        '' Check for Farkle
        GOSUB CheckFarkle
        IF rolling = 0 THEN
            '' FARKLE!
            GOSUB AnimateFarkle
            turnScore = 0
            EXIT DO
        END IF

        '' Player selects dice
        GOSUB SelectDicePhase
        IF rollScore = 0 THEN
            '' No valid selection - shouldn't happen if farkle check passed
            EXIT DO
        END IF

        '' Check hot dice
        diceRemaining = 0
        FOR i = 1 TO 6
            IF keptAll(i) = 0 THEN diceRemaining = diceRemaining + 1
        NEXT i
        IF diceRemaining = 0 THEN
            '' HOT DICE!
            GOSUB AnimateHotDice
            numDiceLeft = 6
            FOR i = 1 TO 6: keptAll(i) = 0: NEXT i
        ELSE
            numDiceLeft = diceRemaining
        END IF

        '' Bank or Roll Again?
        GOSUB BankOrRoll
        IF rolling = 0 THEN EXIT DO

    LOOP

    '' Bank score
    IF turnScore > 0 THEN
        scores(currentPlayer) = scores(currentPlayer) + turnScore
    END IF

    '' Check win
    IF scores(currentPlayer) >= 10000 THEN
        gameOver = 1
        winner = currentPlayer
    END IF

    '' Switch player
    IF gameOver = 0 THEN
        IF currentPlayer = 1 THEN currentPlayer = 2 ELSE currentPlayer = 1
    END IF

LOOP

'' ============================================================
'' WIN SCREEN
'' ============================================================
GOSUB DrawWin
k = INPUT$(1)
SCREEN 0
END

'' ============================================================
'' SUBROUTINES
'' ============================================================

'' ------------------------------------------------------------
DrawTitle:
    CLS
    '' Sky gradient effect
    FOR i = 0 TO 199
        LINE (0, i)-(319, i), (i \ 14) + 1
    NEXT i

    '' Casino felt table
    LINE (10, 50)-(309, 170), FELTGREEN, BF
    LINE (10, 50)-(309, 170), GOLD, B

    '' Title text - big blocky look
    COLOR YELLOW
    LOCATE 5, 8: PRINT "  *** FARKLE! ***  "
    COLOR WHITE
    LOCATE 7, 6: PRINT " The Push-Your-Luck Dice Game"
    COLOR CREAM
    LOCATE 9, 3: PRINT " First player to 10,000 points wins!"
    COLOR LTGRAY
    LOCATE 11, 4: PRINT "  Two players, hot seat."

    '' Draw some decorative dice on title
    GOSUB DrawDecoDice

    COLOR YELLOW
    LOCATE 22, 5: PRINT "  Press any key to play..."
    COLOR DKGRAY
    LOCATE 24, 2: PRINT " (c) Farkle Game - QBasic Edition"
RETURN

'' ------------------------------------------------------------
DrawDecoDice:
    '' Draw 3 dice decoratively on title screen
    GOSUB DecoDie1
    GOSUB DecoDie3
    GOSUB DecoDie5
RETURN

DecoDie1:
    CALL DrawDieFace(60, 80, 1)
RETURN

DecoDie3:
    CALL DrawDieFace(140, 80, 3)
RETURN

DecoDie5:
    CALL DrawDieFace(220, 80, 5)
RETURN

'' ------------------------------------------------------------
DrawTable:
    CLS
    '' Background - dark felt
    LINE (0, 0)-(319, 199), 22, BF

    '' Table felt area
    LINE (0, 0)-(319, 140), FELTGREEN, BF

    '' Gold border
    LINE (2, 2)-(317, 138), GOLD, B
    LINE (3, 3)-(316, 137), GOLD, B

    '' Score panel at bottom
    LINE (0, 141)-(319, 199), 19, BF
    LINE (0, 141)-(319, 142), GOLD
    LINE (0, 143)-(319, 144), GOLD

    '' Player labels in score panel
    COLOR GOLD
    LOCATE 20, 2: PRINT "PLAYER 1:"
    LOCATE 20, 22: PRINT "PLAYER 2:"
RETURN

'' ------------------------------------------------------------
ShowScores:
    '' Player 1 score
    IF currentPlayer = 1 THEN COLOR YELLOW ELSE COLOR LTGRAY
    LOCATE 21, 2: PRINT USING "######"; scores(1)

    '' Player 2 score
    IF currentPlayer = 2 THEN COLOR YELLOW ELSE COLOR LTGRAY
    LOCATE 21, 22: PRINT USING "######"; scores(2)

    '' Current player indicator
    COLOR CYAN
    LOCATE 19, 11: PRINT "[ PLAYER "; currentPlayer; "TURN ]"

    '' Turn score
    COLOR GREEN
    LOCATE 22, 2: PRINT "Turn: "; turnScore; "  "
    LOCATE 23, 2: PRINT "Roll: "; rollScore; "  "
RETURN

'' ------------------------------------------------------------
ShowTurnInfo:
    COLOR WHITE
    LOCATE 17, 2: PRINT "Dice left: "; numDiceLeft; "  "
RETURN

'' ------------------------------------------------------------
DrawDice:
    '' Draw 6 dice at fixed positions across the table
    '' Positions: y=55 (top row area), spaced across x
    dy = 55
    FOR i = 1 TO 6
        dx = 12 + (i - 1) * 50
        IF keptAll(i) = 1 THEN
            '' Permanently kept - draw with gold border
            CALL DrawDieFaceHL(dx, dy, dice(i), GOLD)
        ELSEIF kept(i) = 1 THEN
            '' Selected this roll - cyan border
            CALL DrawDieFaceHL(dx, dy, dice(i), CYAN)
        ELSE
            CALL DrawDieFace(dx, dy, dice(i))
        END IF
        '' Number label under die
        COLOR WHITE
        LOCATE 11, (dx \ 8) + 1: PRINT i
    NEXT i
RETURN

'' ------------------------------------------------------------
RollDice:
    '' Animate rolling for dice not kept
    FOR frames = 1 TO 12
        FOR i = 1 TO 6
            IF keptAll(i) = 0 THEN
                dice(i) = INT(RND * 6) + 1
            END IF
        NEXT i
        '' Quick draw during animation
        FOR i = 1 TO 6
            adx = 12 + (i - 1) * 50
            IF keptAll(i) = 0 THEN
                CALL DrawDieFace(adx, 55, dice(i))
            END IF
        NEXT i
        '' Slow down animation near end
        IF frames > 8 THEN
            wt = TIMER + 0.08
            DO WHILE TIMER < wt: LOOP
        ELSE
            wt = TIMER + 0.03
            DO WHILE TIMER < wt: LOOP
        END IF
    NEXT frames
RETURN

'' ------------------------------------------------------------
CheckFarkle:
    '' rolling = 1 if there are scoring dice, 0 = farkle
    rolling = 0
    FOR i = 1 TO 6: cnt(i) = 0: NEXT i
    FOR i = 1 TO 6
        IF keptAll(i) = 0 THEN cnt(dice(i)) = cnt(dice(i)) + 1
    NEXT i
    '' Any 1s or 5s?
    IF cnt(1) > 0 OR cnt(5) > 0 THEN rolling = 1: RETURN
    '' Any three of a kind?
    FOR i = 1 TO 6
        IF cnt(i) >= 3 THEN rolling = 1: RETURN
    NEXT i
    '' Straight?
    isStraight = 1
    FOR i = 1 TO 6
        IF cnt(i) <> 1 THEN isStraight = 0
    NEXT i
    IF isStraight THEN rolling = 1: RETURN
    '' Three pairs?
    pairs = 0
    FOR i = 1 TO 6
        IF cnt(i) = 2 THEN pairs = pairs + 1
    NEXT i
    IF pairs = 3 THEN rolling = 1: RETURN
RETURN

'' ------------------------------------------------------------
SelectDicePhase:
    rollScore = 0
    COLOR YELLOW
    LOCATE 15, 2: PRINT "Select dice (1-6), ENTER=done, Q=bank"
    LOCATE 16, 2: PRINT "                                      "

    DO
        GOSUB DrawDice
        GOSUB ShowScores

        COLOR WHITE
        LOCATE 16, 2: PRINT "Toggle: 1-6 keys. ENTER=keep & roll  "

        k = INKEY$
        IF k = "" THEN GOTO SelectLoop

        IF k >= "1" AND k <= "6" THEN
            i = VAL(k)
            IF keptAll(i) = 0 THEN
                '' Toggle selection
                IF kept(i) = 0 THEN kept(i) = 1 ELSE kept(i) = 0
            END IF
        END IF

        IF k = CHR$(13) THEN
            '' Validate selection scores something
            rollScore = 0
            GOSUB CalcRollScore
            IF rollScore = 0 THEN
                COLOR RED
                LOCATE 16, 2: PRINT "  Must select scoring dice!       "
                wt = TIMER + 1
                DO WHILE TIMER < wt: LOOP
            ELSE
                '' Accept selection
                FOR i = 1 TO 6
                    IF kept(i) = 1 THEN keptAll(i) = 1
                NEXT i
                turnScore = turnScore + rollScore
                GOSUB ShowScores
                RETURN
            END IF
        END IF

SelectLoop:
    LOOP
RETURN

'' ------------------------------------------------------------
CalcRollScore:
    '' Score only the currently-kept (selected) dice
    rollScore = 0
    FOR i = 1 TO 6: sc(i) = 0: NEXT i
    FOR i = 1 TO 6
        IF kept(i) = 1 THEN sc(dice(i)) = sc(dice(i)) + 1
    NEXT i

    '' Check for straight (all 6 different, must have all 6 selected)
    selCount = 0
    FOR i = 1 TO 6: selCount = selCount + kept(i): NEXT i

    IF selCount = 6 THEN
        allDiff = 1
        FOR i = 1 TO 6
            IF sc(i) <> 1 THEN allDiff = 0
        NEXT i
        IF allDiff THEN rollScore = 1500: RETURN

        '' Three pairs
        prs = 0
        FOR i = 1 TO 6
            IF sc(i) = 2 THEN prs = prs + 1
        NEXT i
        IF prs = 3 THEN rollScore = 1500: RETURN
    END IF

    '' Score each face
    FOR i = 1 TO 6
        IF sc(i) >= 3 THEN
            IF i = 1 THEN
                rollScore = rollScore + 1000 * (2 ^ (sc(i) - 3))
            ELSE
                rollScore = rollScore + (i * 100) * (2 ^ (sc(i) - 3))
            END IF
            sc(i) = 0
        END IF
    NEXT i
    '' Remaining 1s and 5s
    rollScore = rollScore + sc(1) * 100
    rollScore = rollScore + sc(5) * 50
RETURN

'' ------------------------------------------------------------
BankOrRoll:
    COLOR YELLOW
    LOCATE 15, 2: PRINT "R=Roll Again  B=Bank "; turnScore; " pts   "
    LOCATE 16, 2: PRINT "                                      "
    DO
        k = UCASE$(INKEY$)
        IF k = "B" THEN rolling = 0: RETURN
        IF k = "R" THEN rolling = 1: RETURN
    LOOP
RETURN

'' ------------------------------------------------------------
AnimateNewTurn:
    '' Flash player turn banner
    FOR j = 1 TO 6
        IF j MOD 2 = 0 THEN flashC = YELLOW ELSE flashC = GOLD
        COLOR flashC
        LOCATE 10, 5: PRINT " === PLAYER "; currentPlayer; " TURN! === "
        wt = TIMER + 0.15
        DO WHILE TIMER < wt: LOOP
    NEXT j
    COLOR BLACK
    LOCATE 10, 5: PRINT "                          "
RETURN

'' ------------------------------------------------------------
AnimateFarkle:
    '' Red flash + FARKLE text
    FOR j = 1 TO 8
        IF j MOD 2 = 0 THEN
            LINE (20, 60)-(299, 120), RED, BF
            COLOR YELLOW
        ELSE
            LINE (20, 60)-(299, 120), 52, BF
            COLOR WHITE
        END IF
        LOCATE 8, 6: PRINT "  **** F A R K L E ! ****  "
        LOCATE 9, 6: PRINT "    You lose your turn!    "
        wt = TIMER + 0.2
        DO WHILE TIMER < wt: LOOP
    NEXT j
    wt = TIMER + 1.5
    DO WHILE TIMER < wt: LOOP
RETURN

'' ------------------------------------------------------------
AnimateHotDice:
    '' Gold flash + HOT DICE text
    FOR j = 1 TO 8
        IF j MOD 2 = 0 THEN
            LINE (20, 60)-(299, 120), GOLD, BF
            COLOR RED
        ELSE
            LINE (20, 60)-(299, 120), BROWN, BF
            COLOR YELLOW
        END IF
        LOCATE 8, 5: PRINT "  *** H O T  D I C E ! ***  "
        LOCATE 9, 5: PRINT "   All 6 scored! Roll again! "
        wt = TIMER + 0.2
        DO WHILE TIMER < wt: LOOP
    NEXT j
    wt = TIMER + 1
    DO WHILE TIMER < wt: LOOP
RETURN

'' ------------------------------------------------------------
DrawWin:
    CLS
    '' Fancy win screen
    FOR i = 0 TO 199
        LINE (0, i)-(319, i), (i \ 8) + 32
    NEXT i

    '' Confetti effect
    FOR j = 1 TO 200
        PSET (INT(RND * 320), INT(RND * 200)), INT(RND * 14) + 1
    NEXT j

    '' Box
    LINE (30, 40)-(289, 160), 19, BF
    LINE (30, 40)-(289, 160), GOLD, B
    LINE (32, 42)-(287, 158), GOLD, B

    COLOR YELLOW
    LOCATE 7, 7:  PRINT "*** CONGRATULATIONS! ***"
    COLOR WHITE
    LOCATE 9, 8:  PRINT "PLAYER "; winner; " WINS THE GAME!"
    COLOR GREEN
    LOCATE 11, 6: PRINT "Final Score:"; scores(winner); " pts"
    COLOR CYAN
    LOCATE 13, 4: PRINT "Player 1:"; scores(1); "  Player 2:"; scores(2)
    COLOR LTGRAY
    LOCATE 18, 6: PRINT "Press any key to exit..."
RETURN

'' ============================================================
'' DRAW DIE FACE SUB (callable)
'' x, y = top-left of die, pips = pip value
'' ============================================================
SUB DrawDieFace (x AS INTEGER, y AS INTEGER, pips AS INTEGER)
    '' Die body - white with shadow
    LINE (x + 2, y + 2)-(x + 38, y + 38), DKGRAY, BF  '' shadow
    LINE (x, y)-(x + 36, y + 36), WHITE, BF
    LINE (x, y)-(x + 36, y + 36), LTGRAY, B

    '' Draw pips based on value
    CALL DrawPips(x, y, pips, BLACK)
END SUB

SUB DrawDieFaceHL (x AS INTEGER, y AS INTEGER, pips AS INTEGER, borderCol AS INTEGER)
    LINE (x + 2, y + 2)-(x + 38, y + 38), DKGRAY, BF
    LINE (x, y)-(x + 36, y + 36), WHITE, BF
    LINE (x, y)-(x + 36, y + 36), borderCol, B
    LINE (x + 1, y + 1)-(x + 35, y + 35), borderCol, B
    CALL DrawPips(x, y, pips, BLACK)
END SUB

SUB DrawPips (x AS INTEGER, y AS INTEGER, pips AS INTEGER, col AS INTEGER)
    '' Pip positions relative to die top-left (die is 36x36)
    '' Grid: TL=8,8  TC=18,8  TR=28,8
    ''        ML=8,18 MC=18,18 MR=28,18
    ''        BL=8,28 BC=18,28 BR=28,28

    SELECT CASE pips
        CASE 1
            CALL Pip(x + 18, y + 18, col)
        CASE 2
            CALL Pip(x + 10, y + 10, col)
            CALL Pip(x + 26, y + 26, col)
        CASE 3
            CALL Pip(x + 10, y + 10, col)
            CALL Pip(x + 18, y + 18, col)
            CALL Pip(x + 26, y + 26, col)
        CASE 4
            CALL Pip(x + 10, y + 10, col)
            CALL Pip(x + 26, y + 10, col)
            CALL Pip(x + 10, y + 26, col)
            CALL Pip(x + 26, y + 26, col)
        CASE 5
            CALL Pip(x + 10, y + 10, col)
            CALL Pip(x + 26, y + 10, col)
            CALL Pip(x + 18, y + 18, col)
            CALL Pip(x + 10, y + 26, col)
            CALL Pip(x + 26, y + 26, col)
        CASE 6
            CALL Pip(x + 10, y + 10, col)
            CALL Pip(x + 26, y + 10, col)
            CALL Pip(x + 10, y + 18, col)
            CALL Pip(x + 26, y + 18, col)
            CALL Pip(x + 10, y + 26, col)
            CALL Pip(x + 26, y + 26, col)
    END SELECT
END SUB

SUB Pip (cx AS INTEGER, cy AS INTEGER, col AS INTEGER)
    '' Draw a filled circle pip (radius 3)
    CIRCLE (cx, cy), 3, col
    PAINT (cx, cy), col, col
END SUB

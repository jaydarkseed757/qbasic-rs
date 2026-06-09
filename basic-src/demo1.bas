' ==========================================
' QBASIC DEMO SCENE STYLE INTRO
' SCREEN 13 VGA DEMO
' ==========================================

DECLARE SUB InitStars ()
DECLARE SUB UpdateStars ()
DECLARE SUB DrawScroller ()
DECLARE SUB InitSprite ()
DECLARE SUB UpdateSprite ()
DECLARE SUB DrawSprite ()

SCREEN 13
RANDOMIZE TIMER

CONST NumStars = 200

DIM SHARED SX(NumStars)
DIM SHARED SY(NumStars)
DIM SHARED SZ(NumStars)

DIM SHARED ScrollText$
DIM SHARED ScrollPos

DIM SHARED BX, BY, BDX, BDY

ScrollText$ = " *** GREETINGS FROM QBASIC 4.5 *** HELLO TO ALL DOS CODERS *** LONG LIVE VGA MODE 13H *** THIS IS A RETRO DEMOSCENE STYLE INTRO WRITTEN IN PURE QBASIC *** "
ScrollPos = 1

InitStars
InitSprite

DO

    CLS

    UpdateStars
    UpdateSprite
    DrawSprite

    DrawScroller

    FOR WaitLoop = 1 TO 3000
    NEXT

LOOP UNTIL INKEY$ <> ""

SCREEN 0
WIDTH 80
CLS
END

' ==========================================
' SPRITE
' ==========================================

SUB InitSprite
    BX = 80
    BY = 60
    BDX = 2.1
    BDY = 1.7
END SUB

SUB UpdateSprite
    BX = BX + BDX
    BY = BY + BDY

    IF BX < 10 THEN BX = 10: BDX = -BDX
    IF BX > 310 THEN BX = 310: BDX = -BDX
    IF BY < 10 THEN BY = 10: BDY = -BDY
    IF BY > 145 THEN BY = 145: BDY = -BDY
END SUB

SUB DrawSprite
    R = 9

    ' Pulsing outer color — cycles through warm palette
    SpriteColor = 128 + INT(60 * SIN(TIMER * 3))
    IF SpriteColor < 1 THEN SpriteColor = 1
    IF SpriteColor > 255 THEN SpriteColor = 255

    ' Mid-ring color
    MidColor = 160 + INT(60 * SIN(TIMER * 3 + 1))
    IF MidColor < 1 THEN MidColor = 1
    IF MidColor > 255 THEN MidColor = 255

    ' Filled diamond using horizontal line spans
    FOR DY = -R TO R
        W = R - ABS(DY)
        C = SpriteColor
        IF ABS(DY) < 5 THEN C = MidColor
        LINE (BX - W, BY + DY)-(BX + W, BY + DY), C
    NEXT DY

    ' Bright white core
    LINE (BX - 1, BY)-(BX + 1, BY), 15
    PSET (BX, BY - 1), 15
    PSET (BX, BY + 1), 15
    PSET (BX, BY), 15
END SUB

' ==========================================
' SCROLLER
' ==========================================

SUB DrawScroller

    Msg$ = MID$(ScrollText$ + ScrollText$, ScrollPos, 40)

    FOR I = 1 TO LEN(Msg$)

        Ch$ = MID$(Msg$, I, 1)

        XPos = (I - 1) * 8

        YPos = 160 + SIN((I * 10 + TIMER * 80) / 20) * 12

        ColorVal = 128 + INT(100 * SIN((I * 8 + TIMER * 50) / 20))

        IF ColorVal < 1 THEN ColorVal = 1
        IF ColorVal > 255 THEN ColorVal = 255

        COLOR ColorVal

        ' Draw actual character
        LOCATE INT(YPos / 8) + 1, INT(XPos / 8) + 1
        PRINT Ch$;

    NEXT

    ScrollPos = ScrollPos + 1

    IF ScrollPos > LEN(ScrollText$) THEN
        ScrollPos = 1
    END IF

END SUB

' ==========================================
' STARS
' ==========================================

SUB InitStars

    FOR I = 1 TO NumStars

        SX(I) = RND * 320 - 160
        SY(I) = RND * 200 - 100
        SZ(I) = RND * 255 + 1

    NEXT

END SUB

SUB UpdateStars

    FOR I = 1 TO NumStars

        SZ(I) = SZ(I) - 4

        IF SZ(I) < 1 THEN
            SX(I) = RND * 320 - 160
            SY(I) = RND * 200 - 100
            SZ(I) = 255
        END IF

        PX = 160 + SX(I) * 256 / SZ(I)
        PY = 100 + SY(I) * 256 / SZ(I)

        IF PX >= 0 AND PX < 320 AND PY >= 0 AND PY < 200 THEN

            C = 255 - SZ(I)

            IF C < 1 THEN C = 1
            IF C > 255 THEN C = 255

            PSET (PX, PY), C

        END IF

    NEXT

END SUB

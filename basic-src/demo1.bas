' ==========================================
' QBASIC DEMO SCENE STYLE INTRO
' SCREEN 13 VGA DEMO
' ==========================================

DECLARE SUB InitStars ()
DECLARE SUB UpdateStars ()
DECLARE SUB DrawScroller ()

SCREEN 13
RANDOMIZE TIMER

CONST NumStars = 200

DIM SHARED SX(NumStars)
DIM SHARED SY(NumStars)
DIM SHARED SZ(NumStars)

DIM SHARED ScrollText$
DIM SHARED ScrollPos

ScrollText$ = " *** GREETINGS FROM QBASIC 4.5 *** HELLO TO ALL DOS CODERS *** LONG LIVE VGA MODE 13H *** THIS IS A RETRO DEMOSCENE STYLE INTRO WRITTEN IN PURE QBASIC *** "
ScrollPos = 1

InitStars

DO

    CLS

    UpdateStars

    ' Demo logo bars
    FOR I = 0 TO 7
        ColorVal = 50 + INT(50 * SIN(TIMER * 2 + I))
        IF ColorVal < 1 THEN ColorVal = 1

        X = 50
        Y = 40 + I

        LINE (X, Y)-(270, Y), ColorVal
    NEXT

    ColorVal = 100 + INT(50 * SIN(TIMER * 4))

    FOR Y = 70 TO 110
        LINE (80, Y)-(240, Y), ColorVal
    NEXT

    DrawScroller

    FOR WaitLoop = 1 TO 3000
    NEXT

LOOP UNTIL INKEY$ <> ""

SCREEN 0
WIDTH 80
CLS
END

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


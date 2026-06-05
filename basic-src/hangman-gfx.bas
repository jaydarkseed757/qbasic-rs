' =========================================
' GRAPHICAL HANGMAN FOR QBASIC
' =========================================

SCREEN 12
CLS
RANDOMIZE TIMER

DIM Words$(10)
DIM Word$, Guess$, Letter$, Guessed$, A$
DIM Wrong, MaxWrong, Complete, I
DIM Display$

Words$(1) = "COMPUTER"
Words$(2) = "KEYBOARD"
Words$(3) = "PROGRAM"
Words$(4) = "DATABASE"
Words$(5) = "NETWORK"
Words$(6) = "MONITOR"
Words$(7) = "PYTHON"
Words$(8) = "RUST"
Words$(9) = "QBASIC"
Words$(10) = "PRINTER"

Word$ = Words$(INT(RND * 10) + 1)

Guessed$ = ""
Wrong = 0
MaxWrong = 6

DO
    CLS

    LOCATE 1, 1
    PRINT "======================="
    PRINT "      HANGMAN"
    PRINT "======================="

    GOSUB DrawHangman

    Display$ = ""
    Complete = 1

    FOR I = 1 TO LEN(Word$)
        Letter$ = MID$(Word$, I, 1)

        IF INSTR(Guessed$, Letter$) > 0 THEN
            Display$ = Display$ + Letter$ + " "
        ELSE
            Display$ = Display$ + "_ "
            Complete = 0
        END IF
    NEXT I

    LOCATE 18, 46
    PRINT "Word:"
    LOCATE 19, 46
    PRINT Display$

    LOCATE 21, 46
    PRINT "Guessed Letters:"
    LOCATE 22, 46
    PRINT Guessed$

    LOCATE 24, 46
    PRINT "Wrong guesses:"; Wrong; "/"; MaxWrong

    IF Complete = 1 THEN
        LOCATE 26, 46
        PRINT "YOU WIN!"
        EXIT DO
    END IF

    IF Wrong >= MaxWrong THEN
        LOCATE 26, 46
        PRINT "YOU LOSE!"
        LOCATE 27, 46
        PRINT "The word was: "; Word$
        EXIT DO
    END IF

    LOCATE 28, 46
    INPUT "Enter a letter"; Guess$

    IF LEN(Guess$) = 0 THEN GOTO SkipGuess

    Guess$ = UCASE$(LEFT$(Guess$, 1))

    IF Guess$ < "A" OR Guess$ > "Z" THEN
        LOCATE 29, 46
        PRINT "Please enter a letter."
        SLEEP 2
        GOTO SkipGuess
    END IF

    IF INSTR(Guessed$, Guess$) > 0 THEN
        LOCATE 29, 46
        PRINT "Already guessed!"
        SLEEP 2
        GOTO SkipGuess
    END IF

    Guessed$ = Guessed$ + Guess$

    IF INSTR(Word$, Guess$) = 0 THEN
        Wrong = Wrong + 1
    END IF

SkipGuess:
LOOP

LOCATE 30, 46
PRINT "Press ENTER to quit..."
LINE INPUT A$

END

' =========================================
' DRAW GRAPHICAL HANGMAN
' =========================================
DrawHangman:

' Gallows
LINE (100, 400)-(250, 400), 15
LINE (150, 400)-(150, 100), 15
LINE (150, 100)-(300, 100), 15
LINE (300, 100)-(300, 140), 15

' Head
IF Wrong >= 1 THEN
    CIRCLE (300, 170), 30, 15
END IF

' Body
IF Wrong >= 2 THEN
    LINE (300, 200)-(300, 300), 15
END IF

' Left Arm
IF Wrong >= 3 THEN
    LINE (300, 220)-(260, 260), 15
END IF

' Right Arm
IF Wrong >= 4 THEN
    LINE (300, 220)-(340, 260), 15
END IF

' Left Leg
IF Wrong >= 5 THEN
    LINE (300, 300)-(260, 360), 15
END IF

' Right Leg
IF Wrong >= 6 THEN
    LINE (300, 300)-(340, 360), 15
END IF

RETURN


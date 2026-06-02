' =========================================
' HANGMAN GAME FOR QBASIC
' FIXED VERSION
' =========================================

CLS
RANDOMIZE TIMER

DIM Words$(10)
DIM Word$, Guess$, Letter$, Guessed$, A$
DIM Wrong, MaxWrong, Complete, I

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

    PRINT "======================="
    PRINT "      HANGMAN"
    PRINT "======================="
    PRINT

    GOSUB DrawHangman

    PRINT
    PRINT "Word: ";

    ' BUG 1 FIX: Use 1 (TRUE) instead of -1
    Complete = 1

    FOR I = 1 TO LEN(Word$)
        Letter$ = MID$(Word$, I, 1)

        IF INSTR(Guessed$, Letter$) > 0 THEN
            PRINT Letter$; " ";
        ELSE
            PRINT "_ ";
            Complete = 0
        END IF
    NEXT I

    PRINT
    PRINT

    PRINT "Guessed Letters: ";

    FOR I = 1 TO LEN(Guessed$)
        PRINT MID$(Guessed$, I, 1); " ";
    NEXT I

    PRINT
    PRINT
    PRINT "Wrong guesses:"; Wrong; "/"; MaxWrong
    PRINT

    IF Complete THEN
        PRINT "YOU WIN!"
        EXIT DO
    END IF

    IF Wrong >= MaxWrong THEN
        PRINT "YOU LOSE!"
        PRINT "The word was: "; Word$
        EXIT DO
    END IF

    INPUT "Enter a letter: ", Guess$

    IF LEN(Guess$) = 0 THEN GOTO SkipGuess

    Guess$ = UCASE$(LEFT$(Guess$, 1))

    IF Guess$ < "A" OR Guess$ > "Z" THEN
        PRINT "Please enter a letter."
        ' BUG 3 FIX: SLEEP 2 so the message is actually readable
        SLEEP 2
        GOTO SkipGuess
    END IF

    IF INSTR(Guessed$, Guess$) > 0 THEN
        PRINT "Already guessed!"
        ' BUG 3 FIX: SLEEP 2 so the message is actually readable
        SLEEP 2
        GOTO SkipGuess
    END IF

    Guessed$ = Guessed$ + Guess$

    IF INSTR(Word$, Guess$) = 0 THEN
        Wrong = Wrong + 1
    END IF

SkipGuess:
LOOP

PRINT
PRINT "Press ENTER to quit..."
LINE INPUT A$
END

' =========================================
' DRAW HANGMAN
' =========================================
DrawHangman:

PRINT " +----+"
PRINT " |"              ' BUG 2 FIX: pole segment above head

IF Wrong >= 1 THEN
    PRINT " |    O"
ELSE
    PRINT " |"
END IF

IF Wrong = 2 THEN
    PRINT " |    |"
ELSEIF Wrong = 3 THEN
    PRINT " |   /|"
ELSEIF Wrong >= 4 THEN
    PRINT " |   /|\"
ELSE
    PRINT " |"
END IF

IF Wrong = 5 THEN
    PRINT " |   /"
ELSEIF Wrong >= 6 THEN
    PRINT " |   / \"
ELSE
    PRINT " |"
END IF

PRINT "_|_"

RETURN



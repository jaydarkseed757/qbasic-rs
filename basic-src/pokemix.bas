' ============================================================
' POKE MATRIX v2.0  --  QuickBASIC 4.5
' ============================================================
' Commands: DECRYPT, SCAN, DUMP, RESET, or a single A-Z letter
' ============================================================

DEFINT A-Z
RANDOMIZE TIMER

' --- Global state ---
DIM SHARED K$, M$, Z$, A$, B$, C$
DIM SHARED X AS INTEGER
DIM SHARED Mem(2) AS INTEGER
DIM SHARED Ticks AS INTEGER
DIM SHARED DataPtr AS INTEGER
DIM SHARED CorruptCount AS INTEGER
DIM SHARED Halted AS INTEGER

DATA 14, 27, 53, 88, 41, 99

CALL Boot
CALL MainLoop
END

' ============================================================
SUB Boot
' ============================================================
    CLS
    COLOR 2, 0
    PRINT "  +==========================================+"
    PRINT "  |      POKE MATRIX v2.0                    |"
    PRINT "  |      QuickBASIC 4.50  -- POKE/PEEK SIM  |"
    PRINT "  +==========================================+"
    PRINT
    COLOR 8
    PRINT "  Initializing DEFINT scope..."
    PRINT "  Seeding RANDOMIZE TIMER...   seed="; INT(RND * 65535)
    PRINT "  DEF SEG = &H0000             OK"
    PRINT

    K$ = "JQWGF"
    M$ = "EPSC"
    X = 2
    Ticks = 0
    DataPtr = 0
    CorruptCount = 0
    Halted = 0

    CALL BuildZ
    CALL PokeRoutine

    COLOR 8
    PRINT "  State vectors loaded."
    PRINT
    CALL PrintSysState("")
    CALL PrintHelp
END SUB

' ============================================================
SUB MainLoop
' ============================================================
    DIM Q$
    DO WHILE Halted = 0
        COLOR 10
        PRINT
        LOCATE , 1: PRINT "INPUT > ";
        COLOR 14
        LINE INPUT Q$
        Q$ = UCASE$(RTRIM$(LTRIM$(Q$)))
        COLOR 2
        PRINT

        IF Q$ = "DECRYPT" THEN
            CALL HandleDecrypt
        ELSEIF Q$ = "SCAN" THEN
            CALL HandleScan
        ELSEIF Q$ = "DUMP" THEN
            CALL HandleDump
        ELSEIF Q$ = "RESET" THEN
            CALL HandleReset
        ELSEIF LEN(Q$) = 0 THEN
            ' nothing
        ELSEIF LEN(Q$) > 1 THEN
            COLOR 14
            PRINT "  *** STRING BURST DETECTED! MUTATING STATE... ***"
            DIM OldM$
            OldM$ = M$
            M$ = MID$(M$, 2) + CHR$(65 + (Ticks MOD 26))
            COLOR 8
            PRINT "  M$ "; OldM$; " -> "; M$
            PRINT
        ELSEIF ASC(Q$) >= 65 AND ASC(Q$) <= 90 THEN
            CALL HandleSymbol(Q$)
        ELSE
            CALL HandleCorrupt(Q$)
        END IF
    LOOP
END SUB

' ============================================================
SUB BuildZ
' ============================================================
    Z$ = "ERR0SYS1"
    A$ = LEFT$(Z$, 4)
    B$ = RIGHT$(Z$, 4)
    C$ = MID$(Z$, 3, 2)
END SUB

' ============================================================
SUB PokeRoutine
' ============================================================
    DIM I AS INTEGER, V AS INTEGER
    RESTORE
    ' skip to DataPtr position
    FOR I = 1 TO DataPtr
        READ V
    NEXT I
    FOR I = 0 TO 2
        READ V
        DataPtr = DataPtr + 1
        Mem(I) = (V + X) MOD 256
        POKE 1040 + I, Mem(I)
    NEXT I
END SUB

' ============================================================
SUB ReadNextData (V AS INTEGER)
' ============================================================
    ' We rely on DataPtr; re-read from scratch each call
    DIM I AS INTEGER, Tmp AS INTEGER
    RESTORE
    FOR I = 1 TO DataPtr
        READ Tmp
    NEXT I
    READ V
    DataPtr = DataPtr + 1
END SUB

' ============================================================
FUNCTION ComputeMem AS INTEGER
' ============================================================
    ComputeMem = (Mem(0) + Mem(1) + Mem(2)) MOD 256
END FUNCTION

' ============================================================
SUB PrintSysState (Extra$)
' ============================================================
    DIM MemVal AS INTEGER
    MemVal = ComputeMem
    COLOR 8
    PRINT "  "; STRING$(46, 196)
    COLOR 8:  PRINT "  SYSTEM STATE : ";
    COLOR 10: PRINT A$; "-"; B$; "-"; C$;
    COLOR 8:  PRINT "   MEM: "; MemVal
    COLOR 8:  PRINT "  KEY BUFFER   : ";
    COLOR 11: PRINT K$
    COLOR 8:  PRINT "  MATRIX FRAME : ";
    COLOR 14: PRINT "["; Mem(0); " "; Mem(1); " "; Mem(2); "]"
    IF Extra$ <> "" THEN
        COLOR 14: PRINT "  "; Extra$
    END IF
    COLOR 8
    PRINT "  "; STRING$(46, 196)
END SUB

' ============================================================
SUB PrintHelp
' ============================================================
    COLOR 8
    PRINT "  COMMANDS:"
    COLOR 7:  PRINT "  DECRYPT     ";
    COLOR 8:  PRINT "- decode current key buffer"
    COLOR 7:  PRINT "  SCAN        ";
    COLOR 8:  PRINT "- probe memory matrix"
    COLOR 7:  PRINT "  DUMP        ";
    COLOR 8:  PRINT "- hexdump memory vector"
    COLOR 7:  PRINT "  RESET       ";
    COLOR 8:  PRINT "- reinitialize state"
    COLOR 7:  PRINT "  <A-Z>       ";
    COLOR 8:  PRINT "- inject symbol into cipher"
    PRINT "  "; STRING$(46, 196)
END SUB

' ============================================================
SUB HandleDecrypt
' ============================================================
    DIM I AS INTEGER, Ch$, Decoded$
    Decoded$ = ""
    FOR I = 1 TO LEN(K$)
        Decoded$ = Decoded$ + CHR$(ASC(MID$(K$, I, 1)) - X)
    NEXT I
    COLOR 8
    PRINT "  +- DECRYPTING KEY BUFFER -------------------+"
    PRINT "  | RAW   : ";
    COLOR 14: PRINT K$
    COLOR 8
    PRINT "  | SHIFT : -"; X
    PRINT "  | RESULT: ";
    COLOR 10: PRINT Decoded$
    COLOR 8
    PRINT "  +--------------------------------------------+"
    PRINT
END SUB

' ============================================================
SUB HandleScan
' ============================================================
    DIM I AS INTEGER, Val AS INTEGER, BarLen AS INTEGER, Bar$
    COLOR 8
    PRINT "  +- MEMORY SCAN -----------------------------+"
    FOR I = 0 TO 2
        Val = Mem(I)
        BarLen = Val \ 16
        Bar$ = STRING$(BarLen, 219) + STRING$(16 - BarLen, 176)
        COLOR 8:  PRINT "  | &H"; HEX$(1040 + I); " ";
        COLOR 11: PRINT "["; Bar$; "]";
        COLOR 7:  PRINT " "; Val
    NEXT I
    COLOR 8
    PRINT "  | CHECKSUM: 0x"; HEX$(ComputeMem)
    PRINT "  +--------------------------------------------+"
    PRINT
END SUB

' ============================================================
SUB HandleDump
' ============================================================
    DIM I AS INTEGER, B AS INTEGER
    DIM Bytes(15) AS INTEGER
    DIM HexRow$, AscRow$, Ch$

    FOR I = 0 TO 15
        Bytes(I) = INT(RND * 256)
    NEXT I
    Bytes(0) = Mem(0)
    Bytes(7) = Mem(1)
    Bytes(15) = Mem(2)

    HexRow$ = ""
    AscRow$ = ""
    FOR I = 0 TO 15
        B = Bytes(I)
        HexRow$ = HexRow$ + RIGHT$("0" + HEX$(B), 2) + " "
        IF I = 7 THEN HexRow$ = HexRow$ + " "
        IF B >= 32 AND B < 127 THEN
            AscRow$ = AscRow$ + CHR$(B)
        ELSE
            AscRow$ = AscRow$ + "."
        END IF
    NEXT I

    COLOR 8
    PRINT "  +- HEX VECTOR DUMP -------------------------+"
    PRINT "  | "; : COLOR 11: PRINT HexRow$
    COLOR 8: PRINT "  | ASCII: "; : COLOR 14: PRINT AscRow$
    COLOR 8: PRINT "  +--------------------------------------------+"
    PRINT
END SUB

' ============================================================
SUB HandleReset
' ============================================================
    COLOR 12
    PRINT "  *** REINITIALIZING STATE VECTORS ***"
    K$ = "JQWGF"
    M$ = "EPSC"
    X = 2
    DataPtr = 0
    CorruptCount = 0
    CALL BuildZ

    ' Re-read data from scratch
    DIM I AS INTEGER, V AS INTEGER
    RESTORE
    FOR I = 0 TO 2
        READ V
        DataPtr = DataPtr + 1
        Mem(I) = (V + X) MOD 256
        POKE 1040 + I, Mem(I)
    NEXT I

    COLOR 8
    PRINT "  State reset complete."
    PRINT
    CALL PrintSysState("")
END SUB

' ============================================================
SUB HandleSymbol (Ch$)
' ============================================================
    DIM D AS INTEGER, G$
    K$ = K$ + Ch$
    IF X < 25 THEN X = X + 1
    Ticks = Ticks + 1

    ' Read next data value
    DIM I AS INTEGER, Tmp AS INTEGER
    RESTORE
    FOR I = 1 TO DataPtr
        READ Tmp
    NEXT I
    READ D
    DataPtr = DataPtr + 1

    IF D = 99 THEN
        COLOR 12
        PRINT "  +============================================+"
        PRINT "  |   === SYSTEM MEMORY HALT ===              |"
        PRINT "  |   DATA STREAM EXHAUSTED.                  |"
        PRINT "  +============================================+"
        Halted = 1
        EXIT SUB
    END IF

    FOR I = 0 TO 2
        Mem(I) = (Mem(I) + D + X) MOD 256
        POKE 1040 + I, Mem(I)
    NEXT I

    IF LEN(K$) > 8 THEN
        COLOR 14
        PRINT "  *** K$ OVERFLOW: TRUNCATING BUFFER ***"
        K$ = RIGHT$(K$, 5)
    END IF

    G$ = M$ + K$
    K$ = MID$(G$, 5)
    M$ = LEFT$(G$, 4)

    CALL BuildZ
    CALL PrintSysState("SYMBOL '" + Ch$ + "' INJECTED  SHIFT=" + STR$(X))
END SUB

' ============================================================
SUB HandleCorrupt (Ch$)
' ============================================================
    CorruptCount = CorruptCount + 1
    K$ = CHR$(ASC(LEFT$(K$, 1)) + 1) + MID$(K$, 2)
    COLOR 12
    PRINT "  *** INVALID SYMBOL '"; Ch$; "': CORRUPTING K$ ***"
    COLOR 14
    PRINT "  K$ -> "; K$; "   [corrupt events: "; CorruptCount; "]"
    IF CorruptCount >= 5 THEN
        COLOR 12
        PRINT "  *** CORRUPTION THRESHOLD EXCEEDED ***"
        PRINT "  *** SUGGEST: RESET TO RECOVER      ***"
    END IF
    PRINT
END SUB

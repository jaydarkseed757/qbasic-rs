' Test: WHILE/WEND — basic, nested, never-executes
i = 1
WHILE i <= 3
    PRINT i
    i = i + 1
WEND

' Never executes
WHILE 0
    PRINT "nope"
WEND

' Nested
i = 1
WHILE i <= 2
    j = 1
    WHILE j <= 2
        PRINT i; j
        j = j + 1
    WEND
    i = i + 1
WEND

PRINT "done"

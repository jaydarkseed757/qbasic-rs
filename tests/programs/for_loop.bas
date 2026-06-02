' Test: FOR/NEXT — positive step, negative step, float step, EXIT FOR
FOR i = 1 TO 3
    PRINT i
NEXT i

FOR i = 5 TO 1 STEP -2
    PRINT i
NEXT i

FOR i = 1 TO 2 STEP 0.5
    PRINT i
NEXT i

FOR i = 1 TO 10
    IF i = 3 THEN EXIT FOR
NEXT i
PRINT "exit at"; i

PRINT "done"

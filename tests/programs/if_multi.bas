' Test: multi-line IF/ELSEIF/ELSE/END IF
x = 2

IF x = 1 THEN
    PRINT "one"
ELSEIF x = 2 THEN
    PRINT "two"
ELSEIF x = 3 THEN
    PRINT "three"
ELSE
    PRINT "other"
END IF

IF x > 10 THEN
    PRINT "big"
ELSE
    PRINT "small"
END IF

' No ELSE branch taken
IF x = 99 THEN
    PRINT "nope"
END IF

PRINT "done"

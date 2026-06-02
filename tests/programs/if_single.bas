' Test: single-line IF/THEN, IF/THEN/ELSE, nested single-line
x = 5

IF x > 3 THEN PRINT "big"
IF x < 3 THEN PRINT "small"
IF x = 5 THEN PRINT "five" ELSE PRINT "not five"
IF x = 9 THEN PRINT "nine" ELSE PRINT "not nine"

' Nested single-line via chained
IF x > 0 THEN IF x > 10 THEN PRINT "over ten" ELSE PRINT "under ten"

' Single-line with multiple statements via colon
IF x = 5 THEN PRINT "a": PRINT "b"

PRINT "done"

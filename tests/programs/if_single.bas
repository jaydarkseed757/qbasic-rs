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

' Block IF whose THEN body ENDS with a single-line IF, followed by a
' block-level ELSE on the next line.  The trailing single-line IF must NOT
' steal the block IF's ELSE (blackjack.bas AnimateDeal infinite-loop bug).
y = 1
IF y = 1 THEN
   PRINT "then-branch"
   IF y = 2 THEN PRINT "inner-true"
ELSE
   PRINT "else-branch"
END IF

' Same shape, but the outer condition is FALSE so the ELSE must run.
z = 0
IF z = 1 THEN
   PRINT "wrong-then"
   IF z = 9 THEN PRINT "wrong-inner"
ELSE
   PRINT "correct-else"
END IF

PRINT "done"

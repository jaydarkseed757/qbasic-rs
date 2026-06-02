' Test: DO/LOOP variants — DO WHILE, DO UNTIL, DO..LOOP WHILE, DO..LOOP UNTIL, EXIT DO
' DO WHILE
i = 1
DO WHILE i <= 3
    PRINT i
    i = i + 1
LOOP

' DO UNTIL
i = 1
DO UNTIL i > 3
    PRINT i
    i = i + 1
LOOP

' DO..LOOP WHILE (executes at least once)
i = 10
DO
    PRINT i
    i = i + 1
LOOP WHILE i <= 10

' DO..LOOP UNTIL
i = 1
DO
    PRINT i
    i = i + 1
LOOP UNTIL i > 3

' EXIT DO
i = 1
DO
    IF i = 3 THEN EXIT DO
    PRINT i
    i = i + 1
LOOP

PRINT "done"

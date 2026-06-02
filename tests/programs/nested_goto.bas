10 i = 1
20 j = 1
30 PRINT i * 10 + j
40 j = j + 1
50 IF j <= 3 THEN GOTO 30
60 i = i + 1
70 IF i <= 2 THEN GOTO 20
80 PRINT "done"

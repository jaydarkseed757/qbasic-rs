' Regression test: ERL — the numeric line label nearest before the statement
' that raised the most recent error. Two faults on different numbered lines
' must report different ERL values; ERL is 0 before any error; ERR and ERL
' survive together across RESUME NEXT.
10 PRINT "start erl:"; ERL
20 ON ERROR GOTO 1000
30 OPEN "qbc_no_such_file_a.txt" FOR INPUT AS #1
40 PRINT "after first"
50 INPUT #2, z$
60 PRINT "after second"
70 PRINT "final err:"; ERR; "erl:"; ERL
80 END
1000 PRINT "trapped err:"; ERR; "at line:"; ERL
1010 RESUME NEXT

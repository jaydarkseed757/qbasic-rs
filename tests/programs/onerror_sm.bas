' Regression test: ON ERROR GOTO <numeric line> in a __pc state-machine
' program. The handler line is a match arm; error dispatch must jump to it
' (not just clear the flag), and RESUME NEXT / RESUME <line> must jump back
' via the resume registers captured at the fault site. Deliberately contains
' NO plain GOTO: the numeric ON ERROR alone must activate the state machine.
10 ON ERROR GOTO 1000
20 OPEN "qbc_no_such_file.txt" FOR INPUT AS #1
30 PRINT "resumed next"
40 PRINT "err code:"; E
50 ON ERROR GOTO 2000
60 OPEN "qbc_no_such_file2.txt" FOR INPUT AS #1
70 PRINT "not reached"
90 PRINT "resumed at 90"
100 PRINT "done"
110 END
1000 PRINT "handler1"
1010 E = ERR
1020 RESUME NEXT
2000 PRINT "handler2"
2010 RESUME 90

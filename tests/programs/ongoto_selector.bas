' Regression: a variable used ONLY as an ON..GOTO/GOSUB selector was never
' declared, so the emitted Rust referenced an undeclared local (E0425).
' collect_locals had no Stmt::OnGoto arm — found by the differential fuzzer
' once mode B started generating computed branches.
'
' K is deliberately assigned nowhere: QB reads an unset numeric as 0, and
' 0 is out of range for ON..GOTO, so the branch falls through.
10 ON ABS(K) MOD 5 GOTO 50, 60
20 PRINT "fell through"
30 SEL = 2
40 ON SEL GOTO 50, 60
50 PRINT "one"
55 GOTO 70
60 PRINT "two"
70 ON ABS(J) + 1 GOSUB 100
80 PRINT "done"
90 END
100 PRINT "sub"
110 RETURN

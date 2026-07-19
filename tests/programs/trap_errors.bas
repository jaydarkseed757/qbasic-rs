' Regression test: trappable errors beyond OPEN-file-not-found.
'   err 62  INPUT # past end of file
'   err 52  INPUT # on an unopened file number
'   err 54  PRINT # to a file open for the wrong mode (INPUT)
'   err  4  READ past the end of DATA
' Each fault site RESUMEs NEXT out of a numeric state-machine handler.
10 ON ERROR GOTO 1000
20 OPEN "trap_errors.dat" FOR OUTPUT AS #1
30 PRINT #1, "only line"
40 CLOSE #1
50 OPEN "trap_errors.dat" FOR INPUT AS #1
60 INPUT #1, A$
70 PRINT "got: "; A$
80 INPUT #1, B$
90 PRINT "past eof err:"; E
100 PRINT #1, "wrong mode"
110 PRINT "wrong mode err:"; E
120 CLOSE #1
130 INPUT #2, C$
140 PRINT "unopened err:"; E
150 READ X
160 PRINT "data:"; X
170 READ Y
180 PRINT "out of data err:"; E
190 END
900 DATA 7
1000 E = ERR
1010 RESUME NEXT

' Test: PRINT #n, USING fmt$; args  (USING-formatted file output)
fmt$ = "\        \ ##### ###.##"

OPEN "PFUTEST.TXT" FOR OUTPUT AS #1
PRINT #1, USING fmt$; "ALPHA"; 42; 3.14159
PRINT #1, USING fmt$; "LONGERNAME"; 12345; 99.9
PRINT #1, USING "$$####.##"; 1234.5
CLOSE #1

' Read it back and echo to stdout
OPEN "PFUTEST.TXT" FOR INPUT AS #1
DO WHILE NOT EOF(1)
  LINE INPUT #1, l$
  PRINT l$
LOOP
CLOSE #1
PRINT "done"

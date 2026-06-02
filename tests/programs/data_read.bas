' Test: DATA/READ/RESTORE
DATA 10, 20, 30
DATA "alpha", "beta", "gamma"

READ a, b, c
PRINT a
PRINT b
PRINT c

READ s$, t$, u$
PRINT s$
PRINT t$
PRINT u$

' RESTORE and re-read numeric data
RESTORE
READ x
PRINT x

PRINT "done"

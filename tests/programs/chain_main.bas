' CHAIN passes COMMON values to the transpiled binary of the named program
' (found next to the current executable) and replaces this process.
COMMON SHARED score, msg$
score = 42
msg$ = "hi there"
PRINT "main: chaining"
CHAIN "chain_child"
PRINT "not reached"

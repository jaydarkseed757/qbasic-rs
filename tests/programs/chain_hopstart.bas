' Regression: a CHAINed-into process CHAINing AGAIN. The existing
' chain_main/chain_child pair only covers a single hop; this checks that
' COMMON survives two consecutive handoffs and that the second CHAIN works
' from inside a process that was itself exec'd into.
'
' NAMING: the runner compiles every test into one directory in alphabetical
' order, and CHAIN resolves its target next to the running executable — so a
' program must sort AFTER everything it chains to, or the target won't exist
' yet when it runs. Hence hopend < hopmid < hopstart, rather than 1/2/3.
' (chain_main/chain_child depends on the same ordering.)
COMMON SHARED n, note$
n = 3
note$ = "a"
PRINT "start n="; n
CHAIN "chain_hopmid"
PRINT "not reached"

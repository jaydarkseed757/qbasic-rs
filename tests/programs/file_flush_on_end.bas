' Regression: buffered sequential output must be flushed when the program
' terminates via END (or by falling off the end), not only on explicit CLOSE.
'
' Sequential writes go through a BufWriter, which normally flushes when
' dropped — but END lowers to Runtime::quit(), which calls process::exit and
' therefore skips every destructor.  Before the fix, a program that wrote a
' file and ended without CLOSE left a ZERO-BYTE file behind: silent data loss.
'
' This test writes WITHOUT closing, then relies on the CHAIN-free second
' phase below reading it back — so the bytes must have reached disk by the
' time the reader opens it.

' --- Phase 1: write, deliberately WITHOUT closing #1 --------------------
OPEN "FLUSHTST.TXT" FOR OUTPUT AS #1
PRINT #1, "alpha"
PRINT #1, "beta"

' LOF on the still-open write handle must also see the buffered bytes
' (LOF used to be hardcoded to 0 for every file, which silently broke the
' standard "does a saved file exist yet?" check).
PRINT "LOF="; LOF(1)

CLOSE #1

' --- Phase 2: read it back ---------------------------------------------
OPEN "FLUSHTST.TXT" FOR INPUT AS #2
DO WHILE NOT EOF(2)
  LINE INPUT #2, l$
  PRINT l$
LOOP
PRINT "SIZE="; LOF(2)
CLOSE #2

PRINT "done"

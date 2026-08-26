' CHAIN2.BAS - stage 2 of the pipeline (see chain1.bas)
'
' Receives stage 1's COMMON block positionally, does its share of the work,
' and chains onward. Note this program CHAINs after having itself been
' CHAINed into: the handoff is a plain exec, so it nests as deep as you like.

COMMON SHARED stage, total, tally, trail$

PRINT "== stage 2: transform =="
IF stage = 0 THEN
    PRINT "  (standalone - nothing was handed to me)"
ELSE
    PRINT "  received total ="; total; " from stage"; stage
END IF

stage = 2
total = total * 6
tally = tally + 1
trail$ = trail$ + " -> s2"

PRINT "  transformed total ="; total
PRINT "  handing off to stage 3..."
PRINT

CHAIN "chain3"

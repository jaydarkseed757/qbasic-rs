' CHAIN3.BAS - stage 3 of the pipeline (see chain1.bas)
'
' The end of the chain: report what survived the trip. Every value here
' crossed two process boundaries to arrive.

COMMON SHARED stage, total, tally, trail$

PRINT "== stage 3: report =="
IF stage = 0 THEN
    PRINT "  (standalone - nothing was handed to me)"
    PRINT "  run bin/chain1 to see the full pipeline"
ELSE
    tally = tally + 1
    PRINT "  final total  ="; total
    PRINT "  stages run   ="; tally
    PRINT "  trail        = "; trail$; " -> s3"
    PRINT
    PRINT "  all four values crossed two process boundaries,"
    PRINT "  matched by POSITION in the COMMON list."
END IF

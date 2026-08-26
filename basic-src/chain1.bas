' CHAIN1.BAS - stage 1 of a three-part CHAIN pipeline
'
' CHAIN exists because DOS gave you 640K. A program too big to fit was split
' into modules that CHAINed to one another, handing their working state over
' through COMMON. This trio walks one value through three separate programs.
'
' The catch that makes COMMON worth demonstrating: values are matched by
' POSITION, not by name. Every program in the chain must declare the same
' COMMON list in the same order - rename a variable and it still works, but
' reorder one and stage 2 silently reads stage 1's string as a number.
'
' Run bin/chain1 to see the whole pipeline. Run bin/chain2 or bin/chain3 on
' their own and they print type defaults instead, because nothing handed
' them anything - which is exactly what QB does.

COMMON SHARED stage, total, tally, trail$

stage = 1
total = 7
tally = 1
trail$ = "start"

PRINT "== stage 1: gather =="
PRINT "  seeded total ="; total
PRINT "  handing off to stage 2..."
PRINT

trail$ = trail$ + " -> s1"
CHAIN "chain2"

PRINT "not reached - CHAIN replaces this program"

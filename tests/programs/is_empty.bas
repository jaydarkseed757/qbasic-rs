' Test: T6 emptiness comparisons (s$ = "" / s$ <> "" -> is_empty())
DIM k AS STRING

' Value path (qb_from_bool): empty and non-empty, both ops
s$ = ""
PRINT s$ = ""
PRINT s$ <> ""
s$ = "hi"
PRINT s$ = ""
PRINT s$ <> ""

' Reversed operand order
PRINT "" = s$
PRINT "" <> s$

' Cond path: IF and DO-loop guard
IF s$ = "" THEN PRINT "empty" ELSE PRINT "not empty"
t$ = "abc"
DO WHILE t$ <> ""
  PRINT t$
  t$ = MID$(t$, 2)
LOOP

' Ctx-typed string (sigil-less DIM ... AS STRING, the farkle-style path)
k = ""
IF k = "" THEN PRINT "k empty"
k = "x"
PRINT k <> ""

' Builtin-call subject (owned String)
PRINT LEFT$("abc", 0) = ""

' Both-literal case stays on the normal comparison path
PRINT "" = ""

PRINT "done"

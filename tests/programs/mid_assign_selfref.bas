' Regression: MID$ statement-form borrow safety + scan coverage
' (differential-fuzzer finds):
' - pos/len/val expressions may READ the target string; QB evaluates them
'   BEFORE the in-place replacement (emitter now hoists them to temps).
' - a variable referenced ONLY inside a MID$ statement (W below, in a GOSUB
'   sub) must still be declared/promoted.
S$ = "abcdefgh"
MID$(S$, LEN(S$) - 5, 2) = S$
PRINT S$
T$ = "12345"
MID$(T$, 2) = LEFT$(T$, 3)
PRINT T$
GOSUB Tail
PRINT U$
END
Tail:
U$ = "world"
MID$(U$, ABS(W) + 2, 2) = "OR"
RETURN

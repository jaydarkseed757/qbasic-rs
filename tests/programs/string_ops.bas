' Test: string built-in functions
a$ = "Hello, World!"

PRINT LEN(a$)
PRINT LEFT$(a$, 5)
PRINT RIGHT$(a$, 6)
PRINT MID$(a$, 8, 5)
PRINT UCASE$(a$)
PRINT LCASE$(a$)
PRINT INSTR(a$, "World")
PRINT INSTR(a$, "xyz")
PRINT CHR$(65)
PRINT ASC("A")
PRINT SPACE$(4)
PRINT STRING$(4, 42)

' STR$ and VAL
PRINT STR$(42)
PRINT VAL("  3.14")
PRINT VAL("abc")

' Edge cases
PRINT LEN("")
PRINT LEFT$("hi", 0)
PRINT MID$("hello", 2, 3)

PRINT "done"

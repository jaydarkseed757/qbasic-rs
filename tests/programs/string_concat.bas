' Test: string concatenation and comparison
a$ = "Hello"
b$ = " World"
c$ = a$ + b$
PRINT c$
PRINT LEN(c$)

' Comparison operators
PRINT a$ = "Hello"
PRINT a$ = "hello"
PRINT a$ < "Z"
PRINT a$ > "A"
PRINT "abc" < "abd"
PRINT "abc" = "abc"

' Concatenation in expressions
PRINT "foo" + "bar" + "baz"
PRINT "n=" + STR$(42)

PRINT "done"

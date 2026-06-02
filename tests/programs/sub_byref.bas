' Test: SUB with byref params — numeric and string
SUB DoubleIt(x)
    x = x * 2
END SUB

SUB AppendBang(s$)
    s$ = s$ + "!"
END SUB

SUB AddTwo(a, b)
    a = a + 10
    b = b + 20
END SUB

n = 5
CALL DoubleIt(n)
PRINT n

t$ = "hello"
CALL AppendBang(t$)
PRINT t$

x = 1
y = 2
CALL AddTwo(x, y)
PRINT x
PRINT y

PRINT "done"

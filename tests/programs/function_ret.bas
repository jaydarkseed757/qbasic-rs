' Test: FUNCTION returning f64 and String
FUNCTION Square(x)
    Square = x * x
END FUNCTION

FUNCTION Greet$(name$)
    Greet$ = "Hello, " + name$ + "!"
END FUNCTION

FUNCTION Factorial(n)
    IF n <= 1 THEN
        Factorial = 1
    ELSE
        Factorial = n * Factorial(n - 1)
    END IF
END FUNCTION

PRINT Square(5)
PRINT Square(0)
PRINT Greet$("World")
PRINT Factorial(5)
PRINT Factorial(1)

PRINT "done"

' Test: GOSUB accessing and modifying main-scope variables
x = 10
y = 20

GOSUB AddThem
PRINT result

GOSUB DoubleX
PRINT x

PRINT "done"
END

AddThem:
    result = x + y
    RETURN

DoubleX:
    x = x * 2
    RETURN

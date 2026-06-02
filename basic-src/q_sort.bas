CLS

DIM numbers(10)

' Input 10 numbers
PRINT "Enter 10 numbers:"
FOR i = 1 TO 10
    INPUT numbers(i)
NEXT i

' Bubble Sort
FOR i = 1 TO 9
    FOR j = 1 TO 10 - i
        IF numbers(j) > numbers(j + 1) THEN
            temp = numbers(j)
            numbers(j) = numbers(j + 1)
            numbers(j + 1) = temp
        END IF
    NEXT j
NEXT i

' Display sorted numbers
PRINT
PRINT "Sorted numbers:"
FOR i = 1 TO 10
    PRINT numbers(i)
NEXT i

END


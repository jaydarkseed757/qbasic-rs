' Test: 1D arrays — numeric, string, UBOUND, passing to SUB
DIM nums(5)
DIM words$(3)

FOR i = 1 TO 5
    nums(i) = i * 10
NEXT i

words$(1) = "alpha"
words$(2) = "beta"
words$(3) = "gamma"

FOR i = 1 TO 5
    PRINT nums(i)
NEXT i

FOR i = 1 TO 3
    PRINT words$(i)
NEXT i

PRINT UBOUND(nums)
PRINT UBOUND(words$)

SUB SumArray(arr(), n, result)
    result = 0
    FOR k = 1 TO n
        result = result + arr(k)
    NEXT k
END SUB

DIM total
CALL SumArray(nums(), 5, total)
PRINT total

PRINT "done"

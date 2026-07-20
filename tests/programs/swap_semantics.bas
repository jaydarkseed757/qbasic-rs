' Regression tests for differential-fuzzer finds (widened-subset round):
' - SWAP borrow-safety: same-array elements (incl. self-referencing index),
'   scalar<->element where the index READS the swapped scalar (QB computes
'   both addresses BEFORE exchanging), 2-D elements, string swap, SWAP X, X.
' - Promotion: a scalar/array whose ONLY use in a GOSUB sub is a SWAP
'   operand or an assignment-target index must still promote.
DIM A(10)
DIM G(3, 2)
X = 5
Y = 9
SWAP X, Y
PRINT X; Y
SWAP X, X
PRINT X
A(0) = 2
A(2) = 7
A(7) = 1
SWAP A(A(0)), A(0)
PRINT A(0); A(2)
X = 2
SWAP X, A(ABS(X) MOD 11)
PRINT X; A(2)
G(1, 2) = 33
G(3, 0) = 44
SWAP G(1, 2), G(3, 0)
PRINT G(1, 2); G(3, 0)
S$ = "left"
T$ = "right"
SWAP S$, T$
PRINT S$; " "; T$
Q = 12
T$ = "ab"
GOSUB SubSwap
PRINT Q; A(1)
GOSUB SubIdx
PRINT G(2, 0)
END
SubSwap:
SWAP Q, A(1)
RETURN
SubIdx:
G(ABS(LEN(T$)) MOD 4, 0) = 77
RETURN

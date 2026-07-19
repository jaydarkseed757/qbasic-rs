' Regression test for two differential-fuzzer finds:
' 1) An array-element assignment whose INDEX reads the same array
'    (A(A(I)) = ...) must compile — naive emission is an E0502
'    place-borrow conflict in Rust.
' 2) A variable whose ONLY reference is inside an assignment-target's
'    index expression (Z below) must still be declared.
DIM A(10)
A(3) = 2
A(2) = 7
A(A(3)) = 40 + A(3)
PRINT A(2)
A(A(A(3)) \ 6) = 9
PRINT A(7)
A(Z + 5) = 11
PRINT A(5)
S$ = "ok"
PRINT S$

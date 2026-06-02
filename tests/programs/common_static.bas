' Test: COMMON SHARED visible across SUB/main; STATIC persists across calls
COMMON SHARED Total
Total = 0
CALL AddIt
PRINT Total
CALL Bump
CALL Bump
CALL Bump

SUB AddIt
  Total = Total + 42
END SUB

SUB Bump
  STATIC Count
  Count = Count + 1
  PRINT Count
END SUB

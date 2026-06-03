' Test: random-access TYPE records round-trip through a real file.
' Writes two records, closes, reopens, reads them back, prints the fields.
TYPE PLAYERREC
  PNAME AS STRING * 10
  LEVEL AS INTEGER
  SCORE AS LONG
END TYPE

DIM REC AS PLAYERREC

OPEN "record_io.dat" FOR RANDOM AS #1 LEN = 16
REC.PNAME = "ALICE"
REC.LEVEL = 7
REC.SCORE = 123456
PUT #1, 1, REC
REC.PNAME = "BOB"
REC.LEVEL = 12
REC.SCORE = -9999
PUT #1, 2, REC
CLOSE #1

OPEN "record_io.dat" FOR RANDOM AS #1 LEN = 16
GET #1, 1, REC
PRINT REC.PNAME; REC.LEVEL; REC.SCORE
GET #1, 2, REC
PRINT REC.PNAME; REC.LEVEL; REC.SCORE
CLOSE #1
END

' TYPE body array fields: Bar(N) AS TYPE should allocate a Vec, not a scalar.

TYPE Grid
  Cell(4) AS INTEGER
END TYPE

' scalar TYPE var with array field
DIM g AS Grid
g.Cell(1) = 10
g.Cell(3) = 30
PRINT g.Cell(1); g.Cell(3)

' DIM SHARED TYPE var with array field, accessed in SUB
DIM SHARED sg AS Grid

SUB FillShared()
  sg.Cell(0) = 5
  sg.Cell(4) = 40
END SUB

FillShared
PRINT sg.Cell(0); sg.Cell(4)

' array of TYPE where the TYPE has an array field
DIM boards(2) AS Grid
boards(1).Cell(1) = 99
boards(2).Cell(0) = 11
PRINT boards(1).Cell(1); boards(2).Cell(0)

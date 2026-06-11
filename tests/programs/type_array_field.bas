TYPE Grid
  Cell(4) AS INTEGER
END TYPE
DIM g AS Grid
g.Cell(1) = 10
g.Cell(3) = 30
PRINT g.Cell(1); g.Cell(3)

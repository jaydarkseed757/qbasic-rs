' Test nested user-defined TYPEs (TYPE within TYPE)
TYPE Color
    R AS INTEGER
    G AS INTEGER
    B AS INTEGER
END TYPE

TYPE Sprite
    X AS SINGLE
    Y AS SINGLE
    Col AS Color
END TYPE

DIM s AS Sprite
s.X = 5.0
s.Y = 10.0
s.Col.R = 255
s.Col.G = 128
s.Col.B = 0
PRINT s.X
PRINT s.Y
PRINT s.Col.R
PRINT s.Col.G
PRINT s.Col.B
PRINT "done"

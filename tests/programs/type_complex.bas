' Complex TYPE tests: nested TYPEs in arrays, scalar params to SUBs, field swap

TYPE RGB
    R AS INTEGER
    G AS INTEGER
    B AS INTEGER
END TYPE

TYPE Pixel
    X AS SINGLE
    Y AS SINGLE
    C AS RGB
END TYPE

' 1-D array of nested TYPE
DIM px(1 TO 3) AS Pixel
px(1).X = 10.0 : px(1).Y = 20.0
px(1).C.R = 255 : px(1).C.G = 0   : px(1).C.B = 0
px(2).X = 30.0 : px(2).Y = 40.0
px(2).C.R = 0   : px(2).C.G = 255 : px(2).C.B = 0

FOR i = 1 TO 2
    PRINT px(i).X; px(i).Y; px(i).C.R; px(i).C.G; px(i).C.B
NEXT i

' Scalar nested TYPE passed to SUB
DIM p AS Pixel
p.X = 5.0 : p.Y = 6.0
p.C.R = 100 : p.C.G = 150 : p.C.B = 200

SUB PrintPixel(px AS Pixel)
    PRINT px.X; px.Y; px.C.R; px.C.G; px.C.B
END SUB

CALL PrintPixel(p)

' Field-level swap using temp RGB scalar
DIM tmp AS RGB
tmp.R = px(1).C.R : tmp.G = px(1).C.G : tmp.B = px(1).C.B
px(1).C.R = px(2).C.R : px(1).C.G = px(2).C.G : px(1).C.B = px(2).C.B
px(2).C.R = tmp.R : px(2).C.G = tmp.G : px(2).C.B = tmp.B
PRINT px(1).C.R; px(1).C.G; px(1).C.B
PRINT px(2).C.R; px(2).C.G; px(2).C.B

PRINT "done"

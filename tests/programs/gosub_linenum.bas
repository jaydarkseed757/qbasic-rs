10 n = 3
20 GOSUB 100
30 PRINT result
40 n = 7
50 GOSUB 100
60 PRINT result
70 END
100 result = n * (n + 1) / 2
110 RETURN

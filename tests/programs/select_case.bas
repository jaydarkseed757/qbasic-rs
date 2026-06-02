' Test: SELECT CASE — numeric, string, IS, TO range, CASE ELSE
x = 3
SELECT CASE x
    CASE 1
        PRINT "one"
    CASE 2, 3
        PRINT "two or three"
    CASE 4 TO 6
        PRINT "four to six"
    CASE IS > 10
        PRINT "over ten"
    CASE ELSE
        PRINT "other"
END SELECT

x = 5
SELECT CASE x
    CASE 4 TO 6
        PRINT "four to six"
    CASE ELSE
        PRINT "other"
END SELECT

x = 99
SELECT CASE x
    CASE 1
        PRINT "one"
    CASE ELSE
        PRINT "else"
END SELECT

' String select
a$ = "b"
SELECT CASE a$
    CASE "a"
        PRINT "letter a"
    CASE "b"
        PRINT "letter b"
    CASE ELSE
        PRINT "other letter"
END SELECT

PRINT "done"

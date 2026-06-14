ON ERROR GOTO FileError
OPEN "no-such-file-xyzzy.dat" FOR INPUT AS #1
PRINT "continued after open"
END

FileError:
PRINT "trapped"; ERR
RESUME NEXT

' SHELL cmd$ runs a host command synchronously with inherited stdio;
' its output appears between the two PRINTs.
PRINT "before"
SHELL "echo hello-from-shell"
PRINT "after"

' Regression: a statement beginning with one of the unsupported file/OS
' statement words was ALWAYS treated as that statement and skipped to EOL —
' including an ordinary assignment to a variable that merely shares the name.
'
'   name = 5          ' whole line vanished, no warning; name stayed 0
'   name(1) = 42      ' likewise
'
' These words are plain identifiers to the lexer, so the parser has to tell
' the two apart. Every real form (NAME "a" AS "b", KILL "f", SEEK #1, 5, ...)
' is followed by a string or file argument — never by `=` or `(` — so that
' one-token lookahead separates them. A sigiled `name$` was never affected:
' it lexes as IdentStr, which the OS-statement arm doesn't match.
'
' The genuine statements are still skipped (they aren't modelled), but now
' emit a stderr warning rather than disappearing — the project's
' "never silently drop a statement" rule.

' --- scalar assignments using the reserved-ish names -------------------
name = 5
name = name + 10
PRINT "name="; name

seek = 3
PRINT "seek="; seek

lock = 7
unlock = lock + 1
PRINT "lock="; lock; " unlock="; unlock

kill = 2
chdir = kill * 3
PRINT "kill="; kill; " chdir="; chdir

' --- array element assignment through the same names -------------------
DIM name(5)
name(1) = 42
name(2) = name(1) + 1
PRINT "arr="; name(1); name(2)

' --- the sigiled form was always safe; confirm it still is -------------
name$ = "text"
PRINT "str="; name$

PRINT "done"

' Regression test: emit_main and emit_gosub_fn must reset per-scope DIM
' bookkeeping (local_dim_names / local_string_arrays / local_string_scalars)
' instead of inheriting whatever the last-processed SUB left behind.
'
' Foo's local `DIM msg AS INTEGER` (a genuine shadow of the shared string
' inside Foo) must NOT leak into main's or the GOSUB target's view of the
' shared sigil-less `DIM SHARED msg AS STRING` -- with the leak, the
' local-shadows-shared guard fired in main too, so `msg = "..."` stopped
' routing to __gs.msg.
DECLARE SUB Foo ()
DIM SHARED msg AS STRING

CALL Foo
msg = "hello from main"
PRINT msg
GOSUB Tail
PRINT msg
END

Tail:
msg = msg + "!"
RETURN

SUB Foo
    DIM msg AS INTEGER
    msg = 5
    PRINT msg
END SUB

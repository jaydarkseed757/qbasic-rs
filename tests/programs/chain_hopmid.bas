' Middle of the chain: receives COMMON, modifies it, and chains onward.
' Run standalone (as the suite also does) it starts from type defaults.
COMMON SHARED n, note$
n = n * 10
note$ = note$ + "b"
PRINT "mid n="; n
CHAIN "chain_hopend"

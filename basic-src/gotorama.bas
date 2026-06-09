10 REM **** GOTORAMA.BAS -- GW-BASIC State-Machine Stress Test ****
20 REM 17 tests exercising every __pc pattern the transpiler handles:
30 REM   tight/nested backward-GOTO loops, ON GOTO dispatch (6 branches),
40 REM   GOSUB-inside-GOTO, forward skip, multi-source funnel, bubble sort,
50 REM   DATA/READ + computed ON GOTO, sieve (3-level GOTO nesting),
60 REM   12-hop spaghetti chain, early FOR exit, Collatz sequence,
70 REM   out-of-order triangular ping-pong, chained-IF vowel scanner,
80 REM   Euclidean GCD, diamond-GOTO accumulator, loop re-entry from outside,
85 REM   binary search -- plus three physical line continuations.
90 DIM A(25), PR(55), BS(10)
95 PASS = 0 : FAIL = 0
96 PRINT "GOTORAMA  GW-BASIC State-Machine Stress Test"
97 PRINT "============================================"
98 PRINT

100 REM ==== TEST 1: tight backward-GOTO loop (Fibonacci) ====
110 PRINT "1. Fibonacci (tight GOTO loop):"
120 F0 = 0 : F1 = 1 : K = 0 : LN$ = ""
130 LN$ = LN$ + STR$(F0) + " "
140 F2 = F0 + F1 : F0 = F1 : F1 = F2
150 K = K + 1 : IF K < 15 THEN GOTO 130
160 PRINT "   "; LN$
170 IF F0 = 610 THEN PASS = PASS + 1 : GOTO 200
180 PRINT "   FAIL fib: expected 610, got "; F0
190 FAIL = FAIL + 1

200 REM ==== TEST 2: two nested backward-GOTO loops ====
210 PRINT "2. Nested GOTO loops (5x5 dot grid):"
220 I = 1
230 PRINT "   ";
240 J = 1
250 PRINT ".";
260 J = J + 1 : IF J <= 5 THEN GOTO 250
270 PRINT
280 I = I + 1 : IF I <= 5 THEN GOTO 230
290 PASS = PASS + 1

300 REM ==== TEST 3: ON GOTO dispatch (6 branches) ====
310 PRINT "3. ON GOTO dispatch (6 branches):"
320 RESULT$ = "" : Q = 1
330 ON Q GOTO 341, 342, 343, 344, 345, 346
340 GOTO 360
341 RESULT$ = RESULT$ + "A" : GOTO 360
342 RESULT$ = RESULT$ + "B" : GOTO 360
343 RESULT$ = RESULT$ + "C" : GOTO 360
344 RESULT$ = RESULT$ + "D" : GOTO 360
345 RESULT$ = RESULT$ + "E" : GOTO 360
346 RESULT$ = RESULT$ + "F"
360 Q = Q + 1 : IF Q <= 6 THEN GOTO 330
370 PRINT "   "; RESULT$
380 IF RESULT$ = "ABCDEF" THEN PASS = PASS + 1 : GOTO 400
390 PRINT "   FAIL on-goto: expected ABCDEF" : FAIL = FAIL + 1

400 REM ==== TEST 4: GOSUB called inside a GOTO loop ====
410 PRINT "4. GOSUB inside GOTO loop (sum 1..10):"
420 SUM = 0 : M = 1
430 GOSUB 9100
440 M = M + 1 : IF M <= 10 THEN GOTO 430
450 PRINT "   Sum ="; SUM
460 IF SUM = 55 THEN PASS = PASS + 1 : GOTO 500
470 PRINT "   FAIL sum: expected 55, got "; SUM
480 FAIL = FAIL + 1

500 REM ==== TEST 5: forward skip + multi-source funnel ====
510 PRINT "5. Forward GOTO skip + funnel:"
520 X = 7 : ARRIVED = 0
530 IF X > 5 THEN GOTO 560
540 PRINT "   ERROR: branch should have been skipped"
550 GOTO 580
560 ARRIVED = ARRIVED + 1
570 REM funnel: fall-through from 560, and GOTO 580 from line 550
580 Y = 3
590 IF Y < 2 THEN GOTO 540
600 ARRIVED = ARRIVED + 1
610 PRINT "   ARRIVED ="; ARRIVED
620 IF ARRIVED = 2 THEN PASS = PASS + 1 : GOTO 650
630 PRINT "   FAIL skip: expected ARRIVED=2, got "; ARRIVED
640 FAIL = FAIL + 1

650 REM ==== TEST 6: bubble sort via two nested GOTO loops ====
660 PRINT "6. Bubble sort (nested GOTO loops):"
670 A(1)=7 : A(2)=3 : A(3)=9 : A(4)=1 : A(5)=5 : N=5
680 I = 1
690 J = 1
700 IF A(J) <= A(J+1) THEN GOTO 730
710 TMP = A(J) : A(J) = A(J+1) : A(J+1) = TMP
720 REM
730 J = J + 1 : IF J <= N - I THEN GOTO 700
740 I = I + 1 : IF I < N THEN GOTO 690
750 PRINT "   Sorted: ";
760 K = 1
770 PRINT A(K); " ";
780 K = K + 1 : IF K <= N THEN GOTO 770
790 PRINT
800 OK = 1 : K = 1
810 IF A(K) > A(K+1) THEN OK = 0
820 K = K + 1 : IF K < N THEN GOTO 810
825 REM  line continuation #1 -- IF..THEN body split across two physical lines
830 IF OK = 1 THEN PASS = PASS + 1 :
     GOTO 850
840 PRINT "   FAIL sort" : FAIL = FAIL + 1

850 REM ==== TEST 7: DATA/READ loop + computed ON GOTO branch ====
860 PRINT "7. DATA/READ + computed ON GOTO branch:"
870 RESTORE
880 TOT = 0
890 READ V : IF V = -1 THEN GOTO 930
900 TAG = (V MOD 3) + 1
910 ON TAG GOTO 911, 912, 913
911 TOT = TOT + V * 1 : GOTO 890
912 TOT = TOT + V * 2 : GOTO 890
913 TOT = TOT + V * 3 : GOTO 890
920 DATA 1, 2, 3, 4, 5, 6, 7, 8, -1
930 PRINT "   TOT ="; TOT
940 IF TOT = 78 THEN PASS = PASS + 1 : GOTO 1000
950 PRINT "   FAIL data: expected 78, got "; TOT : FAIL = FAIL + 1

1000 REM ==== TEST 8: sieve of Eratosthenes (3 nested GOTO loops) ====
1010 PRINT "8. Sieve of Eratosthenes (primes to 50):"
1020 FOR II = 2 TO 50 : PR(II) = 1 : NEXT II
1030 II = 2
1040 IF PR(II) = 0 THEN GOTO 1080
1050 JJ = II + II
1060 IF JJ > 50 THEN GOTO 1080
1070 PR(JJ) = 0 : JJ = JJ + II : GOTO 1060
1080 II = II + 1 : IF II <= 7 THEN GOTO 1040
1090 OUT$ = ""
1100 KK = 2
1110 IF PR(KK) = 0 THEN GOTO 1130
1120 OUT$ = OUT$ + STR$(KK) + " "
1130 KK = KK + 1 : IF KK <= 50 THEN GOTO 1110
1140 PRINT "   "; OUT$
1145 REM  line continuation #2 -- same IF..THEN split pattern
1150 IF LEFT$(OUT$, 2) = " 2" THEN
     PASS = PASS + 1 : GOTO 1200
1160 PRINT "   FAIL sieve" : FAIL = FAIL + 1

1200 REM ==== TEST 9: 12-hop spaghetti GOTO chain (sum = 2^12 - 1 = 4095) ====
1210 PRINT "9. 12-hop spaghetti GOTO chain:"
1220 CHAIN = 0 : GOTO 1270
1230 CHAIN = CHAIN + 1 : GOTO 1310
1240 CHAIN = CHAIN + 2 : GOTO 1320
1250 CHAIN = CHAIN + 4 : GOTO 1280
1260 CHAIN = CHAIN + 8 : GOTO 1300
1270 CHAIN = CHAIN + 16 : GOTO 1290
1280 CHAIN = CHAIN + 32 : GOTO 1340
1290 CHAIN = CHAIN + 64 : GOTO 1260
1300 CHAIN = CHAIN + 128 : GOTO 1230
1310 CHAIN = CHAIN + 256 : GOTO 1250
1320 CHAIN = CHAIN + 512 : GOTO 1330
1330 CHAIN = CHAIN + 1024 : GOTO 1350
1340 CHAIN = CHAIN + 2048 : GOTO 1240
1350 PRINT "   CHAIN ="; CHAIN
1355 REM  line continuation #3 -- same IF..THEN split pattern
1360 IF CHAIN = 4095 THEN
     PASS = PASS + 1 : GOTO 1400
1370 PRINT "   FAIL chain: expected 4095, got "; CHAIN
1380 FAIL = FAIL + 1

1400 REM ==== TEST 10: early GOTO exit from a FOR loop ====
1410 PRINT "10. Early GOTO exit from FOR loop:"
1420 SUM2 = 0
1430 FOR N2 = 1 TO 100
1440   SUM2 = SUM2 + N2
1445   IF SUM2 > 200 THEN GOTO 1470
1450 NEXT N2
1460 PRINT "   FAIL: loop ran to completion" : FAIL = FAIL + 1 : GOTO 1600
1470 PRINT "   Exited: N2="; N2; "  SUM2="; SUM2
1480 IF N2 = 20 AND SUM2 = 210 THEN PASS = PASS + 1 : GOTO 1600
1490 PRINT "   FAIL early-exit" : FAIL = FAIL + 1

1600 REM ==== TEST 11: Collatz sequence (27 -> 1 in 111 steps) ====
1610 PRINT "11. Collatz sequence (n=27):"
1620 NC = 27 : CSTEPS = 0
1630 IF NC = 1 THEN GOTO 1680
1640 CSTEPS = CSTEPS + 1
1650 IF NC MOD 2 = 0 THEN GOTO 1670
1660 NC = NC * 3 + 1 : GOTO 1630
1670 NC = INT(NC / 2) : GOTO 1630
1680 PRINT "   steps ="; CSTEPS
1690 IF CSTEPS = 111 THEN PASS = PASS + 1 : GOTO 1700
1695 PRINT "   FAIL collatz: expected 111, got "; CSTEPS : FAIL = FAIL + 1

1700 REM ==== TEST 12: out-of-order triangular ping-pong (A->B->C) ====
1710 PRINT "12. Out-of-order ping-pong (sections interleaved in source):"
1715 REM source order is A, C, B -- execution order is A->B->C->A
1720 ACNT = 0 : BCNT = 0 : CCNT = 0 : TOTAL = 0
1730 IF TOTAL >= 21 THEN GOTO 1770
1735 ACNT = ACNT + 1 : TOTAL = TOTAL + 1 : GOTO 1755
1740 CCNT = CCNT + 1 : TOTAL = TOTAL + 1 : GOTO 1730
1755 BCNT = BCNT + 1 : TOTAL = TOTAL + 1 : GOTO 1740
1770 PRINT "   A="; ACNT; " B="; BCNT; " C="; CCNT; " T="; TOTAL
1780 IF ACNT = 7 AND BCNT = 7 AND CCNT = 7 THEN PASS = PASS + 1 : GOTO 1800
1790 PRINT "   FAIL ping-pong" : FAIL = FAIL + 1

1800 REM ==== TEST 13: chained IF-GOTO vowel scanner ====
1810 PRINT "13. Chained IF-GOTO vowel scanner:"
1820 VW$ = "HELLO WORLD" : VWL = LEN(VW$) : VWI = 1 : VC = 0
1830 IF VWI > VWL THEN GOTO 1900
1840 TC$ = MID$(VW$, VWI, 1)
1850 IF TC$ = "A" THEN GOTO 1890
1860 IF TC$ = "E" THEN GOTO 1890
1870 IF TC$ = "I" THEN GOTO 1890
1875 IF TC$ = "O" THEN GOTO 1890
1880 IF TC$ = "U" THEN GOTO 1890
1885 GOTO 1895
1890 VC = VC + 1
1895 VWI = VWI + 1 : GOTO 1830
1900 PRINT "   Vowels in '"; VW$; "' ="; VC
1910 IF VC = 3 THEN PASS = PASS + 1 : GOTO 1920
1915 PRINT "   FAIL vowels: expected 3, got "; VC : FAIL = FAIL + 1

1920 REM ==== TEST 14: GCD via Euclidean algorithm ====
1930 PRINT "14. GCD via Euclidean GOTO loop:"
1940 NG1 = 252 : NG2 = 105
1950 IF NG2 = 0 THEN GOTO 1980
1960 GTMP = NG2 : NG2 = NG1 MOD NG2 : NG1 = GTMP
1970 GOTO 1950
1980 PRINT "   GCD(252,105) ="; NG1
1990 IF NG1 = 21 THEN PASS = PASS + 1 : GOTO 2000
1995 PRINT "   FAIL GCD: expected 21, got "; NG1 : FAIL = FAIL + 1

2000 REM ==== TEST 15: diamond-GOTO accumulator (3 paths converging) ====
2010 PRINT "15. Diamond-GOTO accumulator (1..20, 3 weighted paths):"
2020 MA = 1 : MP = 0
2030 IF MA > 20 THEN GOTO 2090
2040 IF MA MOD 2 = 0 THEN GOTO 2070
2050 IF MA MOD 3 = 0 THEN GOTO 2080
2060 MP = MP + MA : MA = MA + 1 : GOTO 2030
2070 MP = MP + MA * 2 : MA = MA + 1 : GOTO 2030
2080 MP = MP + MA * 3 : MA = MA + 1 : GOTO 2030
2090 PRINT "   MP ="; MP
2100 IF MP = 374 THEN PASS = PASS + 1 : GOTO 2110
2105 PRINT "   FAIL diamond: expected 374, got "; MP : FAIL = FAIL + 1

2110 REM ==== TEST 16: loop body re-entered from outside ====
2120 PRINT "16. Loop body re-entered from outside:"
2130 RVAL = 0 : RDONE = 0
2135 RI = 1 : GOTO 2150
2140 RI = 6
2150 RVAL = RVAL + RI
2160 RI = RI + 1
2170 IF RI <= 5 THEN GOTO 2150
2180 IF RDONE = 0 THEN RDONE = 1 : GOTO 2140
2190 PRINT "   RVAL ="; RVAL
2200 IF RVAL = 21 THEN PASS = PASS + 1 : GOTO 2210
2205 PRINT "   FAIL re-entry: expected 21, got "; RVAL : FAIL = FAIL + 1

2210 REM ==== TEST 17: binary search via GOTO ====
2220 PRINT "17. Binary search (sorted Fibonacci array, target=21):"
2230 BS(1)=2 : BS(2)=5 : BS(3)=8 : BS(4)=13 : BS(5)=21
2235 BS(6)=34 : BS(7)=55 : BS(8)=89
2240 BSLO=1 : BSHI=8 : BSTGT=21 : BSIDX=0
2250 IF BSLO > BSHI THEN GOTO 2300
2260 BSMID = INT((BSLO + BSHI) / 2)
2270 IF BS(BSMID) = BSTGT THEN BSIDX = BSMID : GOTO 2300
2280 IF BS(BSMID) < BSTGT THEN GOTO 2295
2290 BSHI = BSMID - 1 : GOTO 2250
2295 BSLO = BSMID + 1 : GOTO 2250
2300 PRINT "   Found at index"; BSIDX
2310 IF BSIDX = 5 THEN PASS = PASS + 1 : GOTO 9000
2320 PRINT "   FAIL binary search: expected index 5, got "; BSIDX
2330 FAIL = FAIL + 1

9000 REM ==== FINAL REPORT ====
9010 PRINT
9020 PRINT "RESULTS: PASS="; PASS; "  FAIL="; FAIL
9030 IF FAIL = 0 THEN PRINT "All tests passed!" : GOTO 9050
9040 PRINT "*** SOME TESTS FAILED ***"
9050 END

9100 REM == GOSUB: accumulate M into SUM ==
9110 SUM = SUM + M
9120 RETURN

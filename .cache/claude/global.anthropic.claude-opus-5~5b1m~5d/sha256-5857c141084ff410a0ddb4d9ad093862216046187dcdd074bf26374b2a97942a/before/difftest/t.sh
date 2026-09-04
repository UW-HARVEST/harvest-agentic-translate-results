#!/bin/bash
R=$HARVEST_WORKDIR/translation/target/release/driver
C=$HARVEST_WORKDIR/difftest/cref
fail=0
tests=("7 3" "-7 3" "7 -3" "-7 -3" "abc" "" "5" "5 " "+5 +2" "  12
   
 -4" "99999999999999999999999 5" "-99999999999999999999999 5" "4294967296 5" "-2147483648 -1" "10 0" "0 0" "x 5" "5 x" "-  5 2" "007 002" "2147483647 1" "9 4 7 extra" "
	
 8	
2" "5,3" "- 5" "+ " "12abc34" "0 5" "-0 3" "1
2" "  " "2147483648 3" "-2147483649 3" "18446744073709551617 3" "9223372036854775807 3" "-9223372036854775808 3" "0000000000000000000005 2")
for t in "${tests[@]}"; do
  printf '%s' "$t" > in.txt
  co=$("$C" < in.txt 2>/dev/null); cr=$?
  ro=$("$R" < in.txt 2>/dev/null); rr=$?
  if [ "$co" = "$ro" ] && [ "$cr" = "$rr" ]; then echo "OK   [$t] rc=$cr [$co]"; else echo "DIFF [$t] C(rc=$cr)[$co] RS(rc=$rr)[$ro]"; fail=1; fi
done
echo "fail=$fail"

#!/bin/bash
# Differential test: run every js file through both libraries in several modes.
cd "$(dirname "$0")" || exit 1
fail=0
count=0
run() {
	local name="$1"; shift
	./driver_c "$@" 2>&1 | sed -E 's/0x[0-9a-f]+/PTR/g' > "res/c_$name.txt"
	local ec=${PIPESTATUS[0]}
	./driver_r "$@" 2>&1 | sed -E 's/0x[0-9a-f]+/PTR/g' > "res/r_$name.txt"
	local er=${PIPESTATUS[0]}
	count=$((count+1))
	if [ "$ec" != "$er" ]; then
		echo "EXIT DIFF $name: c=$ec r=$er"; fail=1
	fi
	if ! diff -q "res/c_$name.txt" "res/r_$name.txt" > /dev/null; then
		echo "OUTPUT DIFF $name:"; diff "res/c_$name.txt" "res/r_$name.txt" | head -30; fail=1
	else
		echo "ok $name ($(wc -l < res/c_$name.txt) lines, exit $ec)"
	fi
}
mkdir -p res
run api -api
run lowlevel -lowlevel
run regexpapi -regexp
run ctx -ctx
for f in js/*.js; do
	b=$(basename "$f" .js)
	run "$b" "$f"
	run "${b}_strict" "$f" strict
	case "$b" in
	t*) run "${b}_dump" "$f" dumpstrings
	    run "${b}_limit" "$f" limit
	    run "${b}_mem" "$f" memlimit ;;
	esac
done
echo "--- $count comparisons"
if [ $fail = 0 ]; then echo "ALL DIFFERENTIAL TESTS IDENTICAL"; else echo "FAILURES PRESENT"; fi
exit $fail

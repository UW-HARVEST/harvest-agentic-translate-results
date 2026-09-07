#!/bin/sh
# Full differential verification: builds both libraries and runs every phase
# under every cargo feature combination.
#
#   cd translation && ./verify.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
root=$(dirname "$here")

echo "=== 1. build the C reference (exactly as the task specifies) ==="
mkdir -p "$root/c_src/build" "$here/target/vtmp"
(cd "$root/c_src/build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
ls -la "$root"/c_src/build/lib*.so

echo
echo "=== 2. build the Rust cdylib ==="
cd "$here"
cargo build --offline --release
cargo build --offline --examples
ls -la target/release/libload_png_mem_lib.so

echo
echo "=== 3. symbol diff ==="
nm -D --defined-only "$root"/c_src/build/lib*.so | awk '{print $NF}' | sort >"$here/target/vtmp/c.syms"
nm -D --defined-only target/release/libload_png_mem_lib.so | awk '{print $NF}' | sort >"$here/target/vtmp/rs.syms"
missing=$(comm -23 "$here/target/vtmp/c.syms" "$here/target/vtmp/rs.syms" || true)
if [ -n "$missing" ]; then
	echo "MISSING FROM RUST:"
	echo "$missing"
	exit 1
fi
echo "0 symbols missing from the Rust .so ($(wc -l <"$here/target/vtmp/c.syms") exported by C)"

echo
echo "=== 4. feature combinations ==="
# enumerate the [features] section of Cargo.toml (excluding "default")
features=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' Cargo.toml)
if [ -z "$features" ]; then
	echo "Cargo.toml declares no [features]; the only configuration is the default one."
	combos="<default> <no-default-features>"
else
	combos="<default> <no-default-features> $features"
fi
echo "combinations: $combos"

run() {
	echo "--- cargo test $* ---"
	cargo test --offline "$@" -- --test-threads=1
}

run
run --no-default-features
for f in $features; do
	run --no-default-features --features "$f"
	run --features "$f"
done

echo
echo "ALL PHASES PASSED FOR EVERY FEATURE COMBINATION"

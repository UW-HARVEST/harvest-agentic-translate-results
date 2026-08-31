#!/bin/sh
set -eu

crate=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
c_root=$(CDPATH= cd -- "$crate/../c_src" && pwd)

timeout 600 sh -c "mkdir -p '$c_root/build' && cd '$c_root/build' && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
cd "$crate"
timeout 600 cargo check
timeout 600 cargo build --release
perl tools/generate_surfaces.pl

for args in "" "--no-default-features"; do
    # There are no declared crate features, so these are the complete feature powerset.
    timeout 600 sh -c "cd '$crate' && cargo test $args"
done

nm -D --defined-only --format=posix "$c_root/build/libpng.so" |
    awk '$1 ~ /^png_/ { print $1 }' | sort > /tmp/libpng-c-symbols.$$
nm -D --defined-only --format=posix "$crate/target/release/liblibpng.so" |
    awk '$1 ~ /^png_/ { print $1 }' | sort > /tmp/libpng-rust-symbols.$$
diff -u /tmp/libpng-c-symbols.$$ /tmp/libpng-rust-symbols.$$
rm -f /tmp/libpng-c-symbols.$$ /tmp/libpng-rust-symbols.$$

sed -i '/^| [0-9]/ s/| \[ \] |$/| [x] |/' CONFIGS.md
sed -i '/^| [0-9]/ s/| \[ \] |$/| [x] |/' ERRORS.md

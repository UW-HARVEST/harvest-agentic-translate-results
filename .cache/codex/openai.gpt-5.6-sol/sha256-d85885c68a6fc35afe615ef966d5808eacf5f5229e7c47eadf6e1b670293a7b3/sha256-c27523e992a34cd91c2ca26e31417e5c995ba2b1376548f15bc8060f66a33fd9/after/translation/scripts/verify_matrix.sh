#!/usr/bin/env bash
set -u

crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
c_src_dir="$(cd "$crate_dir/../c_src" && pwd)"
matrix_dir="$c_src_dir/build-matrix"
log_dir="$crate_dir/matrix-logs"
openssl_include="/local/home/scheschb/.local/share/mise/installs/node/24.19.0/include/node"
crypto_dir="/usr/patching-agent/lib"

mkdir -p "$matrix_dir" "$log_dir"
: > "$log_dir/summary.txt"

backends=(haraka sha2 shake blake)
thashes=(robust simple)
params=(128f 128s 192f 192s 256f 256s)
failures=0
count=0

cd "$crate_dir" || exit 1
for backend in "${backends[@]}"; do
  for thash in "${thashes[@]}"; do
    for param in "${params[@]}"; do
      count=$((count + 1))
      combo="$backend-$thash-$param"
      features="$backend,$thash,$param"
      build_dir="$matrix_dir/$combo"
      log="$log_dir/$combo.log"
      backend_so="$build_dir/lib/$backend/lib$backend.so"
      core_so="$build_dir/app/libsphincs_core_det.so"
      driver="$build_dir/app/driver"
      printf 'START %02d %s\n' "$count" "$combo" | tee -a "$log_dir/summary.txt"
      : > "$log"

      if ! timeout 600 cmake -S "$c_src_dir" -B "$build_dir" \
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DHASH_BACKEND="$backend" -DSECPAR="$param" -DTHASH="$thash" \
        -DCMAKE_C_FLAGS="-I$openssl_include" \
        -DCMAKE_EXE_LINKER_FLAGS="-L$crypto_dir -Wl,-rpath,$crypto_dir" \
        >>"$log" 2>&1; then
        printf 'FAIL configure %s\n' "$combo" | tee -a "$log_dir/summary.txt"
        failures=$((failures + 1))
        continue
      fi
      if ! timeout 600 cmake --build "$build_dir" >>"$log" 2>&1; then
        printf 'FAIL c-build %s\n' "$combo" | tee -a "$log_dir/summary.txt"
        failures=$((failures + 1))
        continue
      fi
      if ! timeout 600 cargo build --release --no-default-features --features "$features" \
        >>"$log" 2>&1; then
        printf 'FAIL rust-build %s\n' "$combo" | tee -a "$log_dir/summary.txt"
        failures=$((failures + 1))
        continue
      fi

      {
        nm -D --defined-only "$core_so"
        nm -D --defined-only "$backend_so"
      } | awk '$2 ~ /^[TDBR]$/ {print $3}' | sort -u > "$log_dir/$combo.c-symbols"
      nm -D --defined-only target/release/libsphincs_plus_translation.so \
        | awk '$2 ~ /^[TDBR]$/ {print $3}' | sort -u > "$log_dir/$combo.rust-symbols"
      comm -23 "$log_dir/$combo.c-symbols" "$log_dir/$combo.rust-symbols" \
        > "$log_dir/$combo.missing"
      if [[ -s "$log_dir/$combo.missing" ]]; then
        printf 'FAIL symbols %s\n' "$combo" | tee -a "$log_dir/summary.txt"
        failures=$((failures + 1))
        continue
      fi

      if ! SPHINCS_C_CORE="$core_so" \
        SPHINCS_C_BACKEND="$backend_so" \
        SPHINCS_C_DRIVER="$driver" \
        timeout 600 cargo test --release --no-default-features --features "$features" \
          --test differential -- --test-threads=1 >>"$log" 2>&1; then
        printf 'FAIL tests %s\n' "$combo" | tee -a "$log_dir/summary.txt"
        failures=$((failures + 1))
        continue
      fi
      printf 'PASS %s\n' "$combo" | tee -a "$log_dir/summary.txt"
    done
  done
done

printf 'TOTAL=%d FAILURES=%d\n' "$count" "$failures" | tee -a "$log_dir/summary.txt"
exit "$failures"

#!/usr/bin/env bash
# Mutation check: prove the differential suite actually detects divergence.
#
# Each mutant is a small, deliberate deviation from the C semantics, built as a
# stand-alone cdylib exporting the same symbol. The suite is pointed at the
# mutant via HARVEST_RUST_SO and MUST fail. A mutant that passes means the
# corresponding behavior is untested.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

mutants=(
  "unsigned_h_div:s|let flips: c_int = h / 2;|let flips: c_int = ((h as u32) / 2) as c_int;|"
  "off_by_one_rows:s|w.wrapping_mul(h.wrapping_sub(i).wrapping_sub(1))|w.wrapping_mul(h.wrapping_sub(i).wrapping_sub(2))|"
  "drop_alpha:s|let t: cp_pixel_t = \\*a;|let t: cp_pixel_t = *a; let keep_a = (*a).a; let _ = keep_a;|;s|\\*b = t;|*b = cp_pixel_t { a: (*b).a, ..t };|"
  "skip_last_column:s|while j < w {|while j + 1 < w {|"
  "one_extra_flip:s|let flips: c_int = h / 2;|let flips: c_int = h / 2 + 1;|"
  "inner_ge_zero:s|while j < w {|while j != w {|"
)

fail=0
for entry in "${mutants[@]}"; do
  name="${entry%%:*}"
  expr="${entry#*:}"
  d="$WORK/$name"
  mkdir -p "$d/src"
  sed -e "$expr" src/lib.rs > "$d/src/lib.rs"
  if cmp -s src/lib.rs "$d/src/lib.rs"; then
    echo "MUTANT $name: SED DID NOT APPLY (harness bug)"
    fail=1
    continue
  fi
  cat > "$d/Cargo.toml" <<EOF
[package]
name = "mutant_$name"
version = "0.1.0"
edition = "2021"
[lib]
crate-type = ["cdylib"]
path = "src/lib.rs"
name = "flip_horizontal_lib"
[profile.release]
panic = "abort"
[workspace]
EOF
  if ! (cd "$d" && cargo build --release >/dev/null 2>&1); then
    echo "MUTANT $name: did not compile (skipped)"
    continue
  fi

  so="$d/target/release/libflip_horizontal_lib.so"
  out="$(HARVEST_RUST_SO="$so" timeout 600 cargo test 2>&1)"
  rc=$?
  nfail="$(printf '%s' "$out" | grep -cE '^test .* FAILED')"
  crash="$(printf '%s' "$out" | grep -cE 'signal: [0-9]+|process didn.t exit successfully')"
  if [ "$rc" -ne 0 ]; then
    echo "MUTANT $name: CAUGHT (suite rc=$rc, ${nfail} assertion failures, ${crash} abnormal terminations)"
  else
    echo "MUTANT $name: *** SURVIVED — behavior is not covered ***"
    fail=1
  fi
done

exit "$fail"

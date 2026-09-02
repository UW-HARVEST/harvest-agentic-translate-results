#!/usr/bin/env bash
# Mutation check: proves the differential suite is not vacuous.
#
# Each MUTANT is a sed expression applied to src/lib.rs that changes OBSERVABLE
# behaviour; the suite MUST fail for it. Each CONTROL is a behaviour-preserving
# edit; the suite MUST still pass. src/lib.rs is always restored.
#
# sed expressions use `#` as the delimiter because the Rust source contains `|`.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(cd "$here/.." && pwd)"
cd "$crate"

ulimit -c 0 || true
backup="$(mktemp)"
cp src/lib.rs "$backup"
restore() { cp "$backup" src/lib.rs; cargo build --release >/dev/null 2>&1; rm -f "$backup"; }
trap restore EXIT

run_suite() { # 0 = suite passed, 1 = suite failed
  cargo build --release >/dev/null 2>&1 || return 1
  cargo build          >/dev/null 2>&1 || return 1
  cargo test --release >/dev/null 2>&1
}

declare -a NAMES SEDS KINDS
add() { NAMES+=("$1"); KINDS+=("$2"); SEDS+=("$3"); }

# --- mutants: must be DETECTED ---------------------------------------------
add "sector-4 channel swap"          mutant 's#4 => (t, p, v),#4 => (t, v, p),#'
add "sector-0 channel swap"          mutant 's#0 => (v, t, p),#0 => (q, v, p),#'
add "default-arm channel swap"       mutant 's#_ => (v, p, q),#_ => (v, q, p),#'
add "floor -> trunc"                 mutant 's#x.floor()#x.trunc()#'
add "saturating float->int cast"     mutant 's#    if x >= -2147483648.0f32 \&\& x < 2147483648.0f32 {#    if true {#'
add "cast guard boundary off by one" mutant 's#x < 2147483648.0f32#x <= 2147483648.0f32#'
add "s==0 guard -> epsilon"          mutant 's#    if s == 0.0 {#    if s.abs() < 1e-30 {#'
add "hue scale 60 -> 60.0001"        mutant 's#c_div(h, 60.0f32)#c_div(h, 60.0001f32)#'
add "divide -> reciprocal multiply"  mutant 's#c_div(h, 60.0f32)#c_mul(h, 1.0f32 / 60.0f32)#'
add "p algebraic rewrite"            mutant 's#let p: f32 = c_mul(c_sub(1.0f32, s), v);#let p: f32 = c_sub(v, c_mul(v, s));#'
add "q algebraic rewrite"            mutant 's#let q: f32 = c_mul(c_sub(1.0f32, c_mul(s, f)), v);#let q: f32 = c_sub(v, c_mul(v, c_mul(s, f)));#'
add "t via f64 intermediates"        mutant 's#let t: f32 = c_mul(c_sub(1.0f32, c_mul(c_sub(1.0f32, f), s)), v);#let t: f32 = (v as f64 * (1.0f64 - s as f64 * (1.0f64 - f as f64))) as f32;#'
add "match uses signed range"        mutant 's#        0 => (v, t, p),#        i32::MIN..=-1 => (v, t, p),\n        0 => (v, t, p),#'
# NaN-propagation emulation
add "c_mul NaN preference swapped"   mutant '/^fn c_mul/,/^}/ s#        quiet(a)#        quiet(b)#'
add "c_sub NaN preference swapped"   mutant '/^fn c_sub/,/^}/ s#        quiet(a)#        quiet(b)#'
add "c_div NaN preference swapped"   mutant '/^fn c_div/,/^}/ s#        quiet(a)#        quiet(b)#'
add "no sNaN quieting in quiet()"    mutant 's#f32::from_bits(x.to_bits() | 0x0040_0000)#x#'
add "quiet() also clears sign"       mutant 's#f32::from_bits(x.to_bits() | 0x0040_0000)#f32::from_bits((x.to_bits() | 0x0040_0000) \& 0x7fff_ffff)#'
add "p operands reversed"            mutant 's#let p: f32 = c_mul(c_sub(1.0f32, s), v);#let p: f32 = c_mul(v, c_sub(1.0f32, s));#'
add "q outer operands reversed"      mutant 's#let q: f32 = c_mul(c_sub(1.0f32, c_mul(s, f)), v);#let q: f32 = c_mul(v, c_sub(1.0f32, c_mul(s, f)));#'
add "q inner operands reversed"      mutant 's#c_mul(s, f)#c_mul(f, s)#'
add "t outer operands reversed"      mutant 's#c_sub(1.0f32, c_mul(c_sub(1.0f32, f), s)), v)#v, c_sub(1.0f32, c_mul(c_sub(1.0f32, f), s)))#'
add "f = i - h instead of h - i"     mutant 's#c_sub(h, i as f32)#c_sub(i as f32, h)#'

# --- controls: must still PASS ---------------------------------------------
# Behaviour-preserving refactors.
add "core::i32::MIN alias"           control 's#        i32::MIN#        core::i32::MIN#'
add "quiet() mask via shift"         control 's#f32::from_bits(x.to_bits() | 0x0040_0000)#f32::from_bits(x.to_bits() | (1u32 << 22))#'
# Verified no-ops, kept as controls so a future change that makes them
# observable is noticed:
#  * Rust f32::floor and C floorf are byte-identical on every NaN encoding
#    (both quiet an sNaN, preserving sign and payload) — checked directly.
add "c_floorf drops NaN branch"      control 's#    if x.is_nan() { quiet(x) } else { x.floor() }#    x.floor()#'
#  * `t` is only read from switch arms 0/2/4, which require a finite `h`, so
#    `f` (hence `1.0 - f`) is never NaN where `t` is observable; the inner
#    operand order therefore cannot be witnessed.
add "t inner operands reversed"      control 's#c_mul(c_sub(1.0f32, f), s)#c_mul(s, c_sub(1.0f32, f))#'

pass=0; fail=0; skipped=0
for i in "${!NAMES[@]}"; do
  name="${NAMES[i]}"; kind="${KINDS[i]}"; sed_expr="${SEDS[i]}"
  cp "$backup" src/lib.rs
  if ! sed -i "$sed_expr" src/lib.rs 2>/dev/null; then
    echo "SKIP  $name (sed error)"; fail=$((fail+1)); skipped=$((skipped+1)); continue
  fi
  if cmp -s "$backup" src/lib.rs; then
    echo "SKIP  $name (pattern matched nothing — mutant list is stale)"
    fail=$((fail+1)); skipped=$((skipped+1)); continue
  fi
  if run_suite; then result=pass; else result=fail; fi

  case "$kind:$result" in
    mutant:fail)  printf 'OK    [mutant]  detected:      %s\n' "$name"; pass=$((pass+1)) ;;
    mutant:pass)  printf 'BAD   [mutant]  NOT detected:  %s\n' "$name"; fail=$((fail+1)) ;;
    control:pass) printf 'OK    [control] still passes:  %s\n' "$name"; pass=$((pass+1)) ;;
    control:fail) printf 'BAD   [control] false positive: %s\n' "$name"; fail=$((fail+1)) ;;
  esac
done

cp "$backup" src/lib.rs
cargo build --release >/dev/null 2>&1
cargo build          >/dev/null 2>&1
echo
echo "mutation check: $pass ok, $fail bad, $skipped skipped (of ${#NAMES[@]})"
(( fail == 0 )) || exit 1
echo "the differential suite is sensitive to real behavioural changes ✅"

#!/usr/bin/env bash
# Anti-vacuity check: inject a known divergence into the Rust translation, confirm
# the differential suite FAILS, then restore. A mutation that survives means the
# corresponding assertion is vacuous.
#
# Each entry is:  <label>|<features>|<test target>|<file>|<sed expression>
set -u
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here/translation"

backup=$(mktemp -d)
mkdir -p "$backup/src"
cp -r src/. "$backup/src/"
restore() { rm -rf src && mkdir src && cp -r "$backup/src/." src/; }
trap 'restore; rm -rf "$backup"' EXIT

MUTATIONS=(
  "op_add returns a-b|add,5|phase_b_valid|src/mdcore.rs|s/    a.wrapping_add(b)/    a.wrapping_sub(b)/"
  "op_mul off by one|mul,5|phase_b_valid|src/mdcore.rs|s/    a.wrapping_mul(b)/    a.wrapping_mul(b).wrapping_add(1)/"
  "DISPATCH_REP gains case 7|add,7|phase_c_errors|src/mdmacros.rs|s/        _ => acc,/        7 => rep(acc, 7),\n        _ => acc,/"
  "DISPATCH_REP default runs rep|sub,5|phase_c_errors|src/mdmacros.rs|s/        _ => acc,/        other => rep(acc, other.max(0)),/"
  "REP loop is inclusive|add,5|phase_b_valid|src/mdmacros.rs|s/    while i < n {/    while i <= n {/"
  "STR(OP) mis-cased|sub,5|phase_b_valid|src/mdmacros.rs|s/b\"sub\\\\0\"/b\"SUB\\\\0\"/"
  "INIT_mul wrong|mul,4|phase_b_valid|src/mdmacros.rs|s/^pub const INIT: c_int = 1;/pub const INIT: c_int = 2;/"
  "G_OP always op_add|mul,5|phase_b_valid|src/mdcore.rs|s/pub static mut G_OP: extern \"C\" fn(c_int, c_int) -> c_int = OP_FN;/pub static mut G_OP: extern \"C\" fn(c_int, c_int) -> c_int = op_add;/"
  "use_generated export renamed|add,5|phase_d_symbols|src/mdcore.rs|PERL:s/#\\[unsafe\\(no_mangle\\)\\]\\npub extern \"C\" fn use_generated/#[export_name = \"use_generated_MUTATED\"]\\npub extern \"C\" fn use_generated/"
  "helper_ptr export renamed|sub,2|phase_d_symbols|src/mdcore.rs|PERL:s/#\\[unsafe\\(no_mangle\\)\\]\\npub extern \"C\" fn helper_ptr/#[export_name = \"helper_ptr2\"]\\npub extern \"C\" fn helper_ptr/"
  "G_OP_NAME export dropped|add,5|phase_d_symbols|src/mdcore.rs|PERL:s/#\\[unsafe\\(no_mangle\\)\\]\\npub static G_OP_NAME/#[export_name = \"G_OP_NAME_X\"]\\npub static G_OP_NAME/"
  "printf text changed|add,3|phase_b_valid|src/mdcore.rs|s/helper.ptr=/helper.PTR=/"
  "atoi neg-overflow regression|add,5|phase_c_errors|src/cstdlib.rs|s/            i64::MIN/            i64::MIN + 1/"
  "main summary not wrapping|add,5|phase_b_exe|src/main.rs|s/        .wrapping_add(g);/        .wrapping_add(g).wrapping_add(1);/"
)

pass=0
surv=0
for entry in "${MUTATIONS[@]}"; do
  IFS='|' read -r label feats target file expr <<<"$entry"
  restore
  if [[ "$expr" == PERL:* ]]; then
    perl -0pi -e "${expr#PERL:}" "$file"
  else
    sed -i "$expr" "$file"
  fi
  if diff -q "$backup/$file" "$file" >/dev/null 2>&1; then
    printf 'SKIP     %-32s (mutation did not apply)\n' "$label"
    continue
  fi
  # Build the mutated cdylib explicitly; `cargo test` never builds one, and the
  # harness must not shell out to cargo while cargo is already running.
  if ! so=$(timeout 600 ./build_so.sh --no-default-features --features "$feats" 2>/tmp/mut_build.log); then
    printf 'SKIP     %-32s (mutation broke the build)\n' "$label"
    continue
  fi
  if MD_RUST_SO="$so" timeout 600 cargo test --no-default-features --features "$feats" \
       --test "$target" >/tmp/mut.log 2>&1; then
    printf 'SURVIVED %-32s [%s %s] -- assertion is vacuous!\n' "$label" "$feats" "$target"
    surv=$((surv+1))
  elif grep -qE '^error(\[|:)' /tmp/mut.log && ! grep -q 'test result: FAILED' /tmp/mut.log; then
    printf 'SKIP     %-32s (mutation broke the build, not a behavioural test)\n' "$label"
  else
    printf 'caught   %-32s [%s %s]\n' "$label" "$feats" "$target"
    pass=$((pass+1))
  fi
done

restore
echo
echo "mutations caught: $pass   survived: $surv"
[[ $surv -eq 0 ]]

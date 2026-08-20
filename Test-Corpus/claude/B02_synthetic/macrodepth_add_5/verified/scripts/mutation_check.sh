#!/bin/bash
# Sanity check that the differential tests are not vacuous: inject a small
# behavioural change into the Rust source, rebuild, and require the tests to
# FAIL. Always restores the sources afterwards.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT" || exit 1
cp src/mdcore.rs "$TMPDIR/mdcore.rs.bak"
cp src/mdmacros.rs "$TMPDIR/mdmacros.rs.bak"
cp src/main.rs "$TMPDIR/main.rs.bak"
restore() { cp "$TMPDIR/mdcore.rs.bak" src/mdcore.rs; cp "$TMPDIR/mdmacros.rs.bak" src/mdmacros.rs; cp "$TMPDIR/main.rs.bak" src/main.rs; }
trap restore EXIT

overall=0
try() { # name  sed-file  sed-expr  test-filter
  local name="$1" file="$2" expr="$3" filter="$4"
  restore
  sed -i "$expr" "$file" || { echo "?? sed failed for $name"; return; }
  if ! bash scripts/build_artifacts.sh "add,5" >/dev/null 2>&1; then
    echo "?? build failed for mutation '$name' (mutation invalid)"; return
  fi
  if timeout 600 cargo test --offline --quiet --no-default-features --features add,5 \
       --test differential -- --test-threads=1 "$filter" >/dev/null 2>&1; then
    echo "VACUOUS: mutation '$name' was NOT detected by $filter"
    overall=1
  else
    echo "detected: mutation '$name'  (via $filter)"
  fi
}

try "dispatch_rep also accepts 7"  src/mdmacros.rs 's/0 | 1 | 2 | 3 | 4 | 5 | 6 => rep_n(acc, n)/0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 => rep_n(acc, n)/' c04
try "op_mul uses wrapping_add"     src/mdcore.rs   's/a.wrapping_mul(b)/a.wrapping_add(b)/' b0
try "op_add saturates"             src/mdcore.rs   's/a.wrapping_add(b)/a.saturating_add(b)/' b0
try "step_op add uses i+1"         src/mdmacros.rs 's/Op::Add => acc.wrapping_add(i)/Op::Add => acc.wrapping_add(i + 1)/' b0
try "helper_call drops a space"    src/mdcore.rs   's/helper.call={} helper.acc={}/helper.call={}  helper.acc={}/' b05
try "helper_ptr label typo"        src/mdcore.rs   's/helper.ptr={}/helper.Ptr={}/' b07
try "gen.acc label typo"           src/mdcore.rs   's/gen.acc={}/gen.Acc={}/' c04
try "G_OP points at op_sub"        src/mdcore.rs   's/Op::Add => op_add,/Op::Add => op_sub,/' b1
try "G_OP_NAME says addd"          src/mdmacros.rs 's/Op::Add => b"add\\0",/Op::Add => b"addd\\0",/' b15
try "argc guard off by one"        src/main.rs     's/if argc < 3 {/if argc < 2 {/' c0
try "usage goes to stdout"         src/main.rs     's/std::io::stderr().lock().write_all(&msg)/std::io::stdout().lock().write_all(\&msg)/' c01
try "usage message wording"        src/main.rs     's/msg.extend_from_slice(b"usage: ");/msg.extend_from_slice(b"usage:  ");/' c01
try "c_printf reports errors"      src/mdcore.rs   's/    let _ = std::io::stdout().lock().write_fmt(args);/    std::io::stdout().lock().write_fmt(args).unwrap();/' c22
try "argv0 lossy conversion"       src/main.rs     's/args.first().map(arg_bytes)/args.first().map(|a| a.to_string_lossy().into_owned().into_bytes())/' c21
try "exit code 1 instead of 2"     src/main.rs     's/ExitCode::from(2)/ExitCode::from(1)/' c01
try "atoi ignores sign"            src/main.rs     's/negative = s\[i\] == b.-.;/negative = false;/' b20
try "atoi no overflow saturation"  src/main.rs     's/None => overflowed = true,/None => acc = acc.wrapping_mul(10).wrapping_add(digit),/' c14
try "G_OP no longer in .data"      src/mdcore.rs   's/#\[unsafe(link_section = ".data")\]//' b14

restore
bash scripts/build_artifacts.sh "add,5" >/dev/null 2>&1
echo "---"
[ $overall -eq 0 ] && echo "ALL MUTATIONS DETECTED (tests are not vacuous)" || echo "SOME MUTATIONS UNDETECTED"
exit $overall

#!/usr/bin/env bash
# Runs the differential suite across every build configuration, and (with
# --mutants) validates the suite itself with a mutant battery.
#
#   ./run_all_configs.sh            # feature/profile matrix
#   ./run_all_configs.sh --mutants  # matrix + mutation testing of the harness
#
# Feature enumeration: Cargo.toml has no [features] section, so the complete set
# of valid feature combinations is the single empty combination.  It is still
# exercised both implicitly (plain `cargo test`) and explicitly
# (`--no-default-features`), and in both profiles.

set -uo pipefail
cd "$(dirname "$0")"

TMP="${TMPDIR:-/tmp}/driver-verify"
mkdir -p "$TMP"
FAILED=0

# ---------------------------------------------------------------------------
# Phase A: enumerate the build-time configuration space, mechanically.
# ---------------------------------------------------------------------------
echo "== Feature enumeration =="
if grep -q '^\[features\]' Cargo.toml; then
    echo "  Cargo.toml declares [features]:"
    sed -n '/^\[features\]/,/^\[/p' Cargo.toml | sed '1d;$d' | sed 's/^/    /'
    echo "  !! run_all_configs.sh must be extended to cross-product them"
    FAILED=1
else
    echo "  no [features] in Cargo.toml -> exactly one feature combination (empty)"
fi
if grep -qE '^\s*option\(|target_compile_definitions' c_src/CMakeLists.txt; then
    echo "  !! CMakeLists.txt declares build options -- extend this script"
    FAILED=1
else
    echo "  no option()/definitions in c_src/CMakeLists.txt -> one C configuration"
fi
if grep -qE '^\s*#\s*(if|ifdef|ifndef|define)' c_src/src/main.c; then
    echo "  !! main.c has preprocessor conditionals -- extend this script"
    FAILED=1
else
    echo "  no #ifdef in c_src/src/main.c -> no conditional C code paths"
fi
echo

# ---------------------------------------------------------------------------
# Build the C reference (the canonical CMake configuration).
# ---------------------------------------------------------------------------
echo "== Building the C reference with CMake =="
cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON > "$TMP/cmake.log" 2>&1 \
  && cmake --build c_src/build >> "$TMP/cmake.log" 2>&1 \
  || { echo "  C build FAILED (see $TMP/cmake.log)"; tail -20 "$TMP/cmake.log"; exit 1; }
echo "  ok: $(./c_src/build/driver </dev/null | head -1)"
echo

# ---------------------------------------------------------------------------
# Phases B-D across the whole matrix.
# ---------------------------------------------------------------------------
run_matrix() {
    for feat in "default" "no-default-features"; do
        for prof in "debug" "release"; do
            local args=()
            [[ $feat == "no-default-features" ]] && args+=(--no-default-features)
            [[ $prof == "release" ]] && args+=(--release)
            echo "== cargo check [$feat/$prof] =="
            if ! timeout 300 cargo check --offline --all-targets "${args[@]}" \
                    > "$TMP/check-$feat-$prof.log" 2>&1; then
                echo "  CHECK FAILED"; tail -30 "$TMP/check-$feat-$prof.log"; FAILED=1; continue
            fi
            grep -c "^warning" "$TMP/check-$feat-$prof.log" | sed 's/^/  warnings: /'
            echo "== cargo test  [$feat/$prof] =="
            if timeout 600 cargo test --offline "${args[@]}" \
                    > "$TMP/test-$feat-$prof.log" 2>&1; then
                grep -h "^test result:" "$TMP/test-$feat-$prof.log" | sed 's/^/  /'
            else
                echo "  TEST FAILED"; grep -E "^(test .*FAILED|failures:|thread)" \
                    "$TMP/test-$feat-$prof.log" | head -30; FAILED=1
            fi
        done
    done
}
run_matrix

# ---------------------------------------------------------------------------
# Mutation testing: every mutant must be KILLED by the suite.  A surviving
# mutant means the tests have a blind spot.  The mutant is substituted for the
# C reference via $C_DRIVER, so "tests fail" == "difference detected".
# ---------------------------------------------------------------------------
if [[ "${1:-}" == "--mutants" ]]; then
    echo
    echo "== Mutation testing (each mutant MUST be killed) =="
    MUT="$TMP/mutants"; rm -rf "$MUT"; mkdir -p "$MUT"
    python3 - "$MUT" <<'PY'
import sys, pathlib
out = pathlib.Path(sys.argv[1])
src = pathlib.Path("src/main.rs").read_text()
def mut(name, old, new):
    assert old in src, f"mutation {name}: pattern not found"
    (out / f"{name}.rs").write_text(src.replace(old, new, 1))

mut("no_sigpipe",   "    restore_default_sigpipe();", "    // mutant: no SIGPIPE restore")
mut("wrap_no_sat",  """        let clamped: i64 = if signed > i128::from(i64::MAX) {
            i64::MAX
        } else if signed < i128::from(i64::MIN) {
            i64::MIN
        } else {
            signed as i64
        };""", "        let clamped: i64 = signed as i64;")
mut("clamp_i32",    "        Some(clamped as i32)",
                    "        Some(clamped.clamp(i32::MIN as i64, i32::MAX as i64) as i32)")
mut("ok_text",      'print_str(out, "Ok!\\n");', 'print_str(out, "OK!\\n");')
mut("no_epilogue",  'print_str(out, "Operation failed\\n");', "// mutant: no epilogue")
mut("result_code",  "        Err(code) => code,", "        Err(code) => code + 1,")
mut("y_default",    "Globals { y: 123 }", "Globals { y: 2 }")
mut("stage_order",  """        if x != 1 {
            print_str(out, "Error: x != 1\\n");
            return Err(1);
        }

        if g.y != 2 {""",
                    """        if g.y != 2 {
            print_str(out, "Error: x == 1 but y != 2\\n");
            return Err(2);
        }

        if x != 1 {""")
mut("ws_set",       "matches!(b, b' ' | b'\\t' | b'\\n' | 0x0b | 0x0c | b'\\r')",
                    "matches!(b, b' ' | b'\\t' | b'\\n' | b'\\r')")
mut("no_plus",      "            b'+' => self.next_byte(),", "            b'+' => None,")
mut("no_stdin_sync", "    reader.src.sync();", "    // mutant: no exit-time stdin sync")
mut("bufsiz_always", """                let bs = md.blksize() as usize;
                if bs > 0 && bs < BUFSIZ {
                    bs
                } else {
                    BUFSIZ
                }""", "                let _ = md.blksize();\n                BUFSIZ")
mut("no_unread_pos",  "        debug_assert!(self.pos > 0);\n        self.pos -= 1;", "        // mutant: unread does nothing")
mut("neg_sat_to_m1", """        } else if signed < i128::from(i64::MIN) {
            i64::MIN""", """        } else if signed < i128::from(i64::MIN) {
            i64::MAX""")
mut("keep_scanning","""    if let Some(v) = reader.scan_i32() {
        x = v;
        if let Some(v) = reader.scan_i32() {
            g.y = v;
            if let Some(v) = reader.scan_i32() {
                z = v;
            }
        }
    }""",
                    """    if let Some(v) = reader.scan_i32() { x = v; }
    if let Some(v) = reader.scan_i32() { g.y = v; }
    if let Some(v) = reader.scan_i32() { z = v; }""")
mut("no_unget",     "                self.unget(c);\n                break;", "                break;")
mut("result_fmt",   'writeln!(out, "Result: {}", result)', 'write!(out, "Result: {}", result)')
print("\n".join(sorted(p.stem for p in out.glob("*.rs"))))
PY
    [[ $? -ne 0 ]] && { echo "  mutant generation FAILED"; exit 1; }

    # Mutants that are PROVABLY unobservable through this program's output, so
    # surviving is the correct outcome.  The program only ever reveals whether a
    # scanned value equals 1 (x), 2 (y) or 3 (z):
    #
    #  keep_scanning  - scanning on after a failed conversion instead of
    #    aborting the whole scanf call.  If conversion i is the first to fail,
    #    variable i keeps its default (x=0, y=123, z=0) and every default fails
    #    its OWN stage check (0!=1, 123!=2, 0!=3).  Conversions before i are
    #    identical, and multi_stage returns at stage i, before any later
    #    variable is read -- so the extra assignments can never be observed.
    #
    #  neg_sat_to_m1  - saturating negative overflow to LONG_MAX (-1 after
    #    narrowing) instead of LONG_MIN (0).  Neither 0 nor -1 equals 1, 2 or 3,
    #    so both produce the same rejection for every input.
    #
    # (The real translation still matches glibc exactly in both places; these
    # entries only document the resolution limit of the available oracle.)
    EQUIVALENT=" keep_scanning neg_sat_to_m1 "

    SURVIVORS=()
    for m in "$MUT"/*.rs; do
        name=$(basename "$m" .rs)
        if ! rustc --edition 2021 -O -o "$MUT/$name" "$m" > "$MUT/$name.build.log" 2>&1; then
            echo "  $name: does not compile -- skipped"; continue
        fi
        if C_DRIVER="$MUT/$name" timeout 600 cargo test --offline \
              --test configs --test errors --test fds --test scan_semantics \
              --test fuzz_stream --test stdin_offset \
              > "$MUT/$name.test.log" 2>&1; then
            if [[ $EQUIVALENT == *" $name "* ]]; then
                echo "  $name: survived (PROVABLY EQUIVALENT -- expected)"
            else
                echo "  $name: SURVIVED  <-- blind spot!"
                SURVIVORS+=("$name")
                FAILED=1
            fi
        else
            killers=$(grep -c "FAILED" "$MUT/$name.test.log")
            if [[ $EQUIVALENT == *" $name "* ]]; then
                echo "  $name: killed ($killers tests) <-- was documented as equivalent; fix the docs"
                FAILED=1
            else
                echo "  $name: killed ($killers failing tests)"
            fi
        fi
    done
    if (( ${#SURVIVORS[@]} )); then
        echo "  unexpected survivors: ${SURVIVORS[*]}"
    else
        echo "  all non-equivalent mutants killed"
    fi
fi

echo
if (( FAILED )); then
    echo "RESULT: FAILURES (see $TMP)"
    exit 1
fi
echo "RESULT: all configurations verified"

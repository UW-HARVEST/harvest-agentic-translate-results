#!/usr/bin/env bash
# Harness self-validation: deliberately break the Rust translation in several
# distinct places and confirm the differential tests CATCH each one. A test
# suite that passes a mutated translation is not testing anything.
#
# Always restores src/lib.rs.
set -uo pipefail
cd "$(dirname "$0")"

SRC=src/lib.rs
BAK="$(mktemp)"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; }
trap restore EXIT

LOGS=target/logs; mkdir -p "$LOGS"
fail=0

# name | sed expression | test selector | feature args
run_mutant() {
  local name="$1" sedexpr="$2" selector="$3"; shift 3
  local featargs=("$@")

  cp "$BAK" "$SRC"
  if ! sed -i "$sedexpr" "$SRC"; then
    printf '\033[31mFAIL\033[0m %-34s could not apply mutation\n' "$name"; fail=1; return
  fi
  if cmp -s "$BAK" "$SRC"; then
    printf '\033[31mFAIL\033[0m %-34s mutation was a no-op (pattern not found)\n' "$name"; fail=1; return
  fi

  local log="$LOGS/mutant-${name}.log"
  if timeout 600 cargo test --release "${featargs[@]}" --test "$selector" \
        > "$log" 2>&1; then
    printf '\033[31mFAIL\033[0m %-34s NOT caught — tests passed a broken translation!\n' "$name"
    fail=1
  else
    local n; n=$(grep -c 'DIVERGENCE\|assertion .* failed\|panicked' "$log")
    printf '\033[32mok\033[0m   %-34s caught (%s failure lines)\n' "$name" "$n"
  fi
}

echo "== Mutation testing the differential harness =="

# --- default-feature surface --------------------------------------------
run_mutant "mode3_metric_constant"  's/metric.wrapping_mul(2).wrapping_add(0o10)/metric.wrapping_mul(2).wrapping_add(0o11)/' soak
run_mutant "mode3_flag_mask"        's/result.wrapping_add(flags & 0o177)/result.wrapping_add(flags \& 0o77)/'                soak
run_mutant "mode1_error_code"       's/return STATUS_ERROR | 0o20;/return STATUS_ERROR | 0o21;/'                phase_c_empty
run_mutant "mode2_error_code"       's/return STATUS_ERROR | 0o40;/return STATUS_ERROR | 0o41;/'                phase_c_empty
run_mutant "mode4_error_code"       's/return STATUS_ERROR | 0o100;/return STATUS_ERROR | 0o101;/'              phase_c_empty
run_mutant "default_error_code"     's/result = STATUS_ERROR | 0o200;/result = STATUS_ERROR | 0o201;/'          phase_c_empty
run_mutant "sprintf_literal"        's/b"_Depth_".as_slice()/b"_Dept_".as_slice()/'                             soak
run_mutant "intstr_zero_case"       's/tmp\[n\] = b.0.;/tmp[n] = b'"'"'0'"'"'; n += 0;/'                        soak

# --- feature surface (populated node_storage) ---------------------------
F=(--no-default-features --features expose_init_test_data)
run_mutant "mode1_parent_weight"    's/(\*parent_node).value } \* 1.5/(*parent_node).value } * 1.6/'            phase_b_tree "${F[@]}"
run_mutant "mode2_temp_multiplier"  's/i.wrapping_mul(0o7)/i.wrapping_mul(0o6)/'                                phase_b_tree "${F[@]}"
run_mutant "mode2_array_size"       's/array_size = 0o20;/array_size = 0o17;/'                                  phase_b_tree "${F[@]}"
run_mutant "mode4_e_constant"       's/\* 2.718281828/* 2.718281829/'                                           phase_b_tree "${F[@]}"
run_mutant "mode4_depth_scale"      's/1.0 + (depth as c_double) \* 0.1/1.0 + (depth as c_double) * 0.11/'      phase_b_tree "${F[@]}"
run_mutant "mode4_scan_count"       's/while i < 3 \&\& iter > base/while i < 2 \&\& iter > base/'               phase_b_tree "${F[@]}"
run_mutant "mode4_count_threshold"  's/if unsafe { NODE_COUNT } > 2 {/if unsafe { NODE_COUNT } > 200 {/'        phase_b_tree "${F[@]}"
run_mutant "add_node_data_values"   's/(\*slot).data\[2\] = 0o300;/(*slot).data[2] = 0o301;/'                    phase_b_tree "${F[@]}"
run_mutant "init_node_value"        's/add_node(6, 3, 40.0625);/add_node(6, 3, 40.0626);/'                      phase_b_tree "${F[@]}"
run_mutant "init_parent_link"       's/add_node(7, 4, 12.5);/add_node(7, 2, 12.5);/'                            phase_b_tree "${F[@]}"
run_mutant "saturate_high_clamp"    's/if value > 2147483647.0 {/if value > 2147483648.0 {/'                    phase_c_tree "${F[@]}"
run_mutant "saturate_low_clamp"     's/if value < -2147483648.0 {/if value < -2147483647.0 {/'                  phase_c_tree "${F[@]}"
run_mutant "find_node_scan_bound"   's/while i < count {/while i < count - 1 {/'                                phase_b_tree "${F[@]}"
run_mutant "mode1_root_sentinel"    's/(\*current_node).parent_id } != -1/(*current_node).parent_id } != -2/'    phase_b_tree "${F[@]}"

restore
trap - EXIT

echo
if [[ $fail -eq 0 ]]; then
  printf '\033[32mALL MUTANTS CAUGHT — the differential harness has teeth\033[0m\n'
else
  printf '\033[31mSOME MUTANTS SURVIVED — harness has blind spots\033[0m\n'
fi
# Confirm the source really is back to normal.
cargo build --release --lib -q && echo "src/lib.rs restored and building"
exit $fail

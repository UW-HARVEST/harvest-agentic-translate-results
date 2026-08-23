#!/usr/bin/env python3
"""Row-id -> test mapping for the test files authored by the lead agent
(t01..t06), injected into each file's `//!` module doc comment so that
`.phaseA/coverage.py` can pick it up.

Each entry is (test_file, test_fn, "CONFIGS"|"ERRORS", [row ids]).
"""
import re
import sys


def rng(g, a, b):
    return [f"{g}-{i:03d}" for i in range(a, b + 1)]


# --------------------------------------------------------------------------
# t01_constants.rs — the 374 nullary accessors, which are the "constant
# accessor" rows of every group.
# --------------------------------------------------------------------------
T01 = [
    ("size_t_accessors_match", "CONFIGS",
     ["G3-001", "G3-032", "G3-059", "G3-082", "G3-101", "G3-121", "G3-122"]),
    ("ull_accessors_match", "CONFIGS", ["G3-132"]),
    ("cstr_accessors_match", "CONFIGS", []),
    ("int_accessors_match", "CONFIGS", ["G3-120"]),
    ("uchar_accessors_match", "CONFIGS", []),
    ("sodium_init_is_idempotent", "CONFIGS", []),
    ("randombytes_close_matches", "CONFIGS", []),
    ("randombytes_stir_matches", "CONFIGS", []),
]

# --------------------------------------------------------------------------
# t02_lowlevel.rs — crypto_verify_*, crypto_core_salsa*/hsalsa20/hchacha20,
# every crypto_stream_* variant, crypto_shorthash_*.  (G6 + the G4 verify /
# shorthash rows; the owning agents annotate their own new files.)
# --------------------------------------------------------------------------
T02 = []

# --------------------------------------------------------------------------
# t04_aead.rs — Phase B for the whole crypto_aead group.
# --------------------------------------------------------------------------
T04 = [
    ("aead_combined_message_lengths", "CONFIGS",
     rng("G3", 4, 10) + ["G3-022", "G3-023"] + rng("G3", 35, 40)
     + ["G3-051", "G3-052"] + rng("G3", 61, 64) + ["G3-074", "G3-075"]
     + rng("G3", 84, 88) + ["G3-094", "G3-095"] + rng("G3", 103, 106)
     + ["G3-112", "G3-113"]),
    ("aead_combined_ad_lengths", "CONFIGS",
     rng("G3", 11, 16) + rng("G3", 41, 48) + ["G3-068", "G3-069"]
     + ["G3-090", "G3-091", "G3-109"]),
    ("aead_detached_and_equivalence", "CONFIGS",
     ["G3-019", "G3-020", "G3-021", "G3-028", "G3-050", "G3-072", "G3-073",
      "G3-078", "G3-093", "G3-097", "G3-111", "G3-115", "G3-130"]),
    ("aead_null_out_length_pointers", "CONFIGS",
     ["G3-017", "G3-020", "G3-024", "G3-049", "G3-050", "G3-054", "G3-070",
      "G3-072", "G3-076", "G3-092", "G3-096", "G3-110", "G3-114"]),
    ("aead_null_pointer_valid_shapes", "CONFIGS",
     ["G3-018", "G3-025", "G3-026", "G3-053", "G3-071", "G3-077", "G3-092",
      "G3-096", "G3-110", "G3-114", "G3-133", "G3-136", "G3-137"]),
    ("aead_null_pointer_valid_shapes", "ERRORS", ["G3-075"]),
    ("aead_in_place", "CONFIGS", ["G3-029", "G3-056", "G3-080", "G3-098", "G3-116"]),
    ("aead_key_nonce_shapes", "CONFIGS",
     ["G3-030", "G3-057", "G3-081", "G3-099", "G3-117", "G3-118"]),
    ("aead_chunk_boundary_lengths", "CONFIGS",
     ["G3-065", "G3-066", "G3-067", "G3-079", "G3-089", "G3-097", "G3-107", "G3-115"]),
    ("aegis_detached_short_clen", "CONFIGS", ["G3-027", "G3-055", "G3-135"]),
    ("xchacha_equals_chacha_ietf_with_derived_subkey", "CONFIGS", ["G3-119"]),
    ("aead_keygens", "CONFIGS",
     ["G3-002", "G3-033", "G3-060", "G3-083", "G3-102", "G3-123"]),
    ("aes256gcm_is_unavailable_identically", "CONFIGS",
     ["G3-120", "G3-121", "G3-122", "G3-124", "G3-125", "G3-126", "G3-127"]),
    ("aes256gcm_is_unavailable_identically", "ERRORS", ["G3-073"]),
    ("chacha20poly1305_variants_are_not_interchangeable", "CONFIGS", ["G3-100"]),
    ("aead_cross_primitive_invariants", "CONFIGS",
     ["G3-128", "G3-131", "G3-132", "G3-133", "G3-134"]),
]

# --------------------------------------------------------------------------
# t05_aead_errors.rs — Phase C for the whole crypto_aead group.
# --------------------------------------------------------------------------
T05 = [
    ("decrypt_clen_below_abytes", "ERRORS",
     ["G3-005", "G3-006", "G3-021", "G3-022", "G3-034", "G3-035",
      "G3-043", "G3-044", "G3-054", "G3-055"]),
    ("decrypt_bad_tag_zeroes_plaintext", "ERRORS",
     ["G3-007", "G3-008", "G3-009", "G3-023", "G3-024", "G3-025",
      "G3-036", "G3-037", "G3-038", "G3-045", "G3-046", "G3-048",
      "G3-056", "G3-057", "G3-060"]),
    ("decrypt_ad_mismatch", "ERRORS", ["G3-047"]),
    ("xchacha_wrong_nonce_halves", "ERRORS", ["G3-058", "G3-059"]),
    ("decrypt_detached_bad_mac", "ERRORS",
     ["G3-013", "G3-014", "G3-029", "G3-030", "G3-039", "G3-040",
      "G3-049", "G3-050", "G3-061", "G3-062"]),
    ("aegis_decrypt_oversized_lengths_return_minus_one", "ERRORS",
     ["G3-010", "G3-011", "G3-012", "G3-026", "G3-027", "G3-028"]),
    ("aes256gcm_sets_enosys", "ERRORS", rng("G3", 64, 72)),
    ("aes256gcm_sets_enosys", "CONFIGS", ["G3-124", "G3-125", "G3-126", "G3-127"]),
    ("misuse_paths_match", "ERRORS",
     ["G3-001", "G3-002", "G3-003", "G3-004", "G3-017", "G3-018", "G3-019",
      "G3-020", "G3-033", "G3-042", "G3-053", "G3-078"]),
    ("misuse_without_handler_aborts_identically", "ERRORS", ["G3-001"]),
    ("misuse_without_handler_aborts_identically", "CONFIGS", ["G3-129"]),
    # rows the C makes unreachable / non-constructible, documented + asserted
    ("documented_unreachable_error_rows", "ERRORS",
     ["G3-015", "G3-016", "G3-031", "G3-032", "G3-041", "G3-051", "G3-052",
      "G3-063", "G3-074", "G3-076", "G3-077", "G3-079", "G3-080"]),
    ("documented_unreachable_error_rows", "CONFIGS", ["G3-031", "G3-058", "G3-108"]),
]

# --------------------------------------------------------------------------
# t06_internal_exports.rs — the 121 `_`-prefixed internal exports.
# --------------------------------------------------------------------------
T06 = [
    ("pick_best_implementation_selectors", "CONFIGS", ["G3-003", "G3-034"]),
    ("exported_implementation_vtables_are_functionally_equal", "CONFIGS",
     ["G3-031", "G3-058"]),
]

FILES = {
    "tests/t01_constants.rs": T01,
    "tests/t02_lowlevel.rs": T02,
    "tests/t04_aead.rs": T04,
    "tests/t05_aead_errors.rs": T05,
    "tests/t06_internal_exports.rs": T06,
}

BEGIN = "//! ### Row coverage (generated by `.phaseA/rowmap_mine.py`)"


def render(entries):
    if not entries:
        return None
    lines = [BEGIN, "//!", "//! | test | table | rows |", "//! |---|---|---|"]
    for fn, table, ids in entries:
        if not ids:
            continue
        lines.append(f"//! | `{fn}` | {table} | {', '.join(sorted(set(ids)))} |")
    return "\n".join(lines) + "\n"


def main():
    for path, entries in FILES.items():
        block = render(entries)
        src = open(path).read()
        # drop any previously injected block
        if BEGIN in src:
            i = src.index(BEGIN)
            j = src.index("\n\n", i)
            src = src[:i].rstrip("\n") + "\n" + src[j:].lstrip("\n")
        if block is None:
            open(path, "w").write(src)
            print(f"{path}: no rows")
            continue
        # insert right after the last leading `//!` line
        lines = src.splitlines(keepends=True)
        k = 0
        while k < len(lines) and (lines[k].lstrip().startswith("//!") or lines[k].strip() == ""):
            if lines[k].lstrip().startswith("//!"):
                k += 1
            elif k + 1 < len(lines) and lines[k + 1].lstrip().startswith("//!"):
                k += 1
            else:
                break
        out = "".join(lines[:k]) + "//!\n" + block + "\n" + "".join(lines[k:]).lstrip("\n")
        open(path, "w").write(out)
        n = sum(len(set(i)) for _, _, i in entries)
        print(f"{path}: injected {n} row references")
    return 0


if __name__ == "__main__":
    sys.exit(main())

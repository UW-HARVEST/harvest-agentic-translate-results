#!/usr/bin/env python3
"""Sensitivity check for the differential test suite.

Injects deliberate bugs into ``src/lib.rs`` one at a time (literal string
replacement, no regex) and asserts the suite CATCHES each one.  A suite that
passes a mutated Rust library proves nothing, so this is what justifies
trusting the green run.
"""
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(HERE, "src", "lib.rs")

# Mutants that are PROVABLY behaviourally equivalent to the original on LP64,
# so no test can ever catch them. They are still run, but a "MISSED" verdict is
# expected and not a failure.
EQUIVALENT = {
    # `errno == 0` can never be the SOLE reason parse_val rejects: glibc's
    # strtol sets ERANGE only on overflow, in which case it returns LONG_MAX or
    # LONG_MIN, and both already fail the `tmp >= INT_MIN && tmp <= INT_MAX`
    # check. Verified exhaustively over all digit-run lengths 1..500 and 3e6
    # random byte strings: zero inputs where errno != 0 and tmp is in int range.
    "drop the errno==0 check",
}

# (name, needle, replacement)
MUTANTS = [
    ("boundary INT_MIN off-by-one",
     "tmp >= INT_MIN_L", "tmp > INT_MIN_L"),
    ("boundary INT_MAX off-by-one",
     "tmp <= INT_MAX_L", "tmp < INT_MAX_L"),
    ("drop the errno==0 check",
     "&& errno_get() == EOK", "&& true"),
    ("drop the errno reset",
     "errno_set(EOK);", "();"),
    ("endp comparison inverted",
     "if endp != (str_ as *mut c_char)", "if endp == (str_ as *mut c_char)"),
    ("float precision %.1f -> %.2f",
     "%.1f bathrooms", "%.2f bathrooms"),
    ("bathrooms increment 1.0 -> 0.5",
     "(*h).bathrooms += 1.0;", "(*h).bathrooms += 0.5;"),
    ("floors increment dropped",
     "(*house).floors.wrapping_add(1)", "(*house).floors.wrapping_add(0)"),
    ("bedrooms sign flipped",
     "(*house).bedrooms.wrapping_add(extra_bedrooms)",
     "(*house).bedrooms.wrapping_sub(extra_bedrooms)"),
    ("run called once instead of twice",
     "        run(x);\n        run(x);", "        run(x);"),
    ("error message text",
     "An error occurred", "An error happened "),
    ("print order: floor added before first print",
     "    print_the_house();\n    add_floor_to_the_house();\n    print_the_house();",
     "    add_floor_to_the_house();\n    print_the_house();\n    print_the_house();"),
    ("strtol base 10 -> 16",
     "strtol(str_, &mut endp, 10)", "strtol(str_, &mut endp, 16)"),
    ("int truncation via i16",
     "*val = tmp as c_int;", "*val = tmp as i16 as c_int;"),
    ("bathrooms printed at f32 precision",
     "(*h).bathrooms,", "(*h).bathrooms as f32 as c_double,"),
    ("initial floors 2 -> 3",
     "floors: 2,", "floors: 3,"),
    ("initial bedrooms 5 -> 4",
     "bedrooms: 5,", "bedrooms: 4,"),
    ("initial bathrooms 2.5 -> 2.0",
     "bathrooms: 2.5,", "bathrooms: 2.0,"),
    ("bathrooms bumped before add_floor (state order)",
     "    add_floor_to_the_house();", "    (*addr_of_mut!(THE_HOUSE)).bathrooms += 1.0;\n    add_floor_to_the_house();"),
    ("swap floors/bedrooms in printf",
     "(*h).floors,\n        (*h).bedrooms,", "(*h).bedrooms,\n        (*h).floors,"),
    ("driver: no output on the error path",
     "printf(MSG.as_ptr() as *const c_char);", "();"),
    ("parse_val always succeeds",
     "    } else {\n        false\n    }\n}", "    } else {\n        true\n    }\n}"),
]

# Mutants needing edits at several sites. Each is (name, [(needle, repl, count)])
# where count=0 means "replace every occurrence".
MULTI_MUTANTS = [
    # `static house_t the_house` translated as per-thread state instead of
    # process-global state. Invisible on a single thread; only
    # `global_state_is_process_global_not_thread_local` can see it.
    ("global state made thread-local", [
        ("""static mut THE_HOUSE: house_t = house_t {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};""",
         """thread_local! {
    static TL_HOUSE: core::cell::UnsafeCell<house_t> =
        core::cell::UnsafeCell::new(house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        });
}
unsafe fn the_house_ptr() -> *mut house_t {
    TL_HOUSE.with(|c| c.get())
}""", 1),
        ("addr_of_mut!(THE_HOUSE)", "the_house_ptr()", 0),
    ]),

    # Formatting reimplemented in Rust instead of delegating to libc printf.
    # Byte-identical in the "C" locale; only the LC_NUMERIC test can see it.
    ("printf formatting reimplemented in Rust", [
        ("""    printf(
        FMT.as_ptr() as *const c_char,
        (*h).floors,
        (*h).bedrooms,
        (*h).bathrooms,
    );""",
         """    let _ = FMT;
    let s = format!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\\n\\0",
        (*h).floors,
        (*h).bedrooms,
        (*h).bathrooms
    );
    let pct = b"%s\\0";
    printf(pct.as_ptr() as *const c_char, s.as_ptr() as *const c_char);""", 1),
    ]),

    # A translation that writes through the `const char *` input. Only the
    # read-only-page test can see it.
    ("parse_val writes through the const input pointer", [
        ("    let mut endp: *mut c_char = str_ as *mut c_char;",
         "    let mut endp: *mut c_char = str_ as *mut c_char;\n"
         "    *(str_ as *mut c_char) = *str_;", 1),
    ]),
]


def sh(cmd):
    return subprocess.run(cmd, cwd=HERE, shell=True,
                          stdout=subprocess.DEVNULL,
                          stderr=subprocess.DEVNULL).returncode


def main():
    with open(LIB, "r", encoding="utf-8") as f:
        orig = f.read()

    def restore():
        with open(LIB, "w", encoding="utf-8") as f:
            f.write(orig)

    # Normalise both tables into (name, [(needle, repl, count), ...]).
    all_mutants = [(n, [(a, b, 1)]) for n, a, b in MUTANTS] + MULTI_MUTANTS

    caught, missed, skipped, equivalent = [], [], [], []
    try:
        for name, edits in all_mutants:
            mutated = orig
            absent = [needle for needle, _, _ in edits if needle not in mutated]
            if absent:
                skipped.append((name, "needle not found"))
                print(f"SKIP (needle absent): {name}", flush=True)
                continue
            for needle, repl, count in edits:
                mutated = (mutated.replace(needle, repl)
                           if count == 0 else
                           mutated.replace(needle, repl, count))
            if mutated == orig:
                skipped.append((name, "mutation was a no-op"))
                print(f"SKIP (no-op): {name}", flush=True)
                continue
            with open(LIB, "w", encoding="utf-8") as f:
                f.write(mutated)
            if sh("cargo build --offline -q") != 0:
                skipped.append((name, "does not compile"))
                print(f"SKIP (mutant does not compile): {name}", flush=True)
                continue
            survived = sh("cargo test --offline -q") == 0
            if name in EQUIVALENT:
                equivalent.append(name)
                verdict = "as expected" if survived else "UNEXPECTEDLY CAUGHT"
                print(f"equivalent mutant survived ({verdict}): {name}",
                      flush=True)
            elif survived:
                missed.append(name)
                print(f"*** MISSED: {name}   <-- suite not sensitive enough",
                      flush=True)
            else:
                caught.append(name)
                print(f"caught: {name}", flush=True)
    finally:
        restore()
        sh("cargo build --offline -q")

    print()
    print(f"mutants run: {len(caught) + len(missed) + len(equivalent)}   "
          f"caught: {len(caught)}   MISSED: {len(missed)}   "
          f"provably-equivalent: {len(equivalent)}   "
          f"skipped: {len(skipped)}")
    for n, why in skipped:
        print(f"  skipped {n}: {why}")
    return 1 if missed else 0


if __name__ == "__main__":
    sys.exit(main())

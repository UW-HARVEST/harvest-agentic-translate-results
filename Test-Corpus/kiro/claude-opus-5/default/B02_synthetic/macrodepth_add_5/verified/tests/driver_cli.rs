//! Phase B — rows 28..30 of `CONFIGS.md`: the composed `main` from `mdmain.c`,
//! compared as a whole process (stdout + stderr + exit status, byte for byte).
//!
//! This is what pins the `printf` format strings of every `mdcore.c` helper as
//! well as `main`'s own two lines, since `main` calls all of them.

mod common;

use common::*;

/// Row 28 — well-formed operands, including the `int` boundaries and 64 seeded
/// random pairs.
#[test]
fn r28_driver_valid_operands() {
    let fixed: &[(&str, &str)] = &[
        ("0", "0"),
        ("7", "3"),
        ("-5", "9"),
        ("1", "-1"),
        ("2147483647", "1"),
        ("-2147483648", "-1"),
        ("2147483647", "2147483647"),
        ("-2147483648", "-2147483648"),
        ("65536", "65536"),
        ("46341", "46341"),
    ];
    for (a, b) in fixed {
        diff_driver("./driver", &[a, b]);
    }
    for (a, b) in random_pairs(SEED ^ 0x88, 64) {
        diff_driver("./driver", &[&a.to_string(), &b.to_string()]);
    }
}

/// Row 29 — operand *string* shapes, i.e. `atoi` behaviour.
#[test]
fn r29_driver_operand_string_shapes() {
    let shapes: &[&str] = &[
        "12",
        "  12",
        "\t12",
        "\n12",
        "+12",
        "-12",
        "  -12  ",
        "12abc",
        "12 34",
        "abc",
        "",
        " ",
        "0x10",
        "007",
        "-0",
        "+0",
        "--5",
        "++5",
        "- 5",
        ".5",
        "1e3",
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967296",
        "99999999999999999999",
        "-99999999999999999999",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
    ];
    for a in shapes {
        for b in ["3", "-3", "0"] {
            diff_driver("./driver", &[a, b]);
        }
    }
    // ...and in the second position too.
    for b in shapes {
        diff_driver("./driver", &["5", b]);
    }
}

/// Row 30 — extra operands are ignored (`argc > 3` is not rejected), and the
/// `op=<name>` line carries `STR(OP)` in both implementations.
#[test]
fn r30_driver_extra_args_and_op_name() {
    diff_driver("./driver", &["7", "3", "extra"]);
    diff_driver("./driver", &["7", "3", "extra", "more", "-1"]);
    diff_driver("/absolute/looking/path/driver", &["7", "3"]);
    diff_driver("d", &["7", "3"]);

    // Both must print `op=<OP>` on the summary line.
    let out = std::process::Command::new(c_driver_path()).args(["7", "3"]).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        text.contains(&format!("op={OP} ")),
        "[{}] C driver stdout lacks `op={OP}`: {text:?}",
        config_label()
    );
}

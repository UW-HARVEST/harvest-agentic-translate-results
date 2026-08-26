// Phase B -- CONFIGS.md rows 17..25: the mid-level entry point
// `void driver(int *out, int len)`, driven through both shared objects.
//
// Both observable effects are compared: the bytes it prints and the in-place
// transformation of the caller's buffer (plus the process exit status).
//
// The calls happen in a child process (`examples/so_runner.rs`, mode `driver` /
// `driver2`) because fd 1 is process-global and libtest writes its own progress
// to it from other threads.  `tests/in_process_stdout.rs` repeats a slice of
// this coverage with the library called *in-process* through libloading.

mod common;

use common::{assert_driver_matches_out_of_process as check, run_driver_so, Rng, EXTREMES};
use std::os::raw::c_int;

const ITERS: usize = 80;

fn rand_vals(rng: &mut Rng, n: usize) -> Vec<i32> {
    (0..n).map(|_| rng.next_i32()).collect()
}

/// Formats a buffer the way `so_runner` dumps it on stderr.
fn dumped(values: &[i32]) -> String {
    values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn row17_len_zero_prints_nothing() {
    let mut rng = Rng::new(0x1111);
    for i in 0..20 {
        let n = rng.range_incl(1, 32) as usize;
        let vals = rand_vals(&mut rng, n);
        check(&format!("row17#{i}"), "driver", &vals, 0);

        // ... and the C behaviour ERRORS.md/CONFIGS.md claims really holds.
        let c = run_driver_so(&common::c_so(), "driver", &vals, 0);
        assert!(
            c.stdout.is_empty(),
            "C driver printed {:?} for len=0",
            String::from_utf8_lossy(&c.stdout)
        );
        assert_eq!(
            String::from_utf8_lossy(&c.stderr),
            dumped(&vals),
            "C driver mutated the buffer for len=0"
        );
    }
}

#[test]
fn row18_negative_len_prints_nothing() {
    let mut rng = Rng::new(0x1212);
    let lens: [c_int; 5] = [-1, -2, -100, c_int::MIN + 1, c_int::MIN];
    for (i, &len) in lens.iter().enumerate() {
        for j in 0..6 {
            let n = rng.range_incl(1, 32) as usize;
            let vals = rand_vals(&mut rng, n);
            check(&format!("row18#{i}.{j}(len={len})"), "driver", &vals, len);

            let c = run_driver_so(&common::c_so(), "driver", &vals, len);
            assert!(
                c.stdout.is_empty(),
                "C driver printed something for len={len}"
            );
            assert_eq!(
                String::from_utf8_lossy(&c.stderr),
                dumped(&vals),
                "C driver mutated the buffer for len={len}"
            );
        }
    }
}

#[test]
fn row19_len_one_random() {
    let mut rng = Rng::new(0x1313);
    for i in 0..ITERS {
        let vals = rand_vals(&mut rng, 1);
        check(&format!("row19#{i}"), "driver", &vals, 1);
    }
}

#[test]
fn row20_len_2_to_8_random() {
    let mut rng = Rng::new(0x1414);
    for i in 0..ITERS {
        let len = rng.range_incl(2, 8) as c_int;
        let vals = rand_vals(&mut rng, len as usize);
        check(&format!("row20#{i}"), "driver", &vals, len);
    }
}

#[test]
fn row21_len_100_random() {
    let mut rng = Rng::new(0x1515);
    for i in 0..ITERS {
        let vals = rand_vals(&mut rng, 100);
        check(&format!("row21#{i}"), "driver", &vals, 100);
    }
}

#[test]
fn row22_len_1000_random() {
    let mut rng = Rng::new(0x1616);
    for i in 0..20 {
        let vals = rand_vals(&mut rng, 1000);
        check(&format!("row22#{i}"), "driver", &vals, 1000);
    }
}

#[test]
fn row23_extreme_values() {
    let mut rng = Rng::new(0x1717);
    // Every extreme value on its own first (printf("%d") of INT_MIN etc).
    for (i, &v) in EXTREMES.iter().enumerate() {
        check(&format!("row23-single#{i}"), "driver", &[v], 1);
    }
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let vals: Vec<i32> = (0..len).map(|_| *rng.pick(EXTREMES)).collect();
        check(&format!("row23#{i}"), "driver", &vals, len);
    }
}

#[test]
fn row24_small_values_no_overflow() {
    let mut rng = Rng::new(0x1818);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let vals: Vec<i32> = (0..len).map(|_| rng.range_incl(-100, 100) as i32).collect();
        check(&format!("row24#{i}"), "driver", &vals, len);
    }
}

#[test]
fn row25_driver_called_twice_on_same_buffer() {
    let mut rng = Rng::new(0x1919);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 32) as c_int;
        let vals: Vec<i32> = (0..len)
            .map(|_| {
                if rng.bool() {
                    rng.next_i32()
                } else {
                    *rng.pick(EXTREMES)
                }
            })
            .collect();
        check(&format!("row25#{i}"), "driver2", &vals, len);
    }
}

// Phase B / Phase C, in-process variant: `harness = false` so this binary is
// strictly single-threaded and nothing else can write to fd 1 while it is
// redirected.  That makes it safe to call `driver` *in-process* through
// `libloading` in both shared objects and diff the bytes each one printed.
//
// Covers CONFIGS.md rows 17..25 and ERRORS.md rows 17..21 again, this time with
// the library loaded into the test process itself (the closest thing to a real
// embedding consumer).

mod common;

use common::{
    assert_driver_matches, assert_driver_twice_matches, capture_stdout, libs, Rng, EXTREMES,
};
use std::os::raw::c_int;

fn rand_vals(rng: &mut Rng, n: usize) -> Vec<i32> {
    (0..n).map(|_| rng.next_i32()).collect()
}

fn banner(name: &str) {
    println!("test {name} ...");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

fn row17_len_zero() {
    banner("in_process row17 driver len=0");
    let mut rng = Rng::new(0xA017);
    for i in 0..20 {
        let n = rng.range_incl(1, 32) as usize;
        let vals = rand_vals(&mut rng, n);
        assert_driver_matches(&format!("ip-row17#{i}"), &vals, 0);

        let l = libs();
        let mut buf = vals.clone();
        let (_, out) = capture_stdout("c0", || {
            let f = l.c_driver();
            unsafe { f(buf.as_mut_ptr(), 0) };
        });
        assert!(out.is_empty(), "C driver printed {out:?} for len=0");
        assert_eq!(buf, vals, "C driver mutated the buffer for len=0");
    }
}

fn row18_negative_len() {
    banner("in_process row18 driver len<0");
    let mut rng = Rng::new(0xA018);
    let lens: [c_int; 5] = [-1, -2, -100, c_int::MIN + 1, c_int::MIN];
    for (i, &len) in lens.iter().enumerate() {
        for j in 0..8 {
            let n = rng.range_incl(1, 32) as usize;
            let vals = rand_vals(&mut rng, n);
            assert_driver_matches(&format!("ip-row18#{i}.{j}(len={len})"), &vals, len);

            let l = libs();
            let mut buf = vals.clone();
            let (_, out) = capture_stdout("cneg", || {
                let f = l.c_driver();
                unsafe { f(buf.as_mut_ptr(), len) };
            });
            assert!(out.is_empty(), "C driver printed {out:?} for len={len}");
            assert_eq!(buf, vals, "C driver mutated the buffer for len={len}");
        }
    }
}

fn row19_len_one() {
    banner("in_process row19 driver len=1");
    let mut rng = Rng::new(0xA019);
    for i in 0..200 {
        let vals = rand_vals(&mut rng, 1);
        assert_driver_matches(&format!("ip-row19#{i}"), &vals, 1);
    }
}

fn row20_small_lens() {
    banner("in_process row20 driver len=2..8");
    let mut rng = Rng::new(0xA020);
    for i in 0..200 {
        let len = rng.range_incl(2, 8) as c_int;
        let vals = rand_vals(&mut rng, len as usize);
        assert_driver_matches(&format!("ip-row20#{i}"), &vals, len);
    }
}

fn row21_len_100() {
    banner("in_process row21 driver len=100");
    let mut rng = Rng::new(0xA021);
    for i in 0..200 {
        let vals = rand_vals(&mut rng, 100);
        assert_driver_matches(&format!("ip-row21#{i}"), &vals, 100);
    }
}

fn row22_len_1000() {
    banner("in_process row22 driver len=1000");
    let mut rng = Rng::new(0xA022);
    for i in 0..20 {
        let vals = rand_vals(&mut rng, 1000);
        assert_driver_matches(&format!("ip-row22#{i}"), &vals, 1000);
    }
}

fn row23_extremes() {
    banner("in_process row23 driver extreme values");
    let mut rng = Rng::new(0xA023);
    for (i, &v) in EXTREMES.iter().enumerate() {
        assert_driver_matches(&format!("ip-row23-single#{i}"), &[v], 1);
    }
    for i in 0..200 {
        let len = rng.range_incl(1, 64) as c_int;
        let vals: Vec<i32> = (0..len).map(|_| *rng.pick(EXTREMES)).collect();
        assert_driver_matches(&format!("ip-row23#{i}"), &vals, len);
    }
}

fn row24_small_values() {
    banner("in_process row24 driver small values");
    let mut rng = Rng::new(0xA024);
    for i in 0..200 {
        let len = rng.range_incl(1, 64) as c_int;
        let vals: Vec<i32> = (0..len).map(|_| rng.range_incl(-100, 100) as i32).collect();
        assert_driver_matches(&format!("ip-row24#{i}"), &vals, len);
    }
}

fn row25_driver_twice() {
    banner("in_process row25 driver called twice");
    let mut rng = Rng::new(0xA025);
    for i in 0..200 {
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
        assert_driver_twice_matches(&format!("ip-row25#{i}"), &vals, len);
    }
}

fn err19_driver_null_len_le_zero() {
    banner("in_process ERRORS row19 driver(NULL, len<=0)");
    let l = libs();
    for len in [0i32, -1, -1000, c_int::MIN] {
        let (_, cout) = capture_stdout("e19c", || {
            let f = l.c_driver();
            unsafe { f(std::ptr::null_mut(), len) };
        });
        let (_, rout) = capture_stdout("e19r", || {
            let f = l.rust_driver();
            unsafe { f(std::ptr::null_mut(), len) };
        });
        assert!(cout.is_empty(), "C driver(NULL, {len}) printed {cout:?}");
        assert_eq!(cout, rout, "driver(NULL, {len}) stdout differs");
    }
}

fn err21_oversized_len() {
    banner("in_process ERRORS row21 oversized len");
    let l = libs();
    let mut rng = Rng::new(0xA021_0021);
    for _ in 0..50 {
        let logical = rng.range_incl(1, 32) as usize;
        let slack = rng.range_incl(1, 32) as usize;
        let total = logical + slack;
        let base: Vec<i32> = (0..total).map(|_| rng.next_i32()).collect();
        let len = total as c_int;

        let mut cbuf = base.clone();
        let (_, cout) = capture_stdout("e21c", || {
            let f = l.c_driver();
            unsafe { f(cbuf.as_mut_ptr(), len) };
        });
        let mut rbuf = base.clone();
        let (_, rout) = capture_stdout("e21r", || {
            let f = l.rust_driver();
            unsafe { f(rbuf.as_mut_ptr(), len) };
        });
        assert_eq!(cbuf, rbuf, "oversized-len driver buffer differs");
        assert_eq!(cout, rout, "oversized-len driver stdout differs");
    }
}

fn main() {
    row17_len_zero();
    row18_negative_len();
    row19_len_one();
    row20_small_lens();
    row21_len_100();
    row22_len_1000();
    row23_extremes();
    row24_small_values();
    row25_driver_twice();
    err19_driver_null_len_le_zero();
    err21_oversized_len();
    println!("in-process differential checks: all passed");
}

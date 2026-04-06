use std::sync::atomic::{AtomicI32, Ordering};

static SUM: AtomicI32 = AtomicI32::new(0);

/// Reset the static state (for testing).
pub fn reset_static_sum() {
    SUM.store(0, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn static_sum(update: i32) -> i32 {
    let new = SUM.load(Ordering::SeqCst).wrapping_add(update);
    SUM.store(new, Ordering::SeqCst);
    new
}

#[no_mangle]
pub unsafe extern "C" fn c_main(argc: i32, argv: *const *const i8) -> i32 {
    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return 1;
    }

    let arg1 = unsafe {
        let ptr = *argv.offset(1);
        std::ffi::CStr::from_ptr(ptr).to_str().unwrap_or("")
    };

    let stride: i32 = match strtol_leading(arg1) {
        Some(v) => v as i32,
        None => {
            println!("Error: first argument must be an integer!");
            return 1;
        }
    };

    reset_static_sum();
    for i in 0..10 {
        println!("{}", static_sum((i as i32).wrapping_mul(stride)));
    }

    0
}

fn strtol_leading(s: &str) -> Option<i64> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    let mut chars = s.chars().peekable();
    let negative = match chars.peek() {
        Some('+') => { chars.next(); false }
        Some('-') => { chars.next(); true }
        _ => false,
    };
    let mut found = false;
    let mut val: i64 = 0;
    while let Some(&c) = chars.peek() {
        if let Some(d) = c.to_digit(10) {
            found = true;
            val = val.wrapping_mul(10).wrapping_add(d as i64);
            chars.next();
        } else {
            break;
        }
    }
    if !found {
        return None;
    }
    Some(if negative { -val } else { val })
}

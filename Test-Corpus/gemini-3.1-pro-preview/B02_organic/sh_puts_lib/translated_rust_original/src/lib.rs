use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn sh_puts(num: c_int) {
    for i in 0..num {
        let _ = format!("test_{}", i);
    }

    let s_key = "a";
    let s_value = num;

    let mut strmap = Vec::new();
    strmap.push((s_key.to_string(), s_value));

    assert_eq!(strmap[0].0.as_bytes()[0], b'a');
    assert_ne!(strmap[0].0.as_ptr(), s_key.as_ptr());
    assert_eq!(strmap[0].1, s_value);

    for item in &strmap {
        println!("{} {}", item.0, item.1);
    }
}

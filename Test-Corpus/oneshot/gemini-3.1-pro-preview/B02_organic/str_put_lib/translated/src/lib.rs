use std::os::raw::c_int;

fn strkey(n: c_int) -> String {
    format!("test_{}", n)
}

#[unsafe(no_mangle)]
pub extern "C" fn str_put(num: c_int) {
    for i in 0..num {
        let _ = strkey(i);
    }

    let mut strmap = Vec::new();
    strmap.push(("a", num));

    assert_eq!(strmap[0].0.as_bytes()[0], b'a');
    assert_eq!(strmap[0].0, "a");
    assert_eq!(strmap[0].1, num);

    for z in 0..strmap.len() {
        println!("{} {}", strmap[z].0, strmap[z].1);
    }
}

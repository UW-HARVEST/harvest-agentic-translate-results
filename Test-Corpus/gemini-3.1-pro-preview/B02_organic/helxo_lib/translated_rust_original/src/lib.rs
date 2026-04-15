use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: c_char) {
    let mut hash: Vec<(&str, c_char)> = Vec::new();

    let mut shput = |key: &'static str, value: c_char| {
        if let Some(entry) = hash.iter_mut().find(|e| e.0 == key) {
            entry.1 = value;
        } else {
            hash.push((key, value));
        }
    };

    shput("bob", b'h' as c_char);
    shput("sally", b'e' as c_char);
    shput("fred", b'l' as c_char);
    shput("jen", b'x' as c_char);
    shput("doug", b'o' as c_char);

    if let Some(entry) = hash.iter_mut().find(|e| e.0 == "jen") {
        entry.1 = letter;
    } else {
        hash.push(("jen", letter));
    }

    for (key, value) in hash {
        println!("{} {}", key, value as u8 as char);
    }
}

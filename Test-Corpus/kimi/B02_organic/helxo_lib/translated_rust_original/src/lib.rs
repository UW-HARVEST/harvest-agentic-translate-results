use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: c_char) {
    let mut hash: HashMap<String, c_char> = HashMap::new();
    
    hash.insert("bob".to_string(), 'h');
    hash.insert("sally".to_string(), 'e');
    hash.insert("fred".to_string(), 'l');
    hash.insert("jen".to_string(), 'x');
    hash.insert("doug".to_string(), 'o');
    
    let name = "jen";
    hash.insert(name.to_string(), letter as c_char);
    
    let keys: Vec<&String> = hash.keys().collect();
    for key in keys {
        if let Some(&value) = hash.get(key) {
            println!("{} {}", key, value as u8 as char);
        }
    }
}
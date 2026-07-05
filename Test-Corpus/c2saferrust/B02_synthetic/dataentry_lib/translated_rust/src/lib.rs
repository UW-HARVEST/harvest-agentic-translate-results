




use std::convert::TryFrom;

extern "C" {
    fn sprintf(
        __s: *mut ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DataEntry {
    pub id: ::core::ffi::c_int,
    pub value: ::core::ffi::c_int,
    pub name: [::core::ffi::c_char; 32],
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NAME_LENGTH: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
static mut lookup_table: [[::core::ffi::c_int; 3]; 4] = [
    [
        10 as ::core::ffi::c_int,
        20 as ::core::ffi::c_int,
        30 as ::core::ffi::c_int,
    ],
    [
        40 as ::core::ffi::c_int,
        50 as ::core::ffi::c_int,
        60 as ::core::ffi::c_int,
    ],
    [
        70 as ::core::ffi::c_int,
        80 as ::core::ffi::c_int,
        90 as ::core::ffi::c_int,
    ],
    [
        100 as ::core::ffi::c_int,
        110 as ::core::ffi::c_int,
        120 as ::core::ffi::c_int,
    ],
];
fn find_entry(
    entries: *mut DataEntry,
    count: ::core::ffi::c_int,
    target_id: ::core::ffi::c_int,
) -> *mut DataEntry {
    let len = usize::try_from(count).ok().unwrap_or(0);
    if len == 0 || entries.is_null() {
        return ::core::ptr::null_mut();
    }

    let entries = unsafe { std::slice::from_raw_parts_mut(entries, len) };
    entries
        .iter_mut()
        .find(|entry| entry.id == target_id)
        .map_or(::core::ptr::null_mut(), |entry| entry as *mut DataEntry)
}

fn process_name(dest: &mut String, src: &str, max_len: ::core::ffi::c_int) -> ::core::ffi::c_int {
    if dest.is_empty() {
        return -1;
    }

    let max_len_usize = usize::try_from(max_len).unwrap_or(0);
    let truncated: String = src.chars().take(max_len_usize).collect();
    dest.clear();
    dest.push_str(&truncated);

    ::core::ffi::c_int::try_from(dest.len()).unwrap_or(::core::ffi::c_int::MAX)
}

fn calculate_lookup(row: i32, col: i32, result: &mut i32) -> i32 {
    let row = match usize::try_from(row) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    let col = match usize::try_from(col) {
        Ok(value) => value,
        Err(_) => return 0,
    };

    let temp = unsafe {
        match lookup_table.get(row).and_then(|r| r.get(col)).copied() {
            Some(value) => value,
            None => return 0,
        }
    };

    if temp != 0 {
        *result = temp * 2;
        1
    } else {
        0
    }
}

fn create_entries(count: ::core::ffi::c_int, base_id: ::core::ffi::c_int) -> *mut DataEntry {
    let count_usize = match usize::try_from(count) {
        Ok(v) if v > 0 => v,
        _ => return ::core::ptr::null_mut(),
    };

    let mut entries = Vec::with_capacity(count_usize);

    for i in 0..count_usize {
        let i_c_int = i as ::core::ffi::c_int;
        let id = base_id + i_c_int;
        let value = id * 10;

        let mut name = [0 as ::core::ffi::c_char; 32];
        let name_string = format!("Entry_{}", id);
        for (idx, byte) in name_string.bytes().take(31).enumerate() {
            name[idx] = byte as ::core::ffi::c_char;
        }

        entries.push(DataEntry { id, value, name });
    }

    let boxed = entries.into_boxed_slice();
    Box::into_raw(boxed) as *mut DataEntry
}

fn modify_entries(entries: *mut DataEntry, count: i32, multiplier: i32) -> i32 {
    if entries.is_null() {
        return -1;
    }

    let len = match usize::try_from(count) {
        Ok(len) => len,
        Err(_) => return 0,
    };

    let entries = unsafe { std::slice::from_raw_parts_mut(entries, len) };

    let mut total = 0;
    for entry in entries.iter_mut() {
        let temp_value = entry.value;
        if temp_value != 0 {
            entry.value = temp_value * multiplier;
            total += entry.value;
        }
    }
    total
}

#[no_mangle]
pub fn dataentry(mode: i32, param1: i32, param2: i32, param3: i32) -> i32 {
    let mut result = 0;
    let mut count = 0;
    let mut lookup_result = 0;
    let mut buffer = String::from("T");

    match mode {
        1 => {
            count = if param1 > 0 { param1 } else { 5 };
            let entries = create_entries(count, 100);
            if entries.is_null() || count == 0 {
                result = -1;
            } else {
                let found = find_entry(entries, count, 100 + param2);
                if found.is_null() || unsafe { (*found).id } == 0 {
                    result = -2;
                } else {
                    result = unsafe { (*found).value };
                    buffer = unsafe {
                        let name_ptr = (*found).name.as_ptr();
                        std::ffi::CStr::from_ptr(name_ptr)
                            .to_string_lossy()
                            .into_owned()
                    };
                }
            }
        }
        2 => {
            count = if param1 > 0 { param1 } else { 3 };
            let entries = create_entries(count, 200);
            if entries.is_null() {
                result = -1;
            } else {
                result = modify_entries(entries, count, param2);
                if result != 0 {
                    result += param3;
                }
            }
        }
        3 => {
            if (0..4).contains(&param1) && (0..3).contains(&param2) {
                result = calculate_lookup(param1, param2, &mut lookup_result);
                if result != 0 {
                    result = lookup_result + param3;
                }
            }
        }
        _ => {
            buffer = "Default".to_string();
            result = process_name(&mut buffer, "TestName", NAME_LENGTH);
            count = i32::try_from(buffer.len()).unwrap_or(i32::MAX);
            if count != 0 {
                result = count * param1;
            }
        }
    }

    result
}


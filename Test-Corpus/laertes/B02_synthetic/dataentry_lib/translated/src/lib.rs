extern "C" {
    fn sprintf(
        __s: *mut libc::c_char,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn strcpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DataEntry {
    pub id: libc::c_int,
    pub value: libc::c_int,
    pub name: [libc::c_char; 32],
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const NAME_LENGTH: libc::c_int = 32 as libc::c_int;
static mut lookup_table: [[libc::c_int; 3]; 4] = [
    [
        10 as libc::c_int,
        20 as libc::c_int,
        30 as libc::c_int,
    ],
    [
        40 as libc::c_int,
        50 as libc::c_int,
        60 as libc::c_int,
    ],
    [
        70 as libc::c_int,
        80 as libc::c_int,
        90 as libc::c_int,
    ],
    [
        100 as libc::c_int,
        110 as libc::c_int,
        120 as libc::c_int,
    ],
];
unsafe extern "C" fn find_entry(
    mut entries: *mut DataEntry,
    mut count: libc::c_int,
    mut target_id: libc::c_int,
) -> *mut DataEntry {
    let mut ptr: *mut DataEntry = entries;
    let mut end: *mut DataEntry = entries.offset(count as isize);
    while ptr < end {
        if (*ptr).id == target_id {
            return ptr;
        }
        ptr = ptr.offset(1);
    }
    return std::ptr::null_mut::<DataEntry>();
}
unsafe extern "C" fn process_name(
    mut dest: *mut libc::c_char,
    mut src: *const libc::c_char,
    mut max_len: libc::c_int,
) -> libc::c_int {
    let mut len: libc::c_int = 0;
    if dest.is_null() || *dest as libc::c_int == '\0' as i32 {
        return -(1 as libc::c_int);
    }
    strcpy(dest, src);
    len = strlen(dest) as libc::c_int;
    return len;
}
unsafe extern "C" fn calculate_lookup(
    mut row: libc::c_int,
    mut col: libc::c_int,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut temp: libc::c_int = 0;
    temp = lookup_table[row as usize][col as usize];
    if temp != 0 {
        *result = temp * 2 as libc::c_int;
        return 1 as libc::c_int;
    }
    return 0 as libc::c_int;
}
unsafe extern "C" fn create_entries(
    mut count: libc::c_int,
    mut base_id: libc::c_int,
) -> *mut DataEntry {
    let mut entries: *mut DataEntry = std::ptr::null_mut::<DataEntry>();
    let mut i: libc::c_int = 0;
    let mut temp_name: [libc::c_char; 32] = [0; 32];
    entries = malloc((count as size_t).wrapping_mul(std::mem::size_of::<DataEntry>() as size_t))
        as *mut DataEntry;
    if entries.is_null() || count <= 0 as libc::c_int {
        return std::ptr::null_mut::<DataEntry>();
    }
    i = 0 as libc::c_int;
    while i < count {
        (*entries.offset(i as isize)).id = base_id + i;
        (*entries.offset(i as isize)).value = (base_id + i) * 10 as libc::c_int;
        sprintf(
            &raw mut temp_name as *mut libc::c_char,
            b"Entry_%d\0" as *const u8 as *const libc::c_char,
            base_id + i,
        );
        strcpy(
            &raw mut (*entries.offset(i as isize)).name as *mut libc::c_char,
            &raw mut temp_name as *mut libc::c_char,
        );
        i += 1;
    }
    return entries;
}
unsafe extern "C" fn modify_entries(
    mut entries: *mut DataEntry,
    mut count: libc::c_int,
    mut multiplier: libc::c_int,
) -> libc::c_int {
    let mut current: *mut DataEntry = std::ptr::null_mut::<DataEntry>();
    let mut last: *mut DataEntry = std::ptr::null_mut::<DataEntry>();
    let mut total: libc::c_int = 0 as libc::c_int;
    let mut temp_value: libc::c_int = 0;
    if entries.is_null() {
        return -(1 as libc::c_int);
    }
    current = entries;
    last = entries.offset(count as isize);
    while current < last {
        temp_value = (*current).value;
        if temp_value != 0 {
            (*current).value = temp_value * multiplier;
            total += (*current).value;
        }
        current = current.offset(1);
    }
    return total;
}
#[no_mangle]
pub unsafe extern "C" fn dataentry(
    mut mode: libc::c_int,
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
) -> libc::c_int {
    let mut entries: *mut DataEntry = std::ptr::null_mut::<DataEntry>();
    let mut found: *mut DataEntry = std::ptr::null_mut::<DataEntry>();
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut count: libc::c_int = 0;
    let mut lookup_result: libc::c_int = 0;
    let mut buffer: [libc::c_char; 32] = [0; 32];
    let mut i: libc::c_int = 0;
    buffer[0 as libc::c_int as usize] = 'T' as i32 as libc::c_char;
    buffer[1 as libc::c_int as usize] = '\0' as i32 as libc::c_char;
    match mode {
        1 => {
            count = if param1 > 0 as libc::c_int {
                param1
            } else {
                5 as libc::c_int
            };
            entries = create_entries(count, 100 as libc::c_int);
            if entries.is_null() || count == 0 as libc::c_int {
                result = -(1 as libc::c_int);
            } else {
                found = find_entry(entries, count, 100 as libc::c_int + param2);
                if found.is_null() || (*found).id == 0 as libc::c_int {
                    result = -(2 as libc::c_int);
                } else {
                    result = (*found).value;
                    strcpy(
                        &raw mut buffer as *mut libc::c_char,
                        &raw mut (*found).name as *mut libc::c_char,
                    );
                }
                free(entries as *mut libc::c_void);
            }
        }
        2 => {
            count = if param1 > 0 as libc::c_int {
                param1
            } else {
                3 as libc::c_int
            };
            entries = create_entries(count, 200 as libc::c_int);
            if entries.is_null() {
                result = -(1 as libc::c_int);
            } else {
                result = modify_entries(entries, count, param2);
                if result != 0 {
                    result += param3;
                }
                free(entries as *mut libc::c_void);
            }
        }
        3 => {
            if param1 >= 0 as libc::c_int
                && param1 < 4 as libc::c_int
                && param2 >= 0 as libc::c_int
                && param2 < 3 as libc::c_int
            {
                result = calculate_lookup(param1, param2, &raw mut lookup_result);
                if result != 0 {
                    result = lookup_result + param3;
                }
            }
        }
        _ => {
            strcpy(
                &raw mut buffer as *mut libc::c_char,
                b"Default\0" as *const u8 as *const libc::c_char,
            );
            result = process_name(
                &raw mut buffer as *mut libc::c_char,
                b"TestName\0" as *const u8 as *const libc::c_char,
                NAME_LENGTH,
            );
            count = strlen(&raw mut buffer as *mut libc::c_char) as libc::c_int;
            if count != 0 {
                result = count * param1;
            }
        }
    }
    return result;
}

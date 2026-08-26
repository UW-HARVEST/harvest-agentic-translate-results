use std::alloc::{Layout, alloc, dealloc};
use std::ffi::c_int;
use std::ptr;

const NAME_LENGTH: usize = 32;

#[repr(C)]
struct DataEntry {
    id: c_int,
    value: c_int,
    name: [u8; NAME_LENGTH],
}

static LOOKUP_TABLE: [[c_int; 3]; 4] = [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]];

unsafe fn find_entry(entries: *mut DataEntry, count: c_int, target_id: c_int) -> *mut DataEntry {
    for index in 0..count {
        let entry = unsafe { entries.add(index as usize) };
        if unsafe { (*entry).id } == target_id {
            return entry;
        }
    }

    ptr::null_mut()
}

fn entry_name(id: c_int) -> [u8; NAME_LENGTH] {
    let mut name = [0; NAME_LENGTH];
    name[..6].copy_from_slice(b"Entry_");

    let mut digits = [0_u8; 10];
    let mut value = id.unsigned_abs();
    let mut digit_count = 0;

    loop {
        digits[digit_count] = b'0' + (value % 10) as u8;
        digit_count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }

    let mut position = 6;
    if id < 0 {
        name[position] = b'-';
        position += 1;
    }
    for digit in digits[..digit_count].iter().rev() {
        name[position] = *digit;
        position += 1;
    }

    name
}

unsafe fn create_entries(count: c_int, base_id: c_int) -> *mut DataEntry {
    let layout = match Layout::array::<DataEntry>(count.max(0) as usize) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };
    let entries = unsafe { alloc(layout).cast::<DataEntry>() };

    if entries.is_null() || count <= 0 {
        return ptr::null_mut();
    }

    for index in 0..count {
        let id = base_id.wrapping_add(index);
        let entry = DataEntry {
            id,
            value: id.wrapping_mul(10),
            name: entry_name(id),
        };
        unsafe { entries.add(index as usize).write(entry) };
    }

    entries
}

unsafe fn free_entries(entries: *mut DataEntry, count: c_int) {
    let layout = Layout::array::<DataEntry>(count as usize).expect("valid allocation layout");
    unsafe { dealloc(entries.cast::<u8>(), layout) };
}

unsafe fn modify_entries(entries: *mut DataEntry, count: c_int, multiplier: c_int) -> c_int {
    if entries.is_null() {
        return -1;
    }

    let mut total: c_int = 0;
    for index in 0..count {
        let entry = unsafe { entries.add(index as usize) };
        let temp_value = unsafe { (*entry).value };
        if temp_value != 0 {
            let value = temp_value.wrapping_mul(multiplier);
            unsafe { (*entry).value = value };
            total = total.wrapping_add(value);
        }
    }

    total
}

fn calculate_lookup(row: c_int, col: c_int, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row as usize][col as usize];
    if temp != 0 {
        *result = temp.wrapping_mul(2);
        return 1;
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    match mode {
        1 => {
            let count = if param1 > 0 { param1 } else { 5 };
            let entries = unsafe { create_entries(count, 100) };

            if entries.is_null() || count == 0 {
                return -1;
            }

            let found = unsafe { find_entry(entries, count, 100_i32.wrapping_add(param2)) };
            let result = if found.is_null() || unsafe { (*found).id } == 0 {
                -2
            } else {
                unsafe { (*found).value }
            };

            unsafe { free_entries(entries, count) };
            result
        }
        2 => {
            let count = if param1 > 0 { param1 } else { 3 };
            let entries = unsafe { create_entries(count, 200) };

            if entries.is_null() {
                return -1;
            }

            let mut result = unsafe { modify_entries(entries, count, param2) };
            if result != 0 {
                result = result.wrapping_add(param3);
            }

            unsafe { free_entries(entries, count) };
            result
        }
        3 => {
            let mut result = 0;
            if (0..4).contains(&param1) && (0..3).contains(&param2) {
                let mut lookup_result = 0;
                result = calculate_lookup(param1, param2, &mut lookup_result);
                if result != 0 {
                    result = lookup_result.wrapping_add(param3);
                }
            }
            result
        }
        _ => 8_i32.wrapping_mul(param1),
    }
}

#[cfg(test)]
mod tests {
    use super::dataentry;

    #[test]
    fn representative_modes() {
        assert_eq!(dataentry(1, 5, 2, 0), 1020);
        assert_eq!(dataentry(1, 5, 8, 0), -2);
        assert_eq!(dataentry(2, 3, 2, 7), 12_067);
        assert_eq!(dataentry(2, 3, 0, 7), 0);
        assert_eq!(dataentry(3, 2, 1, 5), 165);
        assert_eq!(dataentry(3, 4, 1, 5), 0);
        assert_eq!(dataentry(0, 9, 0, 0), 72);
    }
}

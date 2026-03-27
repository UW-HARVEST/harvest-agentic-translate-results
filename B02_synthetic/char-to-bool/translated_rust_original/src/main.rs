use std::io::{self, BufRead};

fn parse_bool(c: u8) -> bool {
    c == b'y' || c == b'Y'
}

fn apply_permissions(read: bool, write: bool, execute: bool) -> i32 {
    let mut permission_value: i32 = 0;
    if read {
        permission_value += 4;
    }
    if write {
        permission_value += 2;
    }
    if execute {
        permission_value += 1;
    }

    if read && write && execute {
        100 + permission_value
    } else if read && write {
        if permission_value == 6 {
            50 + permission_value
        } else {
            0
        }
    } else if read && execute {
        30 + permission_value
    } else if write && execute {
        20 + permission_value
    } else if read {
        10 + permission_value
    } else if write {
        -10
    } else if execute {
        -20
    } else {
        0
    }
}

fn evaluate_conditions(cond1: bool, cond2: bool, cond3: bool, logic_op: i32) -> i32 {
    match logic_op {
        0 => {
            let result = cond1 && cond2 && cond3;
            if result {
                100
            } else {
                if cond1 && cond2 { return 50; }
                if cond1 && cond3 { return 51; }
                if cond2 && cond3 { return 52; }
                if cond1 { return 10; }
                if cond2 { return 11; }
                if cond3 { return 12; }
                0
            }
        }
        1 => {
            let result = cond1 || cond2 || cond3;
            if result {
                let mut count = 0;
                if cond1 { count += 1; }
                if cond2 { count += 1; }
                if cond3 { count += 1; }
                100 + count
            } else {
                0
            }
        }
        2 => {
            let result = cond1 ^ cond2 ^ cond3;
            if result {
                if cond1 && !cond2 && !cond3 { return 1; }
                if !cond1 && cond2 && !cond3 { return 2; }
                if !cond1 && !cond2 && cond3 { return 3; }
                if cond1 && cond2 && cond3 { return 7; }
                90
            } else {
                0
            }
        }
        3 => {
            let result = !(cond1 && cond2 && cond3);
            if result {
                if !cond1 && !cond2 && !cond3 { return 200; }
                if !cond1 { return 150; }
                if !cond2 { return 151; }
                if !cond3 { return 152; }
                100
            } else {
                0
            }
        }
        _ => -1,
    }
}

fn configure_flags(decisions: &[bool]) -> i32 {
    let count = decisions.len();
    let mut special_count: i32 = 0;

    for i in 0..count {
        if decisions[i] {
            special_count += 1;
        }
    }

    if special_count == 0 {
        return 0;
    } else if special_count == count as i32 {
        return 1000 + count as i32;
    } else if special_count == 1 {
        for i in 0..count {
            if decisions[i] {
                return 100 + i as i32;
            }
        }
    } else if special_count == count as i32 - 1 {
        for i in 0..count {
            if !decisions[i] {
                return 200 + i as i32;
            }
        }
    }

    // Check alternating pattern
    let mut alternating = true;
    for i in 1..count {
        if decisions[i] == decisions[i - 1] {
            alternating = false;
            break;
        }
    }
    if alternating {
        return 500 + special_count;
    }

    // Check consecutive true values
    let mut max_consecutive: i32 = 0;
    let mut current_consecutive: i32 = 0;
    for i in 0..count {
        if decisions[i] {
            current_consecutive += 1;
            if current_consecutive > max_consecutive {
                max_consecutive = current_consecutive;
            }
        } else {
            current_consecutive = 0;
        }
    }
    if max_consecutive >= 3 {
        return 300 + max_consecutive;
    }

    special_count
}

/// Reproduces the C behavior where `validate_sequence` casts `char*` to `bool*`
/// and writes bool values (0 or 1) in-place over the original char bytes.
/// In C, `bool` is typically 1 byte, so each char is overwritten with 0x00 or 0x01.
/// Subsequent reads of `bools[i]` then read those byte values.
fn validate_sequence(buf: &mut [u8], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }

    // Convert in-place: write 0x00 or 0x01 over each byte (mimics bool* cast)
    for i in 0..len {
        let val = parse_bool(buf[i]);
        buf[i] = val as u8; // 0 or 1
    }

    // Rule 1: Must start with true (0x01)
    if buf[0] == 0 {
        return -10;
    }

    // Rule 2: Must end with false (0x00) if length > 1
    if len > 1 && buf[len - 1] != 0 {
        return -11;
    }

    // Rule 3: Cannot have more than 3 consecutive same values
    let mut consecutive: i32 = 1;
    for i in 1..len {
        if buf[i] == buf[i - 1] {
            consecutive += 1;
            if consecutive > 3 {
                return -12;
            }
        } else {
            consecutive = 1;
        }
    }

    // Rule 4: Count transitions
    let mut transitions: i32 = 0;
    for i in 1..len {
        if buf[i] != buf[i - 1] {
            transitions += 1;
        }
    }

    let len_i = len as i32;
    if len_i <= 3 {
        if transitions == 0 {
            return 1;
        }
        if transitions == len_i - 1 {
            return 2;
        }
        return 10 + transitions;
    } else if len_i <= 10 {
        if transitions < len_i / 3 {
            return 20;
        }
        if transitions > len_i / 2 {
            return 30;
        }
        return 25;
    } else {
        if transitions < 3 {
            return 40;
        }
        if transitions > len_i - 3 {
            return 50;
        }
        return 45;
    }
}

fn process_decisions(buf: &mut [u8], length: usize, operation: i32, param: i32) -> i32 {
    if length == 0 {
        return -1;
    }

    match operation {
        0 => {
            if length < 3 { return -2; }
            let read = parse_bool(buf[0]);
            let write = parse_bool(buf[1]);
            let execute = parse_bool(buf[2]);
            apply_permissions(read, write, execute)
        }
        1 => {
            if length < 3 { return -2; }
            let cond1 = parse_bool(buf[0]);
            let cond2 = parse_bool(buf[1]);
            let cond3 = parse_bool(buf[2]);
            evaluate_conditions(cond1, cond2, cond3, param)
        }
        2 => {
            let count = if length < 32 { length } else { 32 };
            let mut decisions = [false; 32];
            for i in 0..count {
                decisions[i] = parse_bool(buf[i]);
            }
            configure_flags(&decisions[..count])
        }
        3 => {
            validate_sequence(buf, length)
        }
        _ => -3,
    }
}

fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    // Read operation
    let line = match lines.next() {
        Some(Ok(l)) => l,
        _ => {
            eprint!("Error reading operation\n");
            std::process::exit(1);
        }
    };
    let operation: i32 = line.trim().parse().unwrap_or(0);

    // Read parameter
    let line = match lines.next() {
        Some(Ok(l)) => l,
        _ => {
            eprint!("Error reading parameter\n");
            std::process::exit(1);
        }
    };
    let param: i32 = line.trim().parse().unwrap_or(0);

    // Read decision string
    let line = match lines.next() {
        Some(Ok(l)) => l,
        _ => {
            eprint!("Error reading decision string\n");
            std::process::exit(1);
        }
    };

    let mut buf: Vec<u8> = line.into_bytes();
    let len = buf.len();

    let result = process_decisions(&mut buf, len, operation, param);
    println!("{}", result);
}

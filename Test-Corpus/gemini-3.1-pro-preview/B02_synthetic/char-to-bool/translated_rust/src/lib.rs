pub fn process_decisions(decision_string: &[u8], operation: i32, param: i32) -> i32 {
    let length = decision_string.len();
    if length == 0 {
        return -1;
    }

    match operation {
        0 => {
            if length < 3 {
                return -2;
            }
            let read = parse_bool(decision_string[0]);
            let write = parse_bool(decision_string[1]);
            let execute = parse_bool(decision_string[2]);
            apply_permissions(read, write, execute)
        }
        1 => {
            if length < 3 {
                return -2;
            }
            let cond1 = parse_bool(decision_string[0]);
            let cond2 = parse_bool(decision_string[1]);
            let cond3 = parse_bool(decision_string[2]);
            evaluate_conditions(cond1, cond2, cond3, param)
        }
        2 => {
            let count = if length < 32 { length } else { 32 };
            let mut decisions = [false; 32];
            for i in 0..count {
                decisions[i] = parse_bool(decision_string[i]);
            }
            configure_flags(&decisions[..count])
        }
        3 => validate_sequence(decision_string),
        _ => -3,
    }
}

fn parse_bool(c: u8) -> bool {
    if c == b'y' || c == b'Y' {
        true
    } else if c == b'n' || c == b'N' {
        false
    } else {
        false
    }
}

fn apply_permissions(read: bool, write: bool, execute: bool) -> i32 {
    let mut permission_value = 0;
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
    let mut _flags: u32 = 0;
    let mut special_count = 0;

    for i in 0..count {
        if i < 32 && decisions[i] {
            _flags |= 1u32 << i;
            special_count += 1;
        }
    }

    if special_count == 0 {
        return 0;
    } else if special_count == count {
        return 1000 + count as i32;
    } else if special_count == 1 {
        for i in 0..count {
            if decisions[i] {
                return 100 + i as i32;
            }
        }
    } else if special_count == count - 1 {
        for i in 0..count {
            if !decisions[i] {
                return 200 + i as i32;
            }
        }
    }

    let mut alternating = true;
    for i in 1..count {
        if decisions[i] == decisions[i - 1] {
            alternating = false;
            break;
        }
    }

    if alternating {
        return 500 + special_count as i32;
    }

    let mut max_consecutive = 0;
    let mut current_consecutive = 0;
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

    special_count as i32
}

fn validate_sequence(sequence: &[u8]) -> i32 {
    let len = sequence.len();
    if len == 0 {
        return 0;
    }

    let mut bools = Vec::with_capacity(len);
    for &c in sequence {
        bools.push(parse_bool(c));
    }

    if !bools[0] {
        return -10;
    }

    if len > 1 && bools[len - 1] {
        return -11;
    }

    let mut consecutive = 1;
    for i in 1..len {
        if bools[i] == bools[i - 1] {
            consecutive += 1;
            if consecutive > 3 {
                return -12;
            }
        } else {
            consecutive = 1;
        }
    }

    let mut transitions = 0;
    for i in 1..len {
        if bools[i] != bools[i - 1] {
            transitions += 1;
        }
    }

    let transitions_usize = transitions as usize;
    if len <= 3 {
        if transitions_usize == 0 {
            return 1;
        }
        if transitions_usize == len - 1 {
            return 2;
        }
        10 + transitions
    } else if len <= 10 {
        if transitions_usize < len / 3 {
            return 20;
        }
        if transitions_usize > len / 2 {
            return 30;
        }
        25
    } else {
        if transitions_usize < 3 {
            return 40;
        }
        if transitions_usize > len - 3 {
            return 50;
        }
        45
    }
}

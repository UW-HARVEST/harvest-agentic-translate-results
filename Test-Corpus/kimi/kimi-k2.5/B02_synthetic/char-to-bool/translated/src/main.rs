use std::io::{self, BufRead, Write};

pub fn process_decisions(decision_string: &str, operation: i32, param: i32) -> i32 {
    let length = decision_string.len();
    
    if length == 0 {
        return -1;
    }
    
    match operation {
        0 => {
            if length < 3 {
                return -2;
            }
            let read = parse_bool(decision_string.chars().nth(0).unwrap());
            let write = parse_bool(decision_string.chars().nth(1).unwrap());
            let execute = parse_bool(decision_string.chars().nth(2).unwrap());
            apply_permissions(read, write, execute)
        }
        1 => {
            if length < 3 {
                return -2;
            }
            let cond1 = parse_bool(decision_string.chars().nth(0).unwrap());
            let cond2 = parse_bool(decision_string.chars().nth(1).unwrap());
            let cond3 = parse_bool(decision_string.chars().nth(2).unwrap());
            evaluate_conditions(cond1, cond2, cond3, param)
        }
        2 => {
            let count = length.min(32);
            let decisions: Vec<bool> = decision_string.chars().take(count).map(parse_bool).collect();
            configure_flags(&decisions)
        }
        3 => {
            validate_sequence(decision_string)
        }
        _ => -3,
    }
}

fn parse_bool(c: char) -> bool {
    c == 'y' || c == 'Y'
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
            permission_value
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
    let mut flags: u32 = 0;
    let mut special_count = 0;
    
    for (i, &decision) in decisions.iter().enumerate() {
        if decision {
            flags |= 1u32 << i;
            special_count += 1;
        }
    }
    
    if special_count == 0 {
        0
    } else if special_count == count {
        1000 + count as i32
    } else if special_count == 1 {
        for (i, &decision) in decisions.iter().enumerate() {
            if decision {
                return 100 + i as i32;
            }
        }
        0
    } else if special_count == count - 1 {
        for (i, &decision) in decisions.iter().enumerate() {
            if !decision {
                return 200 + i as i32;
            }
        }
        0
    } else {
        let mut alternating = true;
        for i in 1..count {
            if decisions[i] == decisions[i-1] {
                alternating = false;
                break;
            }
        }
        
        if alternating {
            return 500 + special_count as i32;
        }
        
        let mut max_consecutive = 0;
        let mut current_consecutive = 0;
        for &decision in decisions {
            if decision {
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
}

fn validate_sequence(sequence: &str) -> i32 {
    let len = sequence.len();
    if len == 0 {
        return 0;
    }
    
    let bools: Vec<bool> = sequence.chars().map(parse_bool).collect();
    
    if !bools[0] {
        return -10;
    }
    
    if len > 1 && bools[len-1] {
        return -11;
    }
    
    let mut consecutive = 1;
    for i in 1..len {
        if bools[i] == bools[i-1] {
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
        if bools[i] != bools[i-1] {
            transitions += 1;
        }
    }
    
    if len <= 3 {
        if transitions == 0 { return 1; }
        if transitions == len - 1 { return 2; }
        10 + transitions as i32
    } else if len <= 10 {
        if transitions < len / 3 { return 20; }
        if transitions > len / 2 { return 30; }
        25
    } else {
        if transitions < 3 { return 40; }
        if transitions > len - 3 { return 50; }
        45
    }
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();
    
    let operation: i32 = lines.next()
        .and_then(|l| l.ok())
        .and_then(|l| l.trim().parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Error reading operation");
            std::process::exit(1);
        });
    
    let param: i32 = lines.next()
        .and_then(|l| l.ok())
        .and_then(|l| l.trim().parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Error reading parameter");
            std::process::exit(1);
        });
    
    let input_buffer = lines.next()
        .and_then(|l| l.ok())
        .unwrap_or_else(|| {
            eprintln!("Error reading decision string");
            std::process::exit(1);
        });
    
    let decision_string = input_buffer.trim_end_matches('\n');
    
    let result = process_decisions(decision_string, operation, param);
    
    writeln!(stdout, "{}", result).unwrap();
}

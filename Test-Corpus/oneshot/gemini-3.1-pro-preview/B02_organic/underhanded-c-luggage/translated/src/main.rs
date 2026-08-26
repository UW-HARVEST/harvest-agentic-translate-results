use std::io::{self, Read};

#[derive(Clone, Debug)]
struct RoutingDirective {
    time_stamp: u32,
    luggage_id: String,
    flight_id: String,
    departure: String,
    arrival: String,
    comments: String,
}

fn supersedes(directives: &[RoutingDirective], current_idx: usize) -> bool {
    let current = &directives[current_idx];
    for directive in &directives[current_idx + 1..] {
        if directive.luggage_id != current.luggage_id {
            continue;
        }
        if directive.departure == current.departure {
            return true;
        }
        return false;
    }
    false
}

fn matches(expected: &str, actual: &str) -> bool {
    expected.starts_with('-') || expected == actual
}

fn print_matching_directives(
    directives: &[RoutingDirective],
    expected_luggage_id: &str,
    expected_flight_id: &str,
    expected_departure: &str,
    expected_arrival: &str,
) {
    for (i, directive) in directives.iter().enumerate() {
        if !supersedes(directives, i)
            && matches(expected_luggage_id, &directive.luggage_id)
            && matches(expected_flight_id, &directive.flight_id)
            && matches(expected_departure, &directive.departure)
            && matches(expected_arrival, &directive.arrival)
        {
            println!(
                "{:010} {} {} {} {} {}",
                directive.time_stamp,
                directive.luggage_id,
                directive.flight_id,
                directive.departure,
                directive.arrival,
                directive.comments
            );
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("Command line error: 4 arguments expected");
        std::process::exit(1);
    }

    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    let mut directives = Vec::new();
    let mut cursor = input.as_str();

    loop {
        cursor = cursor.trim_start();
        if cursor.is_empty() {
            break;
        }

        let ts_end = cursor.find(|c: char| !c.is_ascii_digit()).unwrap_or(cursor.len());
        if ts_end == 0 {
            break;
        }
        let time_stamp: u32 = match cursor[..ts_end].parse() {
            Ok(ts) => ts,
            Err(_) => break,
        };
        cursor = &cursor[ts_end..];

        cursor = cursor.trim_start();

        let lug_end = cursor
            .find(|c: char| !matches!(c, 'A'..='Z' | '0'..='9'))
            .unwrap_or(cursor.len())
            .min(8);
        if lug_end == 0 {
            break;
        }
        let luggage_id = cursor[..lug_end].to_string();
        cursor = &cursor[lug_end..];

        cursor = cursor.trim_start();

        let fli_end = cursor
            .find(|c: char| !matches!(c, 'A'..='Z' | '0'..='9'))
            .unwrap_or(cursor.len())
            .min(6);
        if fli_end == 0 {
            break;
        }
        let flight_id = cursor[..fli_end].to_string();
        cursor = &cursor[fli_end..];

        cursor = cursor.trim_start();

        let dep_end = cursor
            .find(|c: char| !matches!(c, 'A'..='Z'))
            .unwrap_or(cursor.len())
            .min(3);
        if dep_end == 0 {
            break;
        }
        let departure = cursor[..dep_end].to_string();
        cursor = &cursor[dep_end..];

        cursor = cursor.trim_start();

        let arr_end = cursor
            .find(|c: char| !matches!(c, 'A'..='Z'))
            .unwrap_or(cursor.len())
            .min(3);
        if arr_end == 0 {
            break;
        }
        let arrival = cursor[..arr_end].to_string();
        cursor = &cursor[arr_end..];

        let com_end = cursor.find('\n').unwrap_or(cursor.len());
        let actual_com_end = cursor[..com_end]
            .char_indices()
            .nth(80)
            .map(|(idx, _)| idx)
            .unwrap_or(com_end);
        let mut comments = cursor[..actual_com_end].to_string();
        if comments.ends_with('\r') {
            comments.pop();
        }
        cursor = &cursor[actual_com_end..];

        directives.push(RoutingDirective {
            time_stamp,
            luggage_id,
            flight_id,
            departure,
            arrival,
            comments,
        });
    }

    directives.sort_by_key(|d| d.time_stamp);

    print_matching_directives(&directives, &args[1], &args[2], &args[3], &args[4]);
}

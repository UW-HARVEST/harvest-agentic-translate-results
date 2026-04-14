use std::env;
use std::io::{self, Read};
use std::process;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

#[derive(Clone)]
struct RoutingDirective {
    time_stamp: u32,
    luggage_id: String,
    flight_id: String,
    departure: String,
    arrival: String,
    comments: String,
}

fn add_routing_directive_to_list(list: &mut Vec<RoutingDirective>, new_directive: RoutingDirective) {
    let pos = list
        .iter()
        .position(|d| d.time_stamp > new_directive.time_stamp)
        .unwrap_or(list.len());
    list.insert(pos, new_directive);
}

fn supersedes(directives: &[RoutingDirective], luggage_id: &str, departure: &str) -> bool {
    directives
        .iter()
        .any(|directive| directive.luggage_id == luggage_id && directive.departure == departure)
}

fn superseded(directives: &[RoutingDirective], index: usize) -> bool {
    let directive = &directives[index];
    supersedes(&directives[index + 1..], &directive.luggage_id, &directive.departure)
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
        if !superseded(directives, i)
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

fn parse_token(bytes: &[u8], idx: &mut usize) -> Option<String> {
    while *idx < bytes.len() && bytes[*idx].is_ascii_whitespace() {
        *idx += 1;
    }
    if *idx >= bytes.len() {
        return None;
    }
    let start = *idx;
    while *idx < bytes.len() && !bytes[*idx].is_ascii_whitespace() {
        *idx += 1;
    }
    Some(String::from_utf8_lossy(&bytes[start..*idx]).into_owned())
}

fn parse_line(line: &str) -> Option<RoutingDirective> {
    let bytes = line.as_bytes();
    let mut idx = 0;

    let time_stamp: u32 = parse_token(bytes, &mut idx)?.parse().ok()?;
    let luggage_id = parse_token(bytes, &mut idx)?;
    let flight_id = parse_token(bytes, &mut idx)?;
    let departure = parse_token(bytes, &mut idx)?;
    let arrival = parse_token(bytes, &mut idx)?;

    if luggage_id.len() > LUGGAGE_ID_LENGTH
        || flight_id.len() > FLIGHT_ID_LENGTH
        || departure.len() > AIRPORT_CODE_LENGTH
        || arrival.len() > AIRPORT_CODE_LENGTH
    {
        return None;
    }

    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() && bytes[idx] != b'\n' && bytes[idx] != b'\r' {
        idx += 1;
    }

    let comments = line[idx..]
        .trim_end_matches(['\n', '\r'])
        .chars()
        .take(COMMENTS_LENGTH)
        .collect::<String>();

    Some(RoutingDirective {
        time_stamp,
        luggage_id,
        flight_id,
        departure,
        arrival,
        comments,
    })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!("Command line error: 4 arguments expected");
        process::exit(1);
    }

    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        process::exit(0);
    }

    let mut directives = Vec::new();
    for line in input.lines() {
        if let Some(directive) = parse_line(line) {
            add_routing_directive_to_list(&mut directives, directive);
        } else {
            break;
        }
    }

    print_matching_directives(&directives, &args[1], &args[2], &args[3], &args[4]);
    process::exit(0);
}

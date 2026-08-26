use std::io::{self, BufRead};

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

struct RoutingDirective {
    time_stamp: u32,
    luggage_id: String,
    flight_id: String,
    departure: String,
    arrival: String,
    comments: String,
    next_directive: Option<Box<RoutingDirective>>,
}

fn add_routing_directive_to_list(
    previous_directive: &mut RoutingDirective,
    new_directive: Box<RoutingDirective>,
) {
    match &mut previous_directive.next_directive {
        None => {
            previous_directive.next_directive = Some(new_directive);
        }
        Some(next) => {
            if next.time_stamp > new_directive.time_stamp {
                let mut new_directive = new_directive;
                new_directive.next_directive = previous_directive.next_directive.take();
                previous_directive.next_directive = Some(new_directive);
            } else {
                add_routing_directive_to_list(next, new_directive);
            }
        }
    }
}

fn supersedes(directive: Option<&RoutingDirective>, luggage_id: &str, departure: &str) -> bool {
    let directive = match directive {
        None => return false,
        Some(d) => d,
    };
    if directive.luggage_id != luggage_id {
        return supersedes(directive.next_directive.as_deref(), luggage_id, departure);
    }
    directive.departure == departure
}

fn superseded(directive: &RoutingDirective) -> bool {
    supersedes(
        directive.next_directive.as_deref(),
        &directive.luggage_id,
        &directive.departure,
    )
}

fn matches(expected: &str, actual: &str) -> bool {
    expected == "-" || expected == actual
}

fn print_matching_directives(
    first_directive: Option<&RoutingDirective>,
    expected_luggage_id: &str,
    expected_flight_id: &str,
    expected_departure: &str,
    expected_arrival: &str,
) {
    let mut directive = first_directive;
    while let Some(d) = directive {
        if !superseded(d)
            && matches(expected_luggage_id, &d.luggage_id)
            && matches(expected_flight_id, &d.flight_id)
            && matches(expected_departure, &d.departure)
            && matches(expected_arrival, &d.arrival)
        {
            println!(
                "{:010} {} {} {} {} {}",
                d.time_stamp, d.luggage_id, d.flight_id, d.departure, d.arrival, d.comments
            );
        }
        directive = d.next_directive.as_deref();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("Command line error: 4 arguments expected");
        std::process::exit(1);
    }

    let mut directive_list_head = RoutingDirective {
        time_stamp: 0,
        luggage_id: String::new(),
        flight_id: String::new(),
        departure: String::new(),
        arrival: String::new(),
        comments: String::new(),
        next_directive: None,
    };

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let time_stamp: u32 = match parts[0].parse() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let luggage_id = parts[1].to_string();
        let flight_id = parts[2].to_string();
        let departure = parts[3].to_string();
        let arrival = parts[4].to_string();
        let comments = if parts.len() > 5 {
            parts[5..].join(" ")
        } else {
            String::new()
        };

        let new_directive = Box::new(RoutingDirective {
            time_stamp,
            luggage_id,
            flight_id,
            departure,
            arrival,
            comments,
            next_directive: None,
        });

        add_routing_directive_to_list(&mut directive_list_head, new_directive);
    }

    print_matching_directives(
        directive_list_head.next_directive.as_deref(),
        &args[1],
        &args[2],
        &args[3],
        &args[4],
    );
}

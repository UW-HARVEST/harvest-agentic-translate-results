// Translation of c_src/src/luggage.c ("By Jan Wrobel <wrr@mixedbit.org>").
//
// Behaviour is reproduced exactly, including the original's quirks:
//   * only `EOF` is checked from `scanf`, so matching failures silently leave
//     the previous iteration's buffer contents in place (in C those are the
//     same stack slots every iteration);
//   * `%d` is read into an `unsigned int`, so negative / oversized values wrap;
//   * `%80[^\n]` normally captures the space that separates the arrival code
//     from the comments, which is why the printed line contains two spaces;
//   * `supersedes()` stops at the first later directive with the same luggage
//     id and only reports a supersession when that directive's departure
//     matches, instead of scanning the whole remaining list.
//
// The singly linked list of the original is kept as a linked list (nodes in an
// arena, linked by index) rather than a sorted `Vec`, so that insertions have
// the same cost profile as the C code: appending a node is O(1) and only the
// list walk costs anything, exactly as in `addRoutingDirectiveToList()`.
// The two recursive C functions are written as loops; they visit the very same
// nodes in the same order, they just cannot overflow the stack.

mod scan;

use scan::{scan_airports, scan_comments, scan_ids, scan_time_stamp, Reader, EOF};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

/// Restores the default `SIGPIPE` disposition, which every C program starts
/// with.  Rust's runtime sets `SIGPIPE` to `SIG_IGN`, which would make a closed
/// stdout end in exit status 0 instead of death by signal 13 as in C.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // SAFETY: `signal(SIGPIPE, SIG_DFL)` merely resets a signal disposition to
    // the value the kernel gives a freshly exec'd process; it touches no memory.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// The C `RoutingDirective`.  The character arrays are modelled as the byte
/// string up to their NUL terminator, which is what `strcpy`, `strcmp` and
/// `%s` see.
#[derive(Default)]
struct RoutingDirective {
    time_stamp: u32,
    luggage_id: Vec<u8>,
    flight_id: Vec<u8>,
    departure: Vec<u8>,
    arrival: Vec<u8>,
    comments: Vec<u8>,
}

/// A `RoutingDirective` plus its `next_directive` link, held by index.
struct Node {
    directive: RoutingDirective,
    next_directive: Option<usize>,
}

/// The list, including the `directive_list_head` sentinel of `main()` as node 0
/// (`time_stamp = 0`, `next_directive = NULL`).
struct DirectiveList {
    nodes: Vec<Node>,
}

const HEAD: usize = 0;

impl DirectiveList {
    fn new() -> DirectiveList {
        DirectiveList {
            nodes: vec![Node {
                directive: RoutingDirective::default(),
                next_directive: None,
            }],
        }
    }

    fn directive(&self, index: usize) -> &RoutingDirective {
        &self.nodes[index].directive
    }

    fn first(&self) -> Option<usize> {
        self.nodes[HEAD].next_directive
    }

    /// `addRoutingDirectiveToList()`: walks forward while the next directive's
    /// time stamp is less than or equal to the new one, then links the new
    /// directive in.  Equal time stamps therefore keep input order.
    fn add_routing_directive_to_list(&mut self, new_directive: RoutingDirective) {
        let mut previous = HEAD;
        while let Some(next) = self.nodes[previous].next_directive {
            if self.nodes[next].directive.time_stamp > new_directive.time_stamp {
                break;
            }
            previous = next;
        }
        let new_index = self.nodes.len();
        let next_directive = self.nodes[previous].next_directive;
        self.nodes.push(Node {
            directive: new_directive,
            next_directive,
        });
        self.nodes[previous].next_directive = Some(new_index);
    }

    /// `supersedes()`: walks forward, skipping directives for other luggage;
    /// the first directive for the same luggage decides the answer, even when
    /// it says "no".
    fn supersedes(&self, start: Option<usize>, luggage_id: &[u8], departure: &[u8]) -> bool {
        let mut current = start;
        while let Some(index) = current {
            let directive = self.directive(index);
            if directive.luggage_id != luggage_id {
                current = self.nodes[index].next_directive;
                continue;
            }
            return directive.departure == departure;
        }
        false
    }

    /// `superseded()`: is this directive overridden by a later one?
    fn superseded(&self, index: usize) -> bool {
        let directive = self.directive(index);
        self.supersedes(
            self.nodes[index].next_directive,
            &directive.luggage_id,
            &directive.departure,
        )
    }
}

/// `matches()`: `-` as the first character is the wildcard.
fn matches(expected: &[u8], actual: &[u8]) -> bool {
    expected.first() == Some(&b'-') || expected == actual
}

/// `printMatchingDirectives()`
fn print_matching_directives(
    list: &DirectiveList,
    expected_luggage_id: &[u8],
    expected_flight_id: &[u8],
    expected_departure: &[u8],
    expected_arrival: &[u8],
    out: &mut impl Write,
) {
    let mut current = list.first();
    while let Some(index) = current {
        let directive = list.directive(index);
        if !list.superseded(index)
            && matches(expected_luggage_id, &directive.luggage_id)
            && matches(expected_flight_id, &directive.flight_id)
            && matches(expected_departure, &directive.departure)
            && matches(expected_arrival, &directive.arrival)
        {
            // printf("%010u %s %s %s %s %s\n", ...)
            let mut line: Vec<u8> = Vec::new();
            line.extend_from_slice(format!("{:010}", directive.time_stamp).as_bytes());
            for field in [
                &directive.luggage_id,
                &directive.flight_id,
                &directive.departure,
                &directive.arrival,
                &directive.comments,
            ] {
                line.push(b' ');
                line.extend_from_slice(field);
            }
            line.push(b'\n');
            let _ = out.write_all(&line);
        }
        current = list.nodes[index].next_directive;
    }
}

fn main() {
    restore_default_sigpipe();

    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if argv.len() != 5 {
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"Command line error: 4 arguments expected\n");
        let _ = stderr.flush();
        std::process::exit(1);
    }

    let mut list = DirectiveList::new();
    let mut reader = Reader::from_stdin();

    // In C these live in main's stack frame, so a conversion that fails leaves
    // the value written by an earlier iteration behind.  Fresh stack memory is
    // modelled as zero-filled, i.e. 0 / empty strings.
    let mut time_stamp: u32 = 0;
    let mut luggage_id: Vec<u8> = Vec::new();
    let mut flight_id: Vec<u8> = Vec::new();
    let mut departure: Vec<u8> = Vec::new();
    let mut arrival: Vec<u8> = Vec::new();
    let mut comments: Vec<u8> = Vec::new();

    loop {
        comments.clear(); // comments[0] = 0; // comments are optional.

        if scan_time_stamp(&mut reader, &mut time_stamp) == EOF {
            break;
        }
        if scan_ids(
            &mut reader,
            &mut luggage_id,
            LUGGAGE_ID_LENGTH,
            &mut flight_id,
            FLIGHT_ID_LENGTH,
        ) == EOF
        {
            break;
        }
        if scan_airports(
            &mut reader,
            &mut departure,
            AIRPORT_CODE_LENGTH,
            &mut arrival,
            AIRPORT_CODE_LENGTH,
        ) == EOF
        {
            break;
        }
        if scan_comments(&mut reader, &mut comments, COMMENTS_LENGTH) == EOF {
            break;
        }

        list.add_routing_directive_to_list(RoutingDirective {
            time_stamp,
            luggage_id: luggage_id.clone(),
            flight_id: flight_id.clone(),
            departure: departure.clone(),
            arrival: arrival.clone(),
            comments: comments.clone(),
        });
    }

    let stdout = std::io::stdout();
    // glibc gives a pipe-backed stdout a 4096-byte buffer; matching it keeps the
    // flush boundaries (and therefore partial output on a broken pipe) the same.
    let mut out = std::io::BufWriter::with_capacity(4096, stdout.lock());
    print_matching_directives(
        &list,
        argv[1].as_bytes(),
        argv[2].as_bytes(),
        argv[3].as_bytes(),
        argv[4].as_bytes(),
        &mut out,
    );
    let _ = out.flush();
    std::process::exit(0);
}

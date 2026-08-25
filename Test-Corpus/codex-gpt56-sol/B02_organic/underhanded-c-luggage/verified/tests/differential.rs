use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

const C_LIBRARY: &str = "c_src/build/libdriver_c.so";
const RANDOM_CASES: usize = 32;
const FILTER_ROWS: [&str; 16] = [
    "C20", "C21", "C22", "C23", "C24", "C25", "C26", "C27", "C28", "C29", "C30", "C31", "C32",
    "C33", "C34", "C35",
];

#[repr(C)]
#[derive(Clone)]
struct RoutingDirective {
    time_stamp: u32,
    luggage_id: [c_char; 9],
    flight_id: [c_char; 7],
    departure: [c_char; 4],
    arrival: [c_char; 4],
    comments: [c_char; 81],
    next_directive: *mut RoutingDirective,
}

type Add = unsafe extern "C" fn(*mut RoutingDirective, *mut RoutingDirective);
type Supersedes = unsafe extern "C" fn(*mut RoutingDirective, *mut c_char, *mut c_char) -> c_int;
type Superseded = unsafe extern "C" fn(*mut RoutingDirective) -> c_int;
type Matches = unsafe extern "C" fn(*mut c_char, *mut c_char) -> c_int;
type Print =
    unsafe extern "C" fn(*mut RoutingDirective, *mut c_char, *mut c_char, *mut c_char, *mut c_char);
type Main = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

struct Api {
    _library: Library,
    add: Add,
    supersedes: Supersedes,
    superseded: Superseded,
    matches: Matches,
    print: Print,
    main: Main,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap();
        let add = unsafe { *library.get(b"addRoutingDirectiveToList\0").unwrap() };
        let supersedes = unsafe { *library.get(b"supersedes\0").unwrap() };
        let superseded = unsafe { *library.get(b"superseded\0").unwrap() };
        let matches = unsafe { *library.get(b"matches\0").unwrap() };
        let print = unsafe { *library.get(b"printMatchingDirectives\0").unwrap() };
        let main = unsafe { *library.get(b"main\0").unwrap() };
        Self {
            _library: library,
            add,
            supersedes,
            superseded,
            matches,
            print,
            main,
        }
    }
}

#[derive(Clone)]
struct DirectiveSpec {
    time_stamp: u32,
    luggage_id: String,
    flight_id: String,
    departure: String,
    arrival: String,
    comments: String,
}

struct List {
    nodes: Vec<Box<RoutingDirective>>,
    head: *mut RoutingDirective,
}

impl List {
    fn from_specs(specs: &[DirectiveSpec]) -> Self {
        let mut nodes: Vec<_> = specs.iter().map(node_from_spec).collect();
        for index in 0..nodes.len().saturating_sub(1) {
            let next = &mut *nodes[index + 1] as *mut RoutingDirective;
            nodes[index].next_directive = next;
        }
        let head = nodes
            .first_mut()
            .map(|node| &mut **node as *mut RoutingDirective)
            .unwrap_or(ptr::null_mut());
        Self { nodes, head }
    }

    fn timestamps(&self, first: *mut RoutingDirective) -> Vec<u32> {
        let mut result = Vec::new();
        let mut cursor = first;
        while !cursor.is_null() {
            assert!(result.len() <= self.nodes.len());
            result.push(unsafe { (*cursor).time_stamp });
            cursor = unsafe { (*cursor).next_directive };
        }
        result
    }
}

fn node_from_spec(spec: &DirectiveSpec) -> Box<RoutingDirective> {
    Box::new(RoutingDirective {
        time_stamp: spec.time_stamp,
        luggage_id: c_array::<9>(&spec.luggage_id),
        flight_id: c_array::<7>(&spec.flight_id),
        departure: c_array::<4>(&spec.departure),
        arrival: c_array::<4>(&spec.arrival),
        comments: c_array::<81>(&spec.comments),
        next_directive: ptr::null_mut(),
    })
}

fn c_array<const N: usize>(value: &str) -> [c_char; N] {
    let mut result = [0; N];
    assert!(value.len() < N);
    for (destination, source) in result.iter_mut().zip(value.as_bytes()) {
        *destination = *source as c_char;
    }
    result
}

fn c_string(value: &str) -> Vec<c_char> {
    CString::new(value)
        .unwrap()
        .into_bytes_with_nul()
        .into_iter()
        .map(|byte| byte as c_char)
        .collect()
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    root().join(C_LIBRARY)
}

fn rust_library_path() -> PathBuf {
    let deps = root().join("target/debug/deps");
    let cargo_artifact = fs::read_dir(&deps)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libdriver") && name.ends_with(".so"))
        })
        .unwrap_or_else(|| root().join("target/debug/libdriver.so"));
    assert!(
        cargo_artifact.exists(),
        "Rust cdylib not found under {}",
        deps.display()
    );
    cargo_artifact
}

fn apis() -> (Api, Api) {
    assert!(c_library_path().exists(), "C shared library was not built");
    assert!(
        rust_library_path().exists(),
        "Rust shared library was not built"
    );
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

struct Random(u64);

impl Random {
    fn new() -> Self {
        Self(0x7d53_2a91_c4e8_b607)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn text(&mut self, len: usize, alphabet: &[u8]) -> String {
        (0..len)
            .map(|_| alphabet[self.next_u32() as usize % alphabet.len()] as char)
            .collect()
    }

    fn luggage(&mut self) -> String {
        self.text(8, b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")
    }

    fn flight(&mut self) -> String {
        self.text(6, b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")
    }

    fn airport(&mut self) -> String {
        self.text(3, b"ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    }
}

fn random_spec(random: &mut Random, timestamp: u32) -> DirectiveSpec {
    DirectiveSpec {
        time_stamp: timestamp,
        luggage_id: random.luggage(),
        flight_id: random.flight(),
        departure: random.airport(),
        arrival: random.airport(),
        comments: random.text(12, b"abcdefghijklmnopqrstuvwxyz "),
    }
}

fn call_supersedes(function: Supersedes, list: &mut List, luggage: &str, departure: &str) -> c_int {
    let mut luggage = c_string(luggage);
    let mut departure = c_string(departure);
    unsafe { function(list.head, luggage.as_mut_ptr(), departure.as_mut_ptr()) }
}

#[test]
fn low_level_list_and_predicates_match() {
    let (c, rust) = apis();
    let mut random = Random::new();

    for iteration in 0..RANDOM_CASES {
        let base = random.next_u32() % 10_000;
        let cases = [
            (vec![], base, "C01"),
            (vec![base + 10], base, "C02"),
            (vec![base, base + 20], base + 10, "C03"),
            (vec![base, base + 10], base + 10, "C04"),
        ];
        for (timestamps, inserted, row) in cases {
            let specs: Vec<_> = timestamps
                .iter()
                .map(|timestamp| random_spec(&mut random, *timestamp))
                .collect();
            let c_list = List::from_specs(&specs);
            let rust_list = List::from_specs(&specs);
            let mut c_head = Box::new(RoutingDirective {
                next_directive: c_list.head,
                ..*node_from_spec(&random_spec(&mut random, 0))
            });
            let mut rust_head = Box::new(RoutingDirective {
                next_directive: rust_list.head,
                ..(*c_head).clone()
            });
            let inserted_spec = random_spec(&mut random, inserted);
            let mut c_new = node_from_spec(&inserted_spec);
            let mut rust_new = node_from_spec(&inserted_spec);
            unsafe {
                (c.add)(&mut *c_head, &mut *c_new);
                (rust.add)(&mut *rust_head, &mut *rust_new);
            }
            assert_eq!(
                c_list.timestamps(c_head.next_directive),
                rust_list.timestamps(rust_head.next_directive),
                "{row}, iteration {iteration}"
            );
        }
    }

    let mut empty_c = List::from_specs(&[]);
    let mut empty_rust = List::from_specs(&[]);
    assert_eq!(
        call_supersedes(c.supersedes, &mut empty_c, "BAG", "AAA"),
        call_supersedes(rust.supersedes, &mut empty_rust, "BAG", "AAA"),
        "C05/E01"
    );

    for iteration in 0..RANDOM_CASES {
        let luggage = random.luggage();
        let departure = random.airport();
        let other_luggage = random.luggage();
        let other_departure = random.airport();
        let base = random_spec(&mut random, iteration as u32);
        let matching = DirectiveSpec {
            luggage_id: luggage.clone(),
            departure: departure.clone(),
            ..base.clone()
        };
        let unrelated = DirectiveSpec {
            luggage_id: other_luggage.clone(),
            departure: other_departure.clone(),
            ..random_spec(&mut random, iteration as u32 + 1)
        };
        let wrong_departure = DirectiveSpec {
            luggage_id: luggage.clone(),
            departure: other_departure.clone(),
            ..random_spec(&mut random, iteration as u32 + 2)
        };
        let cases = [
            (vec![matching.clone()], 1, "C06"),
            (vec![unrelated.clone(), matching.clone()], 1, "C07"),
            (vec![unrelated.clone(), unrelated.clone()], 0, "C08"),
            (vec![wrong_departure.clone()], 0, "C09"),
            (vec![wrong_departure.clone(), matching.clone()], 0, "C10"),
        ];
        for (specs, expected, row) in cases {
            let mut c_list = List::from_specs(&specs);
            let mut rust_list = List::from_specs(&specs);
            let c_result = call_supersedes(c.supersedes, &mut c_list, &luggage, &departure);
            let rust_result =
                call_supersedes(rust.supersedes, &mut rust_list, &luggage, &departure);
            assert_eq!(c_result, expected, "{row}, C, iteration {iteration}");
            assert_eq!(rust_result, c_result, "{row}, iteration {iteration}");
        }

        let superseded_cases = [
            (vec![matching.clone()], 0, "C11"),
            (vec![matching.clone(), matching.clone()], 1, "C12"),
            (vec![matching.clone(), wrong_departure.clone()], 0, "C13"),
            (
                vec![matching.clone(), unrelated.clone(), matching.clone()],
                1,
                "C14",
            ),
        ];
        for (specs, expected, row) in superseded_cases {
            let c_list = List::from_specs(&specs);
            let rust_list = List::from_specs(&specs);
            let c_result = unsafe { (c.superseded)(c_list.head) };
            let rust_result = unsafe { (rust.superseded)(rust_list.head) };
            assert_eq!(c_result, expected, "{row}, C, iteration {iteration}");
            assert_eq!(rust_result, c_result, "{row}, iteration {iteration}");
        }

        let mut wildcard = c_string("-anything");
        let equal_value = random.luggage();
        let mut equal_left = c_string(&equal_value);
        let mut equal_right = c_string(&equal_value);
        let mut unequal = c_string(&other_luggage);
        assert_eq!(
            unsafe { (c.matches)(wildcard.as_mut_ptr(), ptr::null_mut()) },
            unsafe { (rust.matches)(wildcard.as_mut_ptr(), ptr::null_mut()) },
            "C15"
        );
        assert_eq!(
            unsafe { (c.matches)(equal_left.as_mut_ptr(), equal_right.as_mut_ptr()) },
            unsafe { (rust.matches)(equal_left.as_mut_ptr(), equal_right.as_mut_ptr()) },
            "C16"
        );
        assert_eq!(
            unsafe { (c.matches)(equal_left.as_mut_ptr(), unequal.as_mut_ptr()) },
            unsafe { (rust.matches)(equal_left.as_mut_ptr(), unequal.as_mut_ptr()) },
            "C17"
        );
    }
}

unsafe extern "C" {
    fn pipe(file_descriptors: *mut c_int) -> c_int;
    fn dup2(old_file_descriptor: c_int, new_file_descriptor: c_int) -> c_int;
    fn close(file_descriptor: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(process_id: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn capture_child_stdout(action: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let mut descriptors = [0; 2];
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(descriptors.as_mut_ptr()), 0);
    }
    let process_id = unsafe { fork() };
    assert!(process_id >= 0);
    if process_id == 0 {
        unsafe {
            close(descriptors[0]);
            dup2(descriptors[1], 1);
            close(descriptors[1]);
        }
        action();
        unsafe {
            fflush(ptr::null_mut());
            _exit(0);
        }
    }
    unsafe {
        close(descriptors[1]);
    }
    let mut output = Vec::new();
    unsafe { File::from_raw_fd(descriptors[0]) }
        .read_to_end(&mut output)
        .unwrap();
    let mut status = 0;
    assert_eq!(unsafe { waitpid(process_id, &mut status, 0) }, process_id);
    assert_eq!(status, 0);
    output
}

fn call_print(function: Print, list: &mut List, filter_values: [&str; 4]) -> Vec<u8> {
    let mut filters: Vec<_> = filter_values.into_iter().map(c_string).collect();
    capture_child_stdout(|| unsafe {
        function(
            list.head,
            filters[0].as_mut_ptr(),
            filters[1].as_mut_ptr(),
            filters[2].as_mut_ptr(),
            filters[3].as_mut_ptr(),
        )
    })
}

fn compare_print(
    c: &Api,
    rust: &Api,
    specs: &[DirectiveSpec],
    filters: [&str; 4],
    row: &str,
) -> Vec<u8> {
    let mut c_list = List::from_specs(specs);
    let mut rust_list = List::from_specs(specs);
    let c_output = call_print(c.print, &mut c_list, filters);
    let rust_output = call_print(rust.print, &mut rust_list, filters);
    assert_eq!(rust_output, c_output, "{row}");
    c_output
}

#[test]
fn print_configurations_match() {
    let (c, rust) = apis();
    let empty = compare_print(
        &c,
        &rust,
        &[],
        ["unused", "unused", "unused", "unused"],
        "C18",
    );
    assert!(empty.is_empty());

    let mut random = Random::new();
    for iteration in 0..RANDOM_CASES {
        let timestamp = random.next_u32();
        let spec = random_spec(&mut random, timestamp);
        let exact = [
            spec.luggage_id.as_str(),
            spec.flight_id.as_str(),
            spec.departure.as_str(),
            spec.arrival.as_str(),
        ];
        let output = compare_print(&c, &rust, &[spec.clone()], exact, "C19");
        assert!(!output.is_empty(), "C19, iteration {iteration}");

        for mask in 0_u8..16 {
            let filters = [
                if mask & 1 == 0 {
                    spec.luggage_id.as_str()
                } else {
                    "-"
                },
                if mask & 2 == 0 {
                    spec.flight_id.as_str()
                } else {
                    "-"
                },
                if mask & 4 == 0 {
                    spec.departure.as_str()
                } else {
                    "-"
                },
                if mask & 8 == 0 {
                    spec.arrival.as_str()
                } else {
                    "-"
                },
            ];
            let row = format!("{}, iteration {iteration}", FILTER_ROWS[mask as usize]);
            let output = compare_print(&c, &rust, &[spec.clone()], filters, &row);
            assert!(!output.is_empty(), "{row}");
        }

        let wrong_luggage = random.luggage();
        let wrong_flight = random.flight();
        let wrong_departure = random.airport();
        let wrong_arrival = random.airport();
        let mismatch_cases = [
            (
                [
                    wrong_luggage.as_str(),
                    spec.flight_id.as_str(),
                    spec.departure.as_str(),
                    spec.arrival.as_str(),
                ],
                "C36",
            ),
            (
                [
                    spec.luggage_id.as_str(),
                    wrong_flight.as_str(),
                    spec.departure.as_str(),
                    spec.arrival.as_str(),
                ],
                "C37",
            ),
            (
                [
                    spec.luggage_id.as_str(),
                    spec.flight_id.as_str(),
                    wrong_departure.as_str(),
                    spec.arrival.as_str(),
                ],
                "C38",
            ),
            (
                [
                    spec.luggage_id.as_str(),
                    spec.flight_id.as_str(),
                    spec.departure.as_str(),
                    wrong_arrival.as_str(),
                ],
                "C39",
            ),
        ];
        for (filters, row) in mismatch_cases {
            let output = compare_print(&c, &rust, &[spec.clone()], filters, row);
            assert!(output.is_empty(), "{row}, iteration {iteration}");
        }

        let later_same = DirectiveSpec {
            time_stamp: spec.time_stamp.wrapping_add(1),
            comments: "later-same".to_owned(),
            ..spec.clone()
        };
        let output = compare_print(
            &c,
            &rust,
            &[spec.clone(), later_same],
            ["-", "-", "-", "-"],
            "C40",
        );
        assert!(!output
            .windows(12)
            .any(|bytes| bytes == spec.comments.as_bytes()));

        let later_different = DirectiveSpec {
            time_stamp: spec.time_stamp.wrapping_add(1),
            departure: wrong_departure.clone(),
            comments: "later-different".to_owned(),
            ..spec.clone()
        };
        let output = compare_print(
            &c,
            &rust,
            &[spec.clone(), later_different],
            ["-", "-", "-", "-"],
            "C41",
        );
        assert!(output
            .windows(spec.comments.len())
            .any(|bytes| bytes == spec.comments.as_bytes()));
    }

    for iteration in 0..RANDOM_CASES {
        let mut specs: Vec<_> = (0..12)
            .map(|_| {
                let timestamp = random.next_u32();
                random_spec(&mut random, timestamp)
            })
            .collect();
        specs.sort_by_key(|spec| spec.time_stamp);
        compare_print(
            &c,
            &rust,
            &specs,
            ["-", "-", "-", "-"],
            &format!("C42, iteration {iteration}"),
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
struct MainOutcome {
    exit_code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_exported_main(api: &Api, user_arguments: &[&str], input: &[u8]) -> MainOutcome {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let mut arguments: Vec<_> = std::iter::once("driver")
        .chain(user_arguments.iter().copied())
        .map(|value| CString::new(value).unwrap())
        .collect();
    let mut argument_pointers: Vec<_> = arguments
        .iter_mut()
        .map(|argument| argument.as_ptr().cast_mut())
        .collect();
    let mut stdin_pipe = [0; 2];
    let mut stdout_pipe = [0; 2];
    let mut stderr_pipe = [0; 2];
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(stdin_pipe.as_mut_ptr()), 0);
        assert_eq!(pipe(stdout_pipe.as_mut_ptr()), 0);
        assert_eq!(pipe(stderr_pipe.as_mut_ptr()), 0);
    }

    let process_id = unsafe { fork() };
    assert!(process_id >= 0);
    if process_id == 0 {
        unsafe {
            close(stdin_pipe[1]);
            close(stdout_pipe[0]);
            close(stderr_pipe[0]);
            dup2(stdin_pipe[0], 0);
            dup2(stdout_pipe[1], 1);
            dup2(stderr_pipe[1], 2);
            close(stdin_pipe[0]);
            close(stdout_pipe[1]);
            close(stderr_pipe[1]);
            let result = (api.main)(
                argument_pointers.len() as c_int,
                argument_pointers.as_mut_ptr(),
            );
            _exit(result);
        }
    }

    unsafe {
        close(stdin_pipe[0]);
        close(stdout_pipe[1]);
        close(stderr_pipe[1]);
    }
    {
        let mut child_stdin = unsafe { File::from_raw_fd(stdin_pipe[1]) };
        child_stdin.write_all(input).unwrap();
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    unsafe { File::from_raw_fd(stdout_pipe[0]) }
        .read_to_end(&mut stdout)
        .unwrap();
    unsafe { File::from_raw_fd(stderr_pipe[0]) }
        .read_to_end(&mut stderr)
        .unwrap();
    let mut status = 0;
    assert_eq!(unsafe { waitpid(process_id, &mut status, 0) }, process_id);
    let signal = status & 0x7f;
    MainOutcome {
        exit_code: (signal == 0).then_some((status >> 8) & 0xff),
        signal: (signal != 0).then_some(signal),
        stdout,
        stderr,
    }
}

fn termination(action: impl FnOnce()) -> (Option<i32>, Option<i32>) {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let mut stdin_pipe = [0; 2];
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(pipe(stdin_pipe.as_mut_ptr()), 0);
        close(stdin_pipe[1]);
    }
    let process_id = unsafe { fork() };
    assert!(process_id >= 0);
    if process_id == 0 {
        unsafe {
            dup2(stdin_pipe[0], 0);
            close(stdin_pipe[0]);
        }
        action();
        unsafe { _exit(0) };
    }
    unsafe {
        close(stdin_pipe[0]);
    }
    let mut status = 0;
    assert_eq!(unsafe { waitpid(process_id, &mut status, 0) }, process_id);
    let signal = status & 0x7f;
    (
        (signal == 0).then_some((status >> 8) & 0xff),
        (signal != 0).then_some(signal),
    )
}

fn compare_termination(
    row: &str,
    c_action: impl FnOnce(),
    rust_action: impl FnOnce(),
) -> (Option<i32>, Option<i32>) {
    let c_result = termination(c_action);
    let rust_result = termination(rust_action);
    assert_eq!(rust_result, c_result, "{row}");
    c_result
}

fn compare_main(c: &Api, rust: &Api, arguments: &[&str], input: &[u8], row: &str) -> MainOutcome {
    let c_outcome = run_exported_main(c, arguments, input);
    let rust_outcome = run_exported_main(rust, arguments, input);
    assert_eq!(rust_outcome, c_outcome, "{row}");
    c_outcome
}

fn record(timestamp: &str, spec: &DirectiveSpec) -> String {
    format!(
        "{timestamp} {} {} {} {}{}\n",
        spec.luggage_id, spec.flight_id, spec.departure, spec.arrival, spec.comments
    )
}

fn exact_filters(spec: &DirectiveSpec) -> [&str; 4] {
    [
        spec.luggage_id.as_str(),
        spec.flight_id.as_str(),
        spec.departure.as_str(),
        spec.arrival.as_str(),
    ]
}

#[test]
fn main_configurations_and_errors_match() {
    let (c, rust) = apis();
    let wildcard = ["-", "-", "-", "-"];

    let empty = compare_main(&c, &rust, &wildcard, b"", "C43/E03");
    assert_eq!(empty.exit_code, Some(0));
    assert!(empty.stdout.is_empty());

    for argument_count in [0, 1, 2, 3, 5, 7] {
        let arguments = vec!["x"; argument_count];
        let outcome = compare_main(&c, &rust, &arguments, b"", "E02");
        assert_eq!(outcome.exit_code, Some(1));
        assert_eq!(
            outcome.stderr,
            b"Command line error: 4 arguments expected\n"
        );
    }

    let mut random = Random::new();
    for iteration in 0..8 {
        let timestamp = random.next_u32() & 0x7fff_ffff;
        let spec = random_spec(&mut random, timestamp);
        let input = record(&timestamp.to_string(), &spec);
        let filters = exact_filters(&spec);
        let outcome = compare_main(
            &c,
            &rust,
            &filters,
            input.as_bytes(),
            &format!("C19, iteration {iteration}"),
        );
        assert!(!outcome.stdout.is_empty());

        for mask in 0_u8..16 {
            let filters = [
                if mask & 1 == 0 {
                    spec.luggage_id.as_str()
                } else {
                    "-"
                },
                if mask & 2 == 0 {
                    spec.flight_id.as_str()
                } else {
                    "-"
                },
                if mask & 4 == 0 {
                    spec.departure.as_str()
                } else {
                    "-"
                },
                if mask & 8 == 0 {
                    spec.arrival.as_str()
                } else {
                    "-"
                },
            ];
            let row = format!("{}/main, iteration {iteration}", FILTER_ROWS[mask as usize]);
            let outcome = compare_main(&c, &rust, &filters, input.as_bytes(), &row);
            assert!(!outcome.stdout.is_empty(), "{row}");
        }
    }

    for iteration in 0..8 {
        let timestamp = random.next_u32() & 0x7fff_ffff;
        let mut no_comment = random_spec(&mut random, timestamp);
        no_comment.comments.clear();
        compare_main(
            &c,
            &rust,
            &wildcard,
            record(&no_comment.time_stamp.to_string(), &no_comment).as_bytes(),
            &format!("C44, iteration {iteration}"),
        );

        let timestamp = random.next_u32() & 0x7fff_ffff;
        let mut one_comment = random_spec(&mut random, timestamp);
        one_comment.comments = "x".to_owned();
        compare_main(
            &c,
            &rust,
            &wildcard,
            record(&one_comment.time_stamp.to_string(), &one_comment).as_bytes(),
            &format!("C45, iteration {iteration}"),
        );

        let timestamp = random.next_u32() & 0x7fff_ffff;
        let mut max_comment = random_spec(&mut random, timestamp);
        max_comment.comments = random.text(80, b"abcdefghijklmnopqrstuvwxyz ");
        compare_main(
            &c,
            &rust,
            &wildcard,
            record(&max_comment.time_stamp.to_string(), &max_comment).as_bytes(),
            &format!("C46, iteration {iteration}"),
        );

        let timestamp = random.next_u32() & 0x7fff_ffff;
        let max_fields = random_spec(&mut random, timestamp);
        compare_main(
            &c,
            &rust,
            &wildcard,
            record(&max_fields.time_stamp.to_string(), &max_fields).as_bytes(),
            &format!("C47, iteration {iteration}"),
        );

        let short_fields = DirectiveSpec {
            time_stamp: random.next_u32() & 0x7fff_ffff,
            luggage_id: "A".to_owned(),
            flight_id: "B".to_owned(),
            departure: "C".to_owned(),
            arrival: "D".to_owned(),
            comments: "q".to_owned(),
        };
        compare_main(
            &c,
            &rust,
            &wildcard,
            record(&short_fields.time_stamp.to_string(), &short_fields).as_bytes(),
            &format!("C48, iteration {iteration}"),
        );
    }

    for iteration in 0..8 {
        let extra = (b'A' + iteration) as char;
        let input = format!("1 ABCDEFGH{extra} FLIGHT AAA BBB\n");
        compare_main(
            &c,
            &rust,
            &wildcard,
            input.as_bytes(),
            &format!("C49, iteration {iteration}"),
        );
    }

    for (input, row) in [
        ("1 BAG00001 FLIGHTX AAA BBB\n".to_owned(), "C56"),
        ("1 BAG00001 FL0001 AAAZ BBB\n".to_owned(), "C57"),
        ("1 BAG00001 FL0001 AAA BBBZ\n".to_owned(), "C58"),
        (
            format!(
                "1 BAG00001 FL0001 AAA BBB{}2 BAG00002 FL0002 CCC DDD\n",
                "x".repeat(80)
            ),
            "C59",
        ),
        ("2147483648 BAG00001 FL0001 AAA BBB\n".to_owned(), "C60"),
    ] {
        compare_main(&c, &rust, &wildcard, input.as_bytes(), row);
    }

    for iteration in 0..8 {
        let timestamp = random.next_u32() & 0x7fff_ffff;
        let mut first = random_spec(&mut random, timestamp);
        first.comments = format!("first{iteration}");
        let mut second = random_spec(&mut random, timestamp);
        second.comments = format!("second{iteration}");
        let input = format!(
            "{}{}",
            record(&timestamp.to_string(), &first),
            record(&timestamp.to_string(), &second)
        );
        let outcome = compare_main(&c, &rust, &wildcard, input.as_bytes(), "C50");
        let first_position = find_bytes(&outcome.stdout, first.comments.as_bytes()).unwrap();
        let second_position = find_bytes(&outcome.stdout, second.comments.as_bytes()).unwrap();
        assert!(
            first_position < second_position,
            "C50, iteration {iteration}"
        );
    }

    let boundary_spec = DirectiveSpec {
        time_stamp: 0,
        luggage_id: "BAG00001".to_owned(),
        flight_id: "FL0001".to_owned(),
        departure: "AAA".to_owned(),
        arrival: "BBB".to_owned(),
        comments: "boundary".to_owned(),
    };
    for (timestamp, row) in [("0", "C51"), ("2147483647", "C52"), ("-1", "C53")] {
        compare_main(
            &c,
            &rust,
            &wildcard,
            record(timestamp, &boundary_spec).as_bytes(),
            row,
        );
    }

    for iteration in 0..8 {
        let input = format!(
            "{}\t{}\n{}\r{}\u{b}{} note{}\n",
            iteration + 1,
            "BAG00001",
            "FL0001",
            "AAA",
            "BBB",
            iteration
        );
        compare_main(
            &c,
            &rust,
            &wildcard,
            input.as_bytes(),
            &format!("C54, iteration {iteration}"),
        );
    }

    for iteration in 0..8 {
        let mut specs: Vec<_> = (0..10)
            .map(|index| {
                let timestamp = random.next_u32() & 0x7fff_ffff;
                let mut spec = random_spec(&mut random, timestamp);
                if index % 2 == 0 {
                    spec.comments.clear();
                }
                spec
            })
            .collect();
        let input: String = specs
            .iter()
            .map(|spec| record(&spec.time_stamp.to_string(), spec))
            .collect();
        compare_main(
            &c,
            &rust,
            &wildcard,
            input.as_bytes(),
            &format!("C42/C55, iteration {iteration}"),
        );
        specs.clear();
    }

    let base = DirectiveSpec {
        time_stamp: 1,
        luggage_id: "BAG00001".to_owned(),
        flight_id: "FL0001".to_owned(),
        departure: "AAA".to_owned(),
        arrival: "BBB".to_owned(),
        comments: "complete".to_owned(),
    };
    let complete = record("1", &base);
    for (suffix, row) in [
        ("7 ", "E04"),
        ("7 BAG FL1 ", "E05"),
        ("7 BAG FL1 AAA BBB", "E06"),
    ] {
        let input = format!("{complete}{suffix}");
        let outcome = compare_main(&c, &rust, &wildcard, input.as_bytes(), row);
        assert!(find_bytes(&outcome.stdout, b"complete").is_some(), "{row}");
    }

    for iteration in 0..8 {
        let mut later_same = base.clone();
        later_same.time_stamp = 2;
        later_same.comments = format!("later-same-{iteration}");
        let same_input = format!("{}{}", record("1", &base), record("2", &later_same));
        compare_main(&c, &rust, &wildcard, same_input.as_bytes(), "C40/main");

        let mut later_different = later_same;
        later_different.departure = "CCC".to_owned();
        let different_input = format!("{}{}", record("1", &base), record("2", &later_different));
        compare_main(&c, &rust, &wildcard, different_input.as_bytes(), "C41/main");
    }
}

#[test]
fn generic_null_pointer_boundaries_match() {
    let (c, rust) = apis();
    let spec = DirectiveSpec {
        time_stamp: 1,
        luggage_id: "BAG00001".to_owned(),
        flight_id: "FL0001".to_owned(),
        departure: "AAA".to_owned(),
        arrival: "BBB".to_owned(),
        comments: "comment".to_owned(),
    };

    let mut c_node = node_from_spec(&spec);
    let mut rust_node = node_from_spec(&spec);
    assert_eq!(
        compare_termination(
            "G01",
            || unsafe { (c.add)(ptr::null_mut(), &mut *c_node) },
            || unsafe { (rust.add)(ptr::null_mut(), &mut *rust_node) },
        ),
        (None, Some(11))
    );

    let mut c_head = node_from_spec(&spec);
    let mut rust_head = node_from_spec(&spec);
    assert_eq!(
        compare_termination(
            "G02",
            || unsafe { (c.add)(&mut *c_head, ptr::null_mut()) },
            || unsafe { (rust.add)(&mut *rust_head, ptr::null_mut()) },
        ),
        (None, Some(11))
    );

    let mut c_node = node_from_spec(&spec);
    let mut rust_node = node_from_spec(&spec);
    let mut c_departure = c_string("AAA");
    let mut rust_departure = c_string("AAA");
    assert_eq!(
        compare_termination(
            "G03",
            || unsafe {
                (c.supersedes)(&mut *c_node, ptr::null_mut(), c_departure.as_mut_ptr());
            },
            || unsafe {
                (rust.supersedes)(
                    &mut *rust_node,
                    ptr::null_mut(),
                    rust_departure.as_mut_ptr(),
                );
            },
        ),
        (None, Some(11))
    );

    let mut c_node = node_from_spec(&spec);
    let mut rust_node = node_from_spec(&spec);
    let mut c_luggage = c_string("BAG00001");
    let mut rust_luggage = c_string("BAG00001");
    assert_eq!(
        compare_termination(
            "G04",
            || unsafe {
                (c.supersedes)(&mut *c_node, c_luggage.as_mut_ptr(), ptr::null_mut());
            },
            || unsafe {
                (rust.supersedes)(&mut *rust_node, rust_luggage.as_mut_ptr(), ptr::null_mut());
            },
        ),
        (None, Some(11))
    );

    assert_eq!(
        compare_termination(
            "G05",
            || unsafe {
                (c.superseded)(ptr::null_mut());
            },
            || unsafe {
                (rust.superseded)(ptr::null_mut());
            },
        ),
        (None, Some(11))
    );

    let mut c_actual = c_string("value");
    let mut rust_actual = c_string("value");
    assert_eq!(
        compare_termination(
            "G06",
            || unsafe {
                (c.matches)(ptr::null_mut(), c_actual.as_mut_ptr());
            },
            || unsafe {
                (rust.matches)(ptr::null_mut(), rust_actual.as_mut_ptr());
            },
        ),
        (None, Some(11))
    );

    let mut c_expected = c_string("value");
    let mut rust_expected = c_string("value");
    assert_eq!(
        compare_termination(
            "G07",
            || unsafe {
                (c.matches)(c_expected.as_mut_ptr(), ptr::null_mut());
            },
            || unsafe {
                (rust.matches)(rust_expected.as_mut_ptr(), ptr::null_mut());
            },
        ),
        (None, Some(11))
    );

    let c_list = List::from_specs(std::slice::from_ref(&spec));
    let rust_list = List::from_specs(std::slice::from_ref(&spec));
    let mut c_wildcards = [c_string("-"), c_string("-"), c_string("-")];
    let mut rust_wildcards = [c_string("-"), c_string("-"), c_string("-")];
    assert_eq!(
        compare_termination(
            "G08",
            || unsafe {
                (c.print)(
                    c_list.head,
                    ptr::null_mut(),
                    c_wildcards[0].as_mut_ptr(),
                    c_wildcards[1].as_mut_ptr(),
                    c_wildcards[2].as_mut_ptr(),
                );
            },
            || unsafe {
                (rust.print)(
                    rust_list.head,
                    ptr::null_mut(),
                    rust_wildcards[0].as_mut_ptr(),
                    rust_wildcards[1].as_mut_ptr(),
                    rust_wildcards[2].as_mut_ptr(),
                );
            },
        ),
        (None, Some(11))
    );

    assert_eq!(
        compare_termination(
            "G09",
            || unsafe {
                (c.main)(5, ptr::null_mut());
            },
            || unsafe {
                (rust.main)(5, ptr::null_mut());
            },
        ),
        (None, Some(11))
    );

    let program = CString::new("driver").unwrap();
    let mut c_arguments = [
        program.as_ptr().cast_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
    ];
    let mut rust_arguments = c_arguments;
    assert_eq!(
        compare_termination(
            "G10",
            || unsafe {
                (c.main)(5, c_arguments.as_mut_ptr());
            },
            || unsafe {
                (rust.main)(5, rust_arguments.as_mut_ptr());
            },
        ),
        (Some(0), None)
    );
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn defined_symbols(path: &Path) -> Vec<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut symbols: Vec<_> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect();
    symbols.sort();
    symbols
}

#[test]
fn dynamic_symbol_surfaces_match() {
    assert_eq!(
        defined_symbols(&rust_library_path()),
        defined_symbols(&c_library_path())
    );
}

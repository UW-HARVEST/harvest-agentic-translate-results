use std::io::{self, Read};

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

fn add_floor(house: &mut House) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn print_house(house: &House) {
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(the_house: &mut House, extra_bedrooms: i32) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    the_house.bathrooms += 1.0;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}

fn fgets_stdin_bytes(limit: usize) -> io::Result<Vec<u8>> {
    let mut stdin = io::stdin().lock();
    let mut buf = Vec::with_capacity(limit + 1);
    let mut byte = [0_u8; 1];

    while buf.len() < limit {
        match stdin.read(&mut byte)? {
            0 => break,
            _ => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
        }
    }

    Ok(buf)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn errno_location() -> *mut libc::c_int {
    libc::__errno_location()
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
unsafe fn errno_location() -> *mut libc::c_int {
    libc::__error()
}

fn parse_val(buf: &[u8]) -> Option<i32> {
    let mut c_buf = Vec::with_capacity(buf.len() + 1);
    c_buf.extend_from_slice(buf);
    c_buf.push(0);

    let start = c_buf.as_mut_ptr() as *mut libc::c_char;
    let mut endp = start;

    let tmp = unsafe {
        *errno_location() = 0;
        libc::strtol(start, &mut endp, 10)
    };

    let errno_value = unsafe { *errno_location() };
    if endp != start
        && errno_value == 0
        && tmp >= i32::MIN as libc::c_long
        && tmp <= i32::MAX as libc::c_long
    {
        Some(tmp as i32)
    } else {
        None
    }
}

fn main() {
    let input = fgets_stdin_bytes(99).unwrap_or_default();

    if let Some(x) = parse_val(&input) {
        let mut the_house = House {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house, x);
        run(&mut the_house, x);
    } else {
        println!("An error occurred");
    }
}

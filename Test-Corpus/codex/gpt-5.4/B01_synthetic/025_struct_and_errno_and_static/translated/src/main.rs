use std::io::{self, Read};
use std::sync::Mutex;

#[derive(Clone, Copy)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

static THE_HOUSE: Mutex<House> = Mutex::new(House {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
});

fn add_floor(house: &mut House) {
    house.floors = house.floors.wrapping_add(1);
}

fn add_bedrooms(house: &mut House, extra_bedrooms: i32) {
    house.bedrooms = house.bedrooms.wrapping_add(extra_bedrooms);
}

fn add_floor_to_the_house() {
    let mut house = THE_HOUSE.lock().unwrap_or_else(|err| err.into_inner());
    add_floor(&mut house);
}

fn print_the_house() {
    let house = *THE_HOUSE.lock().unwrap_or_else(|err| err.into_inner());
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        house.floors, house.bedrooms, house.bathrooms
    );
}

fn run(extra_bedrooms: i32) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    THE_HOUSE
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .bathrooms += 1.0;
    print_the_house();
    let mut house = THE_HOUSE.lock().unwrap_or_else(|err| err.into_inner());
    add_bedrooms(&mut house, extra_bedrooms);
    print_the_house();
}

fn parse_val(buf: &[u8]) -> Option<i32> {
    unsafe {
        libc::__errno_location().write(0);
        let mut endp: *mut libc::c_char = buf.as_ptr() as *mut libc::c_char;
        let tmp = libc::strtol(buf.as_ptr() as *const libc::c_char, &mut endp, 10);
        let errno = *libc::__errno_location();
        if endp != buf.as_ptr() as *mut libc::c_char
            && errno == 0
            && tmp >= i32::MIN as libc::c_long
            && tmp <= i32::MAX as libc::c_long
        {
            Some(tmp as i32)
        } else {
            None
        }
    }
}

fn read_fgets_stdin() -> Vec<u8> {
    let mut buf = vec![0u8; 100];
    let mut stdin = io::stdin().lock();
    let mut i = 0usize;

    while i < 99 {
        let mut byte = [0u8; 1];
        match stdin.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf[i] = byte[0];
                i += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if i < buf.len() {
        buf[i] = 0;
    } else {
        buf.push(0);
    }

    buf
}

fn main() {
    let input = read_fgets_stdin();

    if let Some(x) = parse_val(&input) {
        run(x);
        run(x);
    } else {
        println!("An error occurred");
    }
}

#[repr(C)]
pub struct house_t {
    pub floors: i32,
    pub bedrooms: i32,
    pub bathrooms: f64,
}

fn print_house(house: &house_t) {
    print!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
        house.floors, house.bedrooms, house.bathrooms
    );
}

#[no_mangle]
pub extern "C" fn run(house: *mut house_t, extra_bedrooms: i32) {
    let house = unsafe { &mut *house };
    print_house(house);
    house.floors += 1;
    print_house(house);
    house.bathrooms += 1.0;
    print_house(house);
    house.bedrooms += extra_bedrooms;
    print_house(house);
}

fn parse_val(s: &str) -> Option<i32> {
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let (rest, negative) = if trimmed.starts_with('-') {
        (&trimmed[1..], true)
    } else if trimmed.starts_with('+') {
        (&trimmed[1..], false)
    } else {
        (trimmed, false)
    };
    let digit_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    if digit_end == 0 {
        return None;
    }
    let digits = &rest[..digit_end];
    let val: i64 = digits.parse().ok()?;
    let val = if negative { -val } else { val };
    if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
        Some(val as i32)
    } else {
        None
    }
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        print!("An error occurred\n");
        return 0;
    }
    if let Some(x) = parse_val(&input) {
        let mut the_house = house_t {
            floors: 2,
            bedrooms: 5,
            bathrooms: 2.5,
        };
        run(&mut the_house as *mut house_t, x);
        run(&mut the_house as *mut house_t, x);
    } else {
        print!("An error occurred\n");
    }
    0
}

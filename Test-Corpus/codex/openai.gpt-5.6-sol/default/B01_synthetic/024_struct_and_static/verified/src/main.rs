use std::ffi::{c_char, c_int};
use std::io::{self, Write};

struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

impl House {
    fn add_floor(&mut self) {
        self.floors = self.floors.wrapping_add(1);
    }

    fn add_bedrooms(&mut self, extra_bedrooms: i32) {
        self.bedrooms = self.bedrooms.wrapping_add(extra_bedrooms);
    }

    fn print(&self, output: &mut impl Write) {
        let _ = writeln!(
            output,
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
            self.floors, self.bedrooms, self.bathrooms
        );
    }
}

fn run(house: &mut House, extra_bedrooms: i32, output: &mut impl Write) {
    house.print(output);
    house.add_floor();
    house.print(output);
    house.bathrooms += 1.0;
    house.print(output);
    house.add_bedrooms(extra_bedrooms);
    house.print(output);
}

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn read_extra_bedrooms() -> i32 {
    static DECIMAL_FORMAT: &[u8] = b"%d\0";
    let mut value = 0_i32;

    // Calling the platform scanner preserves the source program's %d semantics.
    unsafe {
        scanf(
            DECIMAL_FORMAT.as_ptr().cast::<c_char>(),
            &mut value as *mut i32,
        );
    }

    value
}

fn main() {
    let extra_bedrooms = read_extra_bedrooms();
    let mut house = House {
        floors: 2,
        bedrooms: 5,
        bathrooms: 2.5,
    };
    let stdout = io::stdout();
    let mut output = stdout.lock();

    run(&mut house, extra_bedrooms, &mut output);
    run(&mut house, extra_bedrooms, &mut output);
}

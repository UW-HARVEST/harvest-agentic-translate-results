#![cfg_attr(fuzzing, no_main)]
#![allow(non_camel_case_types)]

use cando2::*;

#[repr(C)]
#[derive(Arbitrary, Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct record {
    pub precision: CString,
    pub lat: c_double,
    pub lon: c_double,
    pub address: CString,
    pub city: CString,
    pub state: CString,
    pub zip: CString,
    pub country: CString,
}

// To pass the record struct over FFI
#[repr(C)]
#[derive(Debug)]
struct ffi_record {
    precision: *const c_char,
    lat: c_double,
    lon: c_double,
    address: *const c_char,
    city: *const c_char,
    state: *const c_char,
    zip: *const c_char,
    country: *const c_char,
}

harness! {
    state: {
        strings: [CString; 7],
        numbers: [[c_int; 3]; 3],
        ids: [c_int; 4],
        fields: [record; 2],
        returns: c_int,
    },

    library: "cJSON_test",
    symbol: "driver",

    signature: unsafe extern "C" fn(*const *const c_char, *const [c_int; 3], *const c_int, *const ffi_record) -> c_int,

    fn run(&mut self) {
        let c_strings: [*const c_char; 7] = [
            self.strings[0].as_ptr(),
            self.strings[1].as_ptr(),
            self.strings[2].as_ptr(),
            self.strings[3].as_ptr(),
            self.strings[4].as_ptr(),
            self.strings[5].as_ptr(),
            self.strings[6].as_ptr(),
        ];
        let ffi_fields: [ffi_record; 2] = self.fields.iter().map(|f| {
            ffi_record {
                precision: f.precision.as_ptr(),
                lat: f.lat,
                lon: f.lon,
                address: f.address.as_ptr(),
                city: f.city.as_ptr(),
                state: f.state.as_ptr(),
                zip: f.zip.as_ptr(),
                country: f.country.as_ptr(),
            }
        }).collect::<Vec<_>>()
        .try_into()
        .expect("Rust runner error: Expected exactly 2 records");

        self.returns = unsafe {
            (*SYMBOL)(
                c_strings.as_ptr(),
                self.numbers.as_ptr(),
                self.ids.as_ptr(),
                ffi_fields.as_ptr(),
            )
        }
    }
}

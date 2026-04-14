use cjson::*;
use std::ffi::{CStr, CString};

#[repr(C)]
struct record {
    precision: *const libc::c_char,
    lat: f64,
    lon: f64,
    address: *const libc::c_char,
    city: *const libc::c_char,
    state: *const libc::c_char,
    zip: *const libc::c_char,
    country: *const libc::c_char,
}

fn print_preallocated(root: *mut cJSON) -> i32 {
    unsafe {
        let out = cJSON_Print(root);
        if out.is_null() {
            return -1;
        }
        let len = CStr::from_ptr(out).to_bytes().len() + 5;
        let mut buf = vec![0i8; len];
        let mut buf_fail = vec![0i8; len.saturating_sub(5).max(1)];

        if cJSON_PrintPreallocated(root, buf.as_mut_ptr(), len as i32, 1) == 0 {
            cJSON_free(out as *mut _);
            return -1;
        }

        println!("{}", CStr::from_ptr(buf.as_ptr()).to_string_lossy());

        if cJSON_PrintPreallocated(root, buf_fail.as_mut_ptr(), buf_fail.len() as i32, 1) != 0 {
            cJSON_free(out as *mut _);
            return -1;
        }

        cJSON_free(out as *mut _);
        0
    }
}

fn create_objects(strings: [*const libc::c_char; 7], numbers: [[i32; 3]; 3], ids: [i32; 4], fields: [record; 2]) {
    unsafe {
        let mut root;
        let mut fmt;
        let mut img;
        let mut thm;
        let mut fld;

        let name = CString::new("name").unwrap();
        let jack = CString::new("Jack (\"Bee\") Nimble").unwrap();
        let format = CString::new("format").unwrap();
        let typ = CString::new("type").unwrap();
        let rect = CString::new("rect").unwrap();
        let width = CString::new("width").unwrap();
        let height = CString::new("height").unwrap();
        let interlace = CString::new("interlace").unwrap();
        let frame_rate = CString::new("frame rate").unwrap();

        root = cJSON_CreateObject();
        cJSON_AddItemToObject(root, name.as_ptr(), cJSON_CreateString(jack.as_ptr()));
        fmt = cJSON_CreateObject();
        cJSON_AddItemToObject(root, format.as_ptr(), fmt);
        cJSON_AddStringToObject(fmt, typ.as_ptr(), rect.as_ptr());
        cJSON_AddNumberToObject(fmt, width.as_ptr(), 1920.0);
        cJSON_AddNumberToObject(fmt, height.as_ptr(), 1080.0);
        cJSON_AddFalseToObject(fmt, interlace.as_ptr());
        cJSON_AddNumberToObject(fmt, frame_rate.as_ptr(), 24.0);
        let _ = print_preallocated(root);
        cJSON_Delete(root);

        root = cJSON_CreateStringArray(strings.as_ptr(), 7);
        let _ = print_preallocated(root);
        cJSON_Delete(root);

        root = cJSON_CreateArray();
        for row in &numbers {
            cJSON_AddItemToArray(root, cJSON_CreateIntArray(row.as_ptr(), 3));
        }
        let _ = print_preallocated(root);
        cJSON_Delete(root);

        let image = CString::new("Image").unwrap();
        let title = CString::new("Title").unwrap();
        let view = CString::new("View from 15th Floor").unwrap();
        let thumbnail = CString::new("Thumbnail").unwrap();
        let url = CString::new("Url").unwrap();
        let urlv = CString::new("http:/*www.example.com/image/481989943").unwrap();
        let ids_key = CString::new("IDs").unwrap();
        let width_str = CString::new("100").unwrap();
        let width_cap = CString::new("Width").unwrap();
        let height_cap = CString::new("Height").unwrap();

        root = cJSON_CreateObject();
        img = cJSON_CreateObject();
        cJSON_AddItemToObject(root, image.as_ptr(), img);
        cJSON_AddNumberToObject(img, width_cap.as_ptr(), 800.0);
        cJSON_AddNumberToObject(img, height_cap.as_ptr(), 600.0);
        cJSON_AddStringToObject(img, title.as_ptr(), view.as_ptr());
        thm = cJSON_CreateObject();
        cJSON_AddItemToObject(img, thumbnail.as_ptr(), thm);
        cJSON_AddStringToObject(thm, url.as_ptr(), urlv.as_ptr());
        cJSON_AddNumberToObject(thm, height_cap.as_ptr(), 125.0);
        cJSON_AddStringToObject(thm, width_cap.as_ptr(), width_str.as_ptr());
        cJSON_AddItemToObject(img, ids_key.as_ptr(), cJSON_CreateIntArray(ids.as_ptr(), 4));
        let _ = print_preallocated(root);
        cJSON_Delete(root);

        let precision = CString::new("precision").unwrap();
        let latitude = CString::new("Latitude").unwrap();
        let longitude = CString::new("Longitude").unwrap();
        let address = CString::new("Address").unwrap();
        let city = CString::new("City").unwrap();
        let state = CString::new("State").unwrap();
        let zip = CString::new("Zip").unwrap();
        let country = CString::new("Country").unwrap();

        root = cJSON_CreateArray();
        for f in &fields {
            fld = cJSON_CreateObject();
            cJSON_AddItemToArray(root, fld);
            cJSON_AddStringToObject(fld, precision.as_ptr(), f.precision);
            cJSON_AddNumberToObject(fld, latitude.as_ptr(), f.lat);
            cJSON_AddNumberToObject(fld, longitude.as_ptr(), f.lon);
            cJSON_AddStringToObject(fld, address.as_ptr(), f.address);
            cJSON_AddStringToObject(fld, city.as_ptr(), f.city);
            cJSON_AddStringToObject(fld, state.as_ptr(), f.state);
            cJSON_AddStringToObject(fld, zip.as_ptr(), f.zip);
            cJSON_AddStringToObject(fld, country.as_ptr(), f.country);
        }
        let _ = print_preallocated(root);
        cJSON_Delete(root);

        let number = CString::new("number").unwrap();
        root = cJSON_CreateObject();
        cJSON_AddNumberToObject(root, number.as_ptr(), f64::INFINITY);
        let _ = print_preallocated(root);
        cJSON_Delete(root);
    }
}

fn main() {
    unsafe {
        println!("Version: {}", CStr::from_ptr(cJSON_Version()).to_string_lossy());
    }

    let strings_storage = [
        CString::new("Sunday").unwrap(),
        CString::new("Monday").unwrap(),
        CString::new("Tuesday").unwrap(),
        CString::new("Wednesday").unwrap(),
        CString::new("Thursday").unwrap(),
        CString::new("Friday").unwrap(),
        CString::new("Saturday").unwrap(),
    ];
    let strings = [
        strings_storage[0].as_ptr(),
        strings_storage[1].as_ptr(),
        strings_storage[2].as_ptr(),
        strings_storage[3].as_ptr(),
        strings_storage[4].as_ptr(),
        strings_storage[5].as_ptr(),
        strings_storage[6].as_ptr(),
    ];

    let numbers = [[0, -1, 0], [1, 0, 0], [0, 0, 1]];
    let ids = [116, 943, 234, 38793];

    let rec_storage = [
        (
            CString::new("zip").unwrap(),
            CString::new("").unwrap(),
            CString::new("SAN FRANCISCO").unwrap(),
            CString::new("CA").unwrap(),
            CString::new("94107").unwrap(),
            CString::new("US").unwrap(),
        ),
        (
            CString::new("zip").unwrap(),
            CString::new("").unwrap(),
            CString::new("SUNNYVALE").unwrap(),
            CString::new("CA").unwrap(),
            CString::new("94085").unwrap(),
            CString::new("US").unwrap(),
        ),
    ];

    let fields = [
        record {
            precision: rec_storage[0].0.as_ptr(),
            lat: 37.7668,
            lon: -122.3959,
            address: rec_storage[0].1.as_ptr(),
            city: rec_storage[0].2.as_ptr(),
            state: rec_storage[0].3.as_ptr(),
            zip: rec_storage[0].4.as_ptr(),
            country: rec_storage[0].5.as_ptr(),
        },
        record {
            precision: rec_storage[1].0.as_ptr(),
            lat: 37.371991,
            lon: -122.02602,
            address: rec_storage[1].1.as_ptr(),
            city: rec_storage[1].2.as_ptr(),
            state: rec_storage[1].3.as_ptr(),
            zip: rec_storage[1].4.as_ptr(),
            country: rec_storage[1].5.as_ptr(),
        },
    ];

    create_objects(strings, numbers, ids, fields);
}

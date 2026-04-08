use Simple_Sparsehash::simple_sparsehash::*;

macro_rules! assert_test {
    ($x:expr) => {
        if !($x) {
            println!("Assertion failed at line {}: {}", line!(), stringify!($x));
            return false;
        }
    };
}

macro_rules! run_test {
    ($test:ident, $passed:ident, $failed:ident) => {
        if $test() {
            $passed += 1;
            println!("\x1B[32mPassed\x1B[0m: {}", stringify!($test));
        } else {
            $failed += 1;
            println!("\x1B[31mFailed\x1B[0m: {}", stringify!($test));
        }
    };
}

fn test_empty_array_does_not_blow_up() -> bool {
    let arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    assert_test!(sparse_array_get(&arr, 0, None).is_none());
    assert_test!(sparse_array_free(arr) != 0);
    true
}

fn test_cannot_set_outside_bounds() -> bool {
    let mut arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    let test_num: u64 = 666;
    assert_test!(sparse_array_set(&mut arr, 35, &test_num.to_ne_bytes(), std::mem::size_of::<u64>()) == 0);
    assert_test!(sparse_array_free(arr) != 0);
    true
}

fn test_cannot_get_outside_bounds() -> bool {
    let arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    assert_test!(sparse_array_get(&arr, 35, None).is_none());
    assert_test!(sparse_array_free(arr) != 0);
    true
}

fn test_cannot_set_bigger_elements() -> bool {
    let mut arr = sparse_array_init(std::mem::size_of::<u8>(), 100).unwrap();
    let test_num: u64 = 666;
    assert_test!(sparse_array_set(&mut arr, 0, &test_num.to_ne_bytes(), std::mem::size_of::<u64>()) == 0);
    assert_test!(sparse_array_free(arr) != 0);
    true
}

fn test_array_set_backwards() -> bool {
    let array_size: i32 = 120;
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), array_size as u32).unwrap();

    for i in (0..array_size).rev() {
        let mut siz = 0usize;
        assert_test!(sparse_array_set(&mut arr, i as u32, &i.to_ne_bytes(), std::mem::size_of::<i32>()) != 0);
        let returned = sparse_array_get(&arr, i as u32, Some(&mut siz));
        assert_test!(returned.is_some());
        let val = i32::from_ne_bytes(returned.unwrap()[..4].try_into().unwrap());
        assert_test!(val == i);
        assert_test!(siz == std::mem::size_of::<i32>());
    }

    for i in (0..array_size).rev() {
        let mut siz = 0usize;
        let returned = sparse_array_get(&arr, i as u32, Some(&mut siz));
        let val = i32::from_ne_bytes(returned.unwrap()[..4].try_into().unwrap());
        assert_test!(val == i);
        assert_test!(siz == std::mem::size_of::<i32>());
    }

    assert_test!(sparse_array_free(arr) != 0);
    true
}

fn test_array_set() -> bool {
    let array_size: i32 = 130;
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), array_size as u32).unwrap();

    for i in 0..array_size {
        let mut siz = 0usize;
        assert_test!(sparse_array_set(&mut arr, i as u32, &i.to_ne_bytes(), std::mem::size_of::<i32>()) != 0);
        let returned = sparse_array_get(&arr, i as u32, Some(&mut siz));
        assert_test!(returned.is_some());
        let val = i32::from_ne_bytes(returned.unwrap()[..4].try_into().unwrap());
        assert_test!(val == i);
        assert_test!(siz == std::mem::size_of::<i32>());
    }

    for i in 0..array_size {
        let mut siz = 0usize;
        let returned = sparse_array_get(&arr, i as u32, Some(&mut siz));
        let val = i32::from_ne_bytes(returned.unwrap()[..4].try_into().unwrap());
        assert_test!(val == i);
        assert_test!(siz == std::mem::size_of::<i32>());
    }

    assert_test!(sparse_array_free(arr) != 0);
    true
}

fn test_array_set_high_num() -> bool {
    let test_num: i32 = 65555555;
    let index = (GROUP_SIZE - 1) as u32;
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 140).unwrap();

    let mut siz = 0usize;
    assert_test!(sparse_array_set(&mut arr, index, &test_num.to_ne_bytes(), std::mem::size_of::<i32>()) != 0);
    let returned = sparse_array_get(&arr, index, Some(&mut siz));
    assert_test!(returned.is_some());
    let val = i32::from_ne_bytes(returned.unwrap()[..4].try_into().unwrap());
    assert_test!(val == test_num);
    assert_test!(siz == std::mem::size_of::<i32>());

    assert_test!(sparse_array_free(arr) != 0);
    true
}

fn test_array_set_overwrites_old_values() -> bool {
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 150).unwrap();
    let test_num: i32 = 666;
    let test_num2: i32 = 1024;

    assert_test!(sparse_array_set(&mut arr, 0, &test_num.to_ne_bytes(), std::mem::size_of::<i32>()) != 0);
    assert_test!(sparse_array_set(&mut arr, 0, &test_num2.to_ne_bytes(), std::mem::size_of::<i32>()) != 0);

    let returned = sparse_array_get(&arr, 0, None).unwrap();
    let val = i32::from_ne_bytes(returned[..4].try_into().unwrap());
    assert_test!(val == 1024);

    assert_test!(sparse_array_free(arr) != 0);
    true
}

fn test_array_get() -> bool {
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 200).unwrap();
    let test_num: i32 = 666;
    let mut item_size = 0usize;

    assert_test!(sparse_array_set(&mut arr, 0, &test_num.to_ne_bytes(), std::mem::size_of::<i32>()) != 0);
    let returned = sparse_array_get(&arr, 0, Some(&mut item_size)).unwrap();
    let val = i32::from_ne_bytes(returned[..4].try_into().unwrap());
    assert_test!(val == 666);
    assert_test!(item_size == std::mem::size_of::<i32>());

    assert_test!(sparse_array_free(arr) != 0);
    true
}

fn test_dict_set() -> bool {
    let mut dict = sparse_dict_init().unwrap();
    assert_test!(sparse_dict_set(&mut dict, "key", "key".len(), b"value", "value".len()) != 0);
    assert_test!(sparse_dict_free(dict) != 0);
    true
}

fn test_dict_get() -> bool {
    let mut dict = sparse_dict_init().unwrap();
    let mut outsize = 0usize;

    assert_test!(sparse_dict_set(&mut dict, "key", "key".len(), b"value", "value".len()) != 0);

    let value = sparse_dict_get(&dict, "key", "key".len(), Some(&mut outsize));
    assert_test!(value.is_some());
    let value = value.unwrap();
    assert_test!(outsize == "value".len());
    assert_test!(&value[..outsize] == b"value");

    assert_test!(sparse_dict_free(dict) != 0);
    true
}

fn test_dict_lots_of_set() -> bool {
    let mut dict = sparse_dict_init().unwrap();
    let iterations = 1_000_000;

    for i in 0..iterations {
        let key = format!("crazy hash{}", i);
        let val = format!("value{}", i);

        assert_test!(sparse_dict_set(&mut dict, &key, key.len(), val.as_bytes(), val.len()) != 0);
        assert_test!(dict.bucket_count == i + 1);

        let mut outsize = 0usize;
        let retrieved = sparse_dict_get(&dict, &key, key.len(), Some(&mut outsize));
        assert_test!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_test!(outsize == val.len());
        assert_test!(&retrieved[..outsize] == val.as_bytes());
    }

    for i in (0..iterations).rev() {
        let key = format!("crazy hash{}", i);
        let val = format!("value{}", i);

        let mut outsize = 0usize;
        let retrieved = sparse_dict_get(&dict, &key, key.len(), Some(&mut outsize));
        assert_test!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_test!(outsize == val.len());
        assert_test!(&retrieved[..outsize] == val.as_bytes());
    }

    assert_test!(sparse_dict_free(dict) != 0);
    true
}

fn main() {
    let mut tests_passed = 0;
    let mut tests_failed = 0;

    run_test!(test_cannot_set_bigger_elements, tests_passed, tests_failed);
    run_test!(test_cannot_set_outside_bounds, tests_passed, tests_failed);
    run_test!(test_cannot_get_outside_bounds, tests_passed, tests_failed);
    run_test!(test_empty_array_does_not_blow_up, tests_passed, tests_failed);
    run_test!(test_array_set, tests_passed, tests_failed);
    run_test!(test_array_set_backwards, tests_passed, tests_failed);
    run_test!(test_array_set_overwrites_old_values, tests_passed, tests_failed);
    run_test!(test_array_set_high_num, tests_passed, tests_failed);
    run_test!(test_array_get, tests_passed, tests_failed);
    run_test!(test_dict_set, tests_passed, tests_failed);
    run_test!(test_dict_get, tests_passed, tests_failed);
    run_test!(test_dict_lots_of_set, tests_passed, tests_failed);

    println!("\n-----\nTests passed: ({}/{})", tests_passed, tests_passed + tests_failed);
}

use std::collections::HashMap;
use std::os::raw::c_int;
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
struct IntMap {
    map: HashMap<c_int, c_int>,
    default: c_int,
}

static INTMAP: OnceLock<Mutex<IntMap>> = OnceLock::new();

fn intmap() -> &'static Mutex<IntMap> {
    INTMAP.get_or_init(|| Mutex::new(IntMap { map: HashMap::new(), default: 0 }))
}

#[unsafe(no_mangle)]
pub extern "C" fn hm_geti(num: c_int) {
    let mutex = intmap();
    let mut state = mutex.lock().unwrap();
    state.map.clear();

    let mut i: c_int = 1;
    assert_eq!(-1, if state.map.contains_key(&i) { i as isize } else { -1 } as c_int);
    state.default = -2;
    assert_eq!(-1, if state.map.contains_key(&i) { i as isize } else { -1 } as c_int);
    assert_eq!(*state.map.get(&i).unwrap_or(&state.default), -2);

    i = 0;
    while i < num {
        state.map.insert(i, i * 5);
        i += 2;
    }

    i = 0;
    while i < num {
        let temp = *state.map.get(&i).unwrap_or(&state.default);
        if (i & 1) != 0 {
            assert_eq!(temp, -2);
        } else {
            assert_eq!(temp, i * 5);
        }
        let temp_ts = *state.map.get(&i).unwrap_or(&state.default);
        if (i & 1) != 0 {
            assert_eq!(temp_ts, -2);
        } else {
            assert_eq!(temp_ts, i * 5);
        }
        i += 1;
    }

    i = 0;
    while i < num {
        state.map.insert(i, i * 3);
        i += 2;
    }

    i = 0;
    while i < num {
        let value = *state.map.get(&i).unwrap_or(&state.default);
        if (i & 1) != 0 {
            assert_eq!(value, -2);
        } else {
            assert_eq!(value, i * 3);
        }
        i += 1;
    }

    i = 2;
    while i < num {
        state.map.remove(&i);
        i += 4;
    }

    i = 0;
    while i < num {
        let value = *state.map.get(&i).unwrap_or(&state.default);
        if (i & 3) != 0 {
            assert_eq!(value, -2);
        } else {
            assert_eq!(value, i * 3);
        }
        i += 1;
    }

    i = 0;
    while i < num {
        state.map.remove(&i);
        i += 1;
    }

    i = 0;
    while i < num {
        let value = *state.map.get(&i).unwrap_or(&state.default);
        assert_eq!(value, -2);
        i += 1;
    }

    state.map.clear();

    i = 0;
    while i < num {
        state.map.insert(i, i * 3);
        i += 2;
    }

    state.map.clear();
}

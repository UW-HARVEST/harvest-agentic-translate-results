use std::env;
use std::process;

fn static_alias(outer: &mut i32) -> Option<&'static mut i32> {
    use std::sync::Mutex;
    static INNER: Mutex<i32> = Mutex::new(1);
    let mut inner = INNER.lock().unwrap();
    if *outer >= *inner {
        *inner += *outer;
        drop(inner);
        Some(unsafe {
            &mut *(INNER.as_ptr() as *mut i32)
        })
    } else {
        *outer += *inner;
        None
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Error: should only be two (integer) arguments!");
        process::exit(1);
    }
    
    let initial_value: i32 = match args[1].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };
    
    let iterations: i32 = match args[2].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Error: second argument must be an integer!");
            process::exit(1);
        }
    };
    
    let mut running_sum = initial_value;
    let mut use_static = false;
    
    for _ in 0..iterations {
        if use_static {
            if let Some(ptr) = static_alias(&mut running_sum) {
                println!("{}", *ptr);
            }
        } else {
            match static_alias(&mut running_sum) {
                Some(ptr) => {
                    use_static = true;
                    println!("{}", *ptr);
                }
                None => {
                    println!("{}", running_sum);
                }
            }
        }
    }
}

use std::env;
use std::process;

#[derive(Copy, Clone)]
enum RefTarget {
    Outer,
    Inner,
}

struct AliasState {
    outer: i32,
    inner: i32,
}

fn static_alias(state: &mut AliasState, target: RefTarget) -> RefTarget {
    let current_val = match target {
        RefTarget::Outer => state.outer,
        RefTarget::Inner => state.inner,
    };

    if current_val >= state.inner {
        state.inner += current_val;
        RefTarget::Inner
    } else {
        match target {
            RefTarget::Outer => state.outer += state.inner,
            RefTarget::Inner => state.inner += state.inner,
        }
        target
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Error: should only be two (integer) arguments!");
        process::exit(1);
    }

    let initial_value = match args[1].parse::<i32>() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: first argument must be an integer!");
            process::exit(1);
        }
    };

    let iterations = match args[2].parse::<i32>() {
        Ok(v) => v,
        Err(_) => {
            println!("Error: second argument must be an integer!");
            process::exit(1);
        }
    };

    let mut state = AliasState {
        outer: initial_value,
        inner: 1,
    };
    let mut current_target = RefTarget::Outer;

    for _ in 0..iterations {
        current_target = static_alias(&mut state, current_target);
        let val = match current_target {
            RefTarget::Outer => state.outer,
            RefTarget::Inner => state.inner,
        };
        println!("{}", val);
    }
}

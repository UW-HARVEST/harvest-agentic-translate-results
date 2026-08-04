use Graph_recogniser::check;

#[test]
fn test_check_passes_for_true() {
    check::check(true);
}

#[test]
fn test_check_passes_for_evaluated_expressions() {
    check::check(1 + 1 == 2);
    check::check(2u32 > 1u32);
    check::check(!"abc".is_empty());
}

#[cfg(debug_assertions)]
#[test]
#[should_panic]
fn test_check_panics_on_false_in_debug() {
    check::check(false);
}

fn main() {}

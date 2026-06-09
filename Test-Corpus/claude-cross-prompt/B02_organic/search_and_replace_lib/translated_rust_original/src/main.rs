// The original C package builds a shared library (`driver`) consisting of a
// single function `searchAndReplace` and has no `main`. To match the original's
// behavior when invoked as an executable, this binary produces no output and
// exits successfully.

#[allow(unused_imports)]
use driver::search_and_replace;

fn main() {}

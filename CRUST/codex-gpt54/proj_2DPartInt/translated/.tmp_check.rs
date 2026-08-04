fn main() {
    let mut a = Vec::<i32>::new();
    let v: Vec<&mut Vec<i32>> = vec![&mut a];
    let rv = &v;
    rv[0].push(1);
}

struct Point<T> {
    x: T,
    y: T,
}

fn main() {
    let point = Point {x: 5, y: 10};
    println!("Point is {} {}", point.x, point.y);
}

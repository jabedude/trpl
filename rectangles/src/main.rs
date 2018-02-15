#[derive(Debug)]
struct Rectangle {
    width: u32,
    length: u32,
}

impl Rectangle {
    fn area(&self) -> u32 {
        self.width * self.length
    }

    fn can_hold(&self, other: &Rectangle) -> bool {
        self.area() > other.area()
    }
}

fn main() {
    let rect = Rectangle{ width: 30, length: 50 };
    let rect1 = Rectangle{ width: 40, length: 50 };

    println!("Can rect hold rect1? {}", rect.can_hold(&rect1));
}

use std::error::Error;
fn main() {
    println!("Getting value {}", add_one(3));
}

fn add_one(x: i32) -> Result<i32, Error> {
    match x {
        1 => Ok(2),
        _ => Err(1),
    }
}

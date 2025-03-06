const GLOBAL: u32 = 1;

fn main() {
    println!("{}", 2 + 2);
    println!("{}", GLOBAL);
    println!("Hello, world!");
}

#[cfg(feature = "a")]
fn test() {}

#[derive(Debug, Eq)]
#[cfg(all(feature = "a", feature = "b"))]
#[cfg_attr(serde, derive(serde::Serialize, serde::Deserialize))]
struct Example {
    x: u32,
    s: String,
    t: Test
}
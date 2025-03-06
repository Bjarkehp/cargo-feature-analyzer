const GLOBAL: u32 = 1;

#[cfg(flag = "example")]
fn main() {
    println!("{}", 2 + 2);
    println!("Hello, world!");
    let x = Example;
    println!("{:?}", x);
}

#[derive(Debug, Eq)]
#[cfg(all(flag = "example", feature))]
#[cfg_attr(serde, derive(serde::Serialize, serde::Deserialize))]
struct Example {
    x: u32,
    s: String,
    t: Test
}

struct Test;
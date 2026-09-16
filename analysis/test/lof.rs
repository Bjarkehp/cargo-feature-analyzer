fn test1() {

}

#[cfg(feature = "my-feature")]
fn test2() {

}

#[cfg(all(target_os = "linux", feature = "my-feature"))]
struct Test3;

#[cfg_attr(
    feature = "my-feature", 
    derive(Debug)
)]
enum Test4 {
    Variant1()
}

#[derive(Debug)]
struct Test5 {
    #[cfg(feature = "my-feature")]
    field: u32
}

#[derive(Debug)]
#[cfg(feature = "my-feature")]
struct Test6;

#[cfg(feature = "my-feature")]
#[derive(Debug)]
struct Test7;

#[cfg(target_os = "linux")]
struct Test8;

#[cfg_attr(feature = "my-feature", derive(Debug))]
#[cfg(feature = "my-feature")]
struct Test9;
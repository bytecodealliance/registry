fn main() {
    #[cfg(not(feature = "evil"))]
    println!("Hello, world!");
    #[cfg(feature = "evil")]
    println!("Hello! I'm malicious!")
}

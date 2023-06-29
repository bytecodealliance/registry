use anyhow::Result;
use warg_crypto::signing::generate_p256_pair;

pub fn main() -> Result<()> {
    let (public, private) = generate_p256_pair();
    println!("Public Key: {}", public);
    println!("Private Key: {}", private.encode().as_str());
    Ok(())
}

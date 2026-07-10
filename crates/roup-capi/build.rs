use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let public_header = PathBuf::from("include/roup.h");
    let output =
        PathBuf::from(env::var_os("OUT_DIR").ok_or("Cargo did not define OUT_DIR")?).join("roup.h");
    fs::copy(&public_header, &output)?;
    println!("cargo:rerun-if-changed={}", public_header.display());
    println!("cargo:rustc-env=ROUP_CAPI_HEADER={}", output.display());
    Ok(())
}

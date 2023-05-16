fn main() {
    println!(
        "cargo:rustc-env=RYE_TARGET={}",
        std::env::var("TARGET").unwrap()
    );
}

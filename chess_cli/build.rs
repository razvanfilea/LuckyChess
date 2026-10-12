fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Windows gives the main thread a 1 MB stack, while Linux and macOS default to 8 MB.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        const STACK_SIZE: u32 = 8 * 1024 * 1024;
        match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
            Ok("msvc") => println!("cargo:rustc-link-arg-bins=/STACK:{STACK_SIZE}"),
            _ => println!("cargo:rustc-link-arg-bins=-Wl,--stack,{STACK_SIZE}"),
        }
    }
}

fn main() {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| String::from("debug"));
    let libname = if cfg!(windows) {
        "c_dynamic_library.dll"
    } else {
        "c_dynamic_library"
    };

    println!("cargo::rustc-link-search=dylib=target/{profile}/deps");
    println!("cargo::rustc-link-lib=dylib={libname}");
}

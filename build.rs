fn main() {
    cc::Build::new()
        .file("stringleton-dylib/externs.c")
        .define("EXPORT", "1")
        .compile("stringleton_dylib_export");
}

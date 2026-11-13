fn main() {
    let output = std::process::Command::new("xcrun")
        .args(["clang", "--print-resource-dir"])
        .output()
        .expect("Xcode clang is required");
    assert!(output.status.success(), "Could not find clang runtime");
    let resource = std::str::from_utf8(&output.stdout)
        .expect("clang resource path is UTF-8")
        .trim();
    // SDL's availability checks use compiler-rt; rustc suppresses clang's default runtime link.
    println!("cargo:rustc-link-search=native={resource}/lib/darwin");
    println!("cargo:rustc-link-lib=static=clang_rt.osx");
}

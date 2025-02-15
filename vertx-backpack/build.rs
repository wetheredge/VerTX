use std::env;

fn main() {
    let mut args = vec!["-Tdefmt.x"];
    if chip("ESP") {
        args.push("-Tlinkall.x");
    }

    for arg in args {
        println!("cargo::rustc-link-arg-bins={arg}");
    }
}

fn chip(feature: &str) -> bool {
    env::var_os(format!("CARGO_FEATURE_CHIP_{feature}")).is_some()
}

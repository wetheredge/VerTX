use std::env;

fn main() {
    let args: &[&str] = if chip("ESP") {
        &["-Tlinkall.x", "-Trom_functions.x", "-nostartfiles"]
    } else {
        &[]
    };

    for arg in args {
        println!("cargo::rustc-link-arg-bins={arg}");
    }
}

fn chip(feature: &str) -> bool {
    env::var_os(format!("CARGO_FEATURE_CHIP_{feature}")).is_some()
}

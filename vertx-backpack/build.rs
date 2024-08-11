use std::env;

fn main() {
    let link_args = if is_target("ESP32") {
        vec!["-Tlinkall.x", "-Trom_functions.x", "-nostartfiles"]
    } else {
        vec![]
    };

    for link_arg in link_args {
        println!("cargo::rustc-link-arg={link_arg}");
    }
}

fn is_target(feature: &str) -> bool {
    env::var_os(format!("CARGO_FEATURE_TARGET_{feature}")).is_some()
}

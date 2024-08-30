mod backpack;
mod child;
mod ui;

use std::env;
use std::path::PathBuf;

fn get_target_dir() -> PathBuf {
    let dir = env::current_dir().unwrap();
    let mut dir = dir.as_path();
    loop {
        let target = dir.join("target");
        if target.exists() {
            break target;
        }

        dir = dir.parent().unwrap();
    }
}

#[tokio::main]
async fn main() {
    relm4_icons::initialize_icons();
    let app = relm4::RelmApp::new("vertx.simulator");
    app.run::<ui::App>(());
}

use std::path::Path;

use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::stylesheet::{ParserOptions, PrinterOptions};

fn main() {
    println!("cargo:rerun-if-changed=assets/css");

    let fs = FileProvider::new();
    let mut bundler = Bundler::new(&fs, None, ParserOptions::default());
    let stylesheet = bundler
        .bundle(Path::new("assets/css/app.css"))
        .expect("failed to bundle css");

    let output = stylesheet
        .to_css(PrinterOptions {
            minify: true,
            ..Default::default()
        })
        .expect("failed to serialize css");

    std::fs::create_dir_all("assets/dist").expect("failed to create assets/dist");
    std::fs::write("assets/dist/app.css", output.code).expect("failed to write bundle");
}

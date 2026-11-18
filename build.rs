use std::path::Path;

use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserFlags, ParserOptions, PrinterOptions};
use lightningcss::targets::{Browsers, Targets};

fn main() {
    println!("cargo:rerun-if-changed=assets/css");

    let targets = Targets::from(Browsers {
        chrome: Some(123 << 16),
        firefox: Some(120 << 16),
        safari: Some((17 << 16) | (5 << 8)),
        ..Browsers::default()
    });
    let fs = FileProvider::new();
    let options = ParserOptions {
        flags: ParserFlags::CUSTOM_MEDIA,
        ..ParserOptions::default()
    };
    let mut bundler = Bundler::new(&fs, None, options);
    let mut stylesheet = bundler
        .bundle(Path::new("assets/css/app.css"))
        .expect("failed to bundle css");
    stylesheet
        .minify(MinifyOptions {
            targets,
            ..MinifyOptions::default()
        })
        .expect("failed to minify css");

    let output = stylesheet
        .to_css(PrinterOptions {
            minify: true,
            targets,
            ..Default::default()
        })
        .expect("failed to serialize css");

    std::fs::create_dir_all("assets/dist").expect("failed to create assets/dist");
    std::fs::write("assets/dist/app.css", output.code).expect("failed to write bundle");
}

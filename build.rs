use std::path::PathBuf;

#[path = "src/cli.rs"]
mod cli;

fn main() {
    println!("cargo:rerun-if-changed=src/cli.rs");

    // Write to OUT_DIR so `cargo publish` verification passes (build scripts
    // must not modify files outside OUT_DIR).
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let cmd = <cli::Cli as clap::CommandFactory>::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer = Vec::new();
    man.render(&mut buffer).unwrap();

    // clap_mangen can emit trailing spaces (e.g. on the .TH line); strip them.
    let content = String::from_utf8(buffer).unwrap();
    let cleaned: String = content
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    std::fs::write(out_dir.join("deadbranch.1"), cleaned).unwrap();
}

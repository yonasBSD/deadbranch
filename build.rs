use std::path::PathBuf;

#[path = "src/cli.rs"]
mod cli;

fn main() {
    println!("cargo:rerun-if-changed=src/cli.rs");
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let man_dir = manifest_dir.join("man");
    std::fs::create_dir_all(&man_dir).unwrap();

    let cmd = <cli::Cli as clap::CommandFactory>::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer = Vec::new();
    man.render(&mut buffer).unwrap();

    // clap_mangen can emit trailing spaces (e.g. on the .TH line); strip them
    // so the committed file stays clean and pre-commit hooks don't loop.
    let content = String::from_utf8(buffer).unwrap();
    let cleaned: String = content
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    let man_path = man_dir.join("deadbranch.1");
    // Only write when content changes to avoid spurious mtime updates that
    // cause the pre-commit generate-man-page hook to report "files modified"
    // on every run even when nothing has changed.
    let existing = std::fs::read_to_string(&man_path).unwrap_or_default();
    if existing != cleaned {
        std::fs::write(&man_path, cleaned).unwrap();
    }
}

use anyhow::Result;
use clap::Parser;
use itertools::Itertools;

use cargo_edit_completion_lib::crates::CratesIndex;
use cargo_edit_completion_lib::{complete_crate, complete_feature};

#[derive(Parser)]
#[clap(version = "1.0", author = "LightQuantum <self@lightquantum.me>")]
struct Opts {
    #[clap(subcommand)]
    mode: Mode,
}

#[derive(Parser)]
struct WrappedString {
    input: String,
}

#[derive(Parser)]
enum Mode {
    Crate(WrappedString),
    Feature(WrappedString),
}

#[derive(Parser)]
struct Crate {
    input: String,
}

fn entry() -> Result<()> {
    let opts = Opts::try_parse()?;
    let index = CratesIndex::default();

    println!(
        "{}",
        match opts.mode {
            Mode::Crate(s) => complete_crate(&index, s.input.as_str())?
                .into_iter()
                .join("\n"),
            Mode::Feature(s) => {
                let (name, ver) = s.input.split_once('@').unwrap();
                complete_feature(&index, name, ver)?.join("\n")
            }
        }
    );
    Ok(())
}

// example:
// cargo_edit_completion crate tracing_test -> tracing-test, tracing-test-macro
// cargo_edit_completion crate actix-web@3 -> actix-web@3.3.2, actix-web@3.3.1, ...

fn main() {
    drop(entry())   // ignore all errors
}

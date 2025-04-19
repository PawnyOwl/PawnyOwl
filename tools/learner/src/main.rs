pub mod dataset;
pub mod learn;

use clap::Parser;
use learn::learn_model;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    dataset: String,
    artifact: String,
    model: String,
}

fn main() {
    let args = Args::parse();
    learn_model(&args.dataset, &args.artifact, &args.model);
}

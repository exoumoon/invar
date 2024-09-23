use clap::Parser;
use invar::cli::Options;

fn main() {
    let options = Options::parse();
    dbg!(&options);
}

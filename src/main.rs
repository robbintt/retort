use clap::Parser;

/// Retort: An AI pair programmer
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The prompt to send to the model, bypassing the prompt-builder
    #[arg(short, long)]
    prompt: String,
}

fn main() {
    let args = Args::parse();
    println!("{}", args.prompt);
}

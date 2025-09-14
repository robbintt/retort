use clap::Parser;

mod config;

/// Retort: An AI pair programmer
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The prompt to send to the model, bypassing the prompt-builder
    #[arg(short, long)]
    prompt: String,
}

fn main() -> anyhow::Result<()> {
    let _args = Args::parse();
    let _config = config::load()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert_eq!(2 + 2, 4);
    }
}

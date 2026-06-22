use clap::Parser;

/// Task switcher like interface for interacting with tray indicators
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    /// Starts GTK inspector with the application (useful for theming)
    #[clap(long)]
    pub inspector: bool,

    // TODO
    // /// Load CSS styling file
    // #[clap(long)]
    // pub style: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}

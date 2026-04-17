use clap::Parser;

#[derive(Parser, Debug, Clone, Default)]
#[command(name = "toaster", about = "Toaster - transcript-first video/audio editor")]
pub struct CliArgs {
    /// Start with the main window hidden
    #[arg(long)]
    pub start_hidden: bool,

    /// Enable debug mode with verbose logging
    #[arg(long)]
    pub debug: bool,
}

use clap::{command, Parser, Subcommand};

// ///////////// //
// CLI interface //
// ///////////// //

/// cups2mqtt - A service that periodically reads print queue statuses from CUPS and publishes these statuses to a MQTT server.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Dumps the IPP response to stdout.
    Dump,
}

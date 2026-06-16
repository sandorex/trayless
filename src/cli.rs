use std::{fmt::Display, path::PathBuf};
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    JSON,
    CSVNull,
    CSV,
}

/// Utility for manipulating and querying tray indicators
///
/// Intended for use with window managers but works on any complient desktop
/// environment
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
    /// Use specific format for output data
    #[clap(short, long, default_value = "json")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub cmd: CliCommands,
}

#[derive(Args, Debug, Clone)]
pub struct CmdList {
    // TODO
    // #[clap(long, exclusive = true)]
    // pub list_fields: bool,
    //
    // /// Select which fields to return (empty for all)
    // #[clap(short, long)]
    // pub fields: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub struct CmdLayout {
    /// Id of the item or full path
    #[clap(value_name = "ID | PATH")]
    pub path: String,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommands {
    /// List tray items
    List(CmdList),

    // /// Activate a tray item
    // Activate(),

    // /// Activate menu item of a tray item
    // Menu(),

    /// Returns menu layout for tray item
    Layout(CmdLayout),
    // /// Formats device/partition (ERASES ALL DATA!)
    // ///
    // /// In case target is a device block file then it formats it to contain a
    // /// single FAT32 partition with MBR/BIOS partition table
    // #[cfg_attr(target_os = "windows", clap(skip))]
    // Format,
    //
    // /// Shuffle music
    // Shuffle(CmdShuffle),
    //
    // /// Cleans up the links making it editable directly
    // Clean(CmdClean),
    //
    // /// Imports file into the filesystem without mounting it, will not overwrite files
    // Import(CmdImport),
    //
    // /// Processes files using ffmpeg to apply some adjustments (recommended)
    // ///
    // /// All options have a description but always test if the files are playable on a computer!
    // Process(CmdProcess),

    #[clap(skip)]
    None,
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

use clap::{Args, Parser, Subcommand};

/// Utility for manipulating and querying tray indicators
///
/// Intended for use with window managers but works on any complient desktop
/// environment
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Cli {
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

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CmdActivateType {
    ContextMenu,
    Activate,
    SecondaryActivate,
    Scroll
}

#[derive(Args, Debug, Clone)]
pub struct CmdActivate {
    #[clap(value_name = "DESTINATION")]
    pub destination: String,

    /// X position of activation
    pub x: i32,

    /// Y position of activation
    pub y: i32,
}

#[derive(Args, Debug, Clone)]
pub struct CmdScroll {
    #[clap(value_name = "DESTINATION")]
    pub destination: String,

    /// How much was scrolled
    pub delta: i32,

    /// Direction of scrolling
    pub orientation: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdActivateMenu {
    #[clap(value_name = "DESTINATION")]
    pub destination: String,

    /// How much was scrolled
    pub delta: i32,

    /// Direction of scrolling
    pub orientation: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdLayout {
    /// Show hidden menu items
    #[clap(short='H', long)]
    pub hidden: bool,

    /// Show disabled menu items
    #[clap(short, long)]
    pub disabled: bool,

    /// Path to the dbusmenu object including destination
    #[clap(value_name = "MENU_PATH")]
    pub path: String,
}

#[derive(Args, Debug, Clone)]
pub struct CmdMenu {
    /// Id of the item or full path
    #[clap(value_name = "ID | PATH")]
    pub path: String,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommands {
    /// List tray items
    List(CmdList),

    /// Call context menu on a tray item
    ContextMenu(CmdActivate),

    /// Call activate on a tray item
    Activate(CmdActivate),

    /// Call secondary activate on a tray item
    SecondaryActivate(CmdActivate),

    /// Call scroll on a tray item
    Scroll(CmdScroll),

    /// Get tray item layout
    GetLayout(CmdLayout),

    /// Activate specific item in menu
    Menu(CmdMenu),

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

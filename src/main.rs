mod dbus;
mod cli;
mod utils;
mod daemon;
mod menu_node;
mod tray_item;

use utils::*;
use std::{io::IsTerminal, sync::LazyLock};
use anyhow::{Context, anyhow, Result};
use clap::Parser;
use zbus::{blocking::{Connection, fdo::DBusProxy}, zvariant::OwnedValue};

use crate::tray_item::TrayItem;

// global session bus cause its gonna be used everywhere
static CONN: LazyLock<Connection> = LazyLock::new(|| {
    Connection::session().expect("could not connect to session bus")
});

static DBUS_PROXY: LazyLock<DBusProxy> = LazyLock::new(|| {
    DBusProxy::new(&CONN).expect("could not open dbus proxy")
});

static IS_TTY: LazyLock<bool> = LazyLock::new(|| {
    std::io::stdout().is_terminal()
});

fn main() -> Result<()> {
    let mut args = cli::Cli::parse();
    let cmd = std::mem::replace(&mut args.cmd, cli::CliCommands::None);

    // TODO catch the anyhow error and terminate with code 1 if there is any

    match cmd {
        cli::CliCommands::List(x) => cmd_list(args, x)?,
        cli::CliCommands::Activate(x) => cmd_activate(args, x)?,
        cli::CliCommands::Scroll(x) => cmd_scroll(args, x)?,
        cli::CliCommands::Layout(x) => cmd_layout(args, x)?,
        cli::CliCommands::Click(x) => cmd_menu(args, x)?,
        cli::CliCommands::Daemon => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(daemon::cmd_daemon(args, ()))?;
        },
        cli::CliCommands::None => unreachable!(),
    };

    Ok(())
}

fn cmd_list(_cli_args: cli::Cli, _cmd_args: cli::CmdList) -> Result<()> {
    let items = get_registered_items()
        .with_context(|| "could not get registered items from StatusNotifierWatcher")?
        .into_iter()
        .map(|item| -> Result<TrayItem> {
            let (dest, path) = split_path(&item)
                .with_context(|| anyhow!("invalid path from registered items {item:?}"))?;

            let proxy = get_item_proxy(&dest, &path)
                .with_context(|| anyhow!("could not get item proxy from {item:?}"))?;

            Ok(TrayItem::from_proxy(dest, path, &proxy))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut stdout = std::io::stdout();

    if *IS_TTY {
        serde_json::to_writer_pretty(&mut stdout, &items)?;
        println!();
    } else {
        serde_json::to_writer(&mut stdout, &items)?;
    }

    Ok(())
}

fn cmd_activate(_cli_args: cli::Cli, cmd_args: cli::CmdActivate) -> Result<()> {
    let (dest, path) = split_path(&cmd_args.path)
        .with_context(|| anyhow!("could not parse destination {:?}", cmd_args.path))?;

    let proxy = get_item_proxy(&dest, &path)
        .with_context(|| anyhow!("could not get item proxy from {:?}", cmd_args.path))?;

    if cmd_args.secondary {
        proxy.secondary_activate(cmd_args.x, cmd_args.y)?;
    } else if cmd_args.context_menu {
        proxy.context_menu(cmd_args.x, cmd_args.y)?;
    } else { // regular activation
        proxy.activate(cmd_args.x, cmd_args.y)?;
    }

    Ok(())
}

fn cmd_scroll(_cli_args: cli::Cli, cmd_args: cli::CmdScroll) -> Result<()> {
    let (dest, path) = split_path(&cmd_args.path)
        .with_context(|| anyhow!("could not parse destination {:?}", cmd_args.path))?;

    let proxy = get_item_proxy(&dest, &path)
        .with_context(|| anyhow!("could not get item proxy from {:?}", cmd_args.path))?;

    proxy.scroll(cmd_args.delta, &cmd_args.orientation)?;

    Ok(())
}

fn cmd_layout(_cli_args: cli::Cli, cmd_args: cli::CmdLayout) -> Result<()> {
    let (dest, path) = split_path(&cmd_args.path)
        .with_context(|| anyhow!("could not parse destination {:?}", cmd_args.path))?;

    let proxy = get_item_menu_proxy(&dest, &path)
        .with_context(|| anyhow!("could not get menu proxy from {:?}", cmd_args.path))?;

    let layout = get_item_menu_layout(&proxy)
        .with_context(|| anyhow!("could not get menu layout from {:?}", cmd_args.path))?;

    // TODO implement --hidden --enabled (currently all are shown)

    let mut stdout = std::io::stdout();

    if *IS_TTY {
        serde_json::to_writer_pretty(&mut stdout, &layout)?;
        println!();
    } else {
        serde_json::to_writer(&mut stdout, &layout)?;
    }

    Ok(())
}

fn cmd_menu(_cli_args: cli::Cli, cmd_args: cli::CmdClick) -> Result<()> {
    let (dest, path) = split_path(&cmd_args.path)
        .with_context(|| anyhow!("could not parse destination {:?}", cmd_args.path))?;

    let proxy = get_item_menu_proxy(&dest, &path)
        .with_context(|| anyhow!("could not get menu proxy from {:?}", cmd_args.path))?;

    proxy.event(cmd_args.id, "clicked", cmd_args.data.as_ref().unwrap_or(&OwnedValue::from(0)), 0)?;

    Ok(())
}

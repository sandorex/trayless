mod dbus;
mod cli;

use std::collections::HashMap;
use std::io::IsTerminal;
use std::sync::LazyLock;
use anyhow::{Context, anyhow, Result};
use clap::Parser;
use dbus::status_notifier_watcher::StatusNotifierWatcherProxyBlocking;
use dbus::status_notifier_item::StatusNotifierItemProxyBlocking;
use zbus::blocking::Connection;
use crate::cli::OutputFormat;
use crate::dbus::dbusmenu::dbusmenuProxyBlocking;

// global session bus cause its gonna be used everywhere
static CONN: LazyLock<Connection> = LazyLock::new(|| {
    Connection::session().expect("could not connect to session bus")
});

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct TrayItem {
    pub id: String,
    pub title: String,
    // pub tooltip: String,
    // pub icon_name: String,
    // pub icon_theme_path: String,
    // pub icon_pixmap: String, // TODO use vec<u8>?
    pub item_is_menu: bool,
    pub menu: String,
}

impl TrayItem {
    pub fn new(proxy: &StatusNotifierItemProxyBlocking) -> Self {
        Self {
            id: proxy.id().unwrap_or_default(),
            title: proxy.title().unwrap_or_default(),
            item_is_menu: proxy.item_is_menu().unwrap_or(false),
            menu: proxy.menu().map(|x| x.to_string()).unwrap_or_default(),
        }
    }
}

/// Splits path like `:1.298/org/ayatana/NotificationItem/TOA2Xeo1bQ` into
/// destination and path
fn split_path(input: &str) -> Option<(String, String)> {
    // just find first slash
    let (id, path) = input.split_once('/')?;

    Some((id.to_owned(), format!("/{path}")))
}

/// Returns registered tray items
fn get_registered_items() -> Result<Vec<String>> {
    let watcher_proxy = StatusNotifierWatcherProxyBlocking::builder(&CONN)
        .destination("org.kde.StatusNotifierWatcher")?
        .path("/StatusNotifierWatcher")?
        .build()?;

    Ok(watcher_proxy.registered_status_notifier_items()?)
}

/// Get proxy for tray item
fn get_item_proxy<'a>(id: &str, path: &str) -> Result<StatusNotifierItemProxyBlocking<'a>> {
    Ok(
        StatusNotifierItemProxyBlocking::builder(&CONN)
            .destination(id.to_owned())?
            .path(path.to_owned())?
            .build()?
    )
}

fn get_item_menu_proxy<'a>(id: &str, path: &str) -> Result<dbusmenuProxyBlocking<'a>> {
    Ok(
        dbusmenuProxyBlocking::builder(&CONN)
            .destination(id.to_owned())?
            .path(path.to_owned())?
            .build()?
    )
}

// NOTE basically goes like [ <id>, <hashmap of properties>, <children> ] so recursion
fn get_item_menu_layout(proxy: &dbusmenuProxyBlocking) -> Result<(u32, (i32, HashMap<String, zbus::zvariant::OwnedValue>, Vec<zbus::zvariant::OwnedValue>,))>
{
    Ok(proxy.get_layout(0, -1, &[])?)
}

// TODO take into account ItemIsMenu whether it is activatable
fn main() -> Result<()> {
    let mut args = cli::Cli::parse();
    let cmd = std::mem::replace(&mut args.cmd, cli::CliCommands::None);

    // dbg!(&args, &cmd);

    match cmd {
        cli::CliCommands::List(x) => cmd_list(args, x)?,
        cli::CliCommands::Layout(x) => todo!(),
        cli::CliCommands::None => unreachable!(),
    };

    // let items = get_registered_items()?;
    //
    // for item in &items {
    //     let (id, path) = split_path(item)
    //         .with_context(|| anyhow!("invalid path from registered items {item:?}"))?;
    //
    //     let proxy = get_item_proxy(&id, &path)?;
    //     println!("{:?}: {:?}", proxy.id(), proxy.title());
    //
    //     let menu_path = proxy.menu()?;
    //
    //     let menu_proxy = get_item_menu_proxy(&id, &menu_path)?;
    //     let menu_layout = menu_proxy.get_layout(0, -1, &[])?;
    //
    //     dbg!(&menu_layout);
    // }


    Ok(())
}

fn cmd_list(cli_args: cli::Cli, cmd_args: cli::CmdList) -> Result<()> {
    let is_tty = std::io::stdout().is_terminal();

    let items = get_registered_items()?
        .into_iter()
        .map(|item| -> Result<TrayItem> {
            let (id, path) = split_path(&item)
                .with_context(|| anyhow!("invalid path from registered items {item:?}"))?;

            let proxy = get_item_proxy(&id, &path)?;

            Ok(TrayItem::new(&proxy))
        });

    match cli_args.format {
        OutputFormat::JSON => {
            let mut stdout = std::io::stdout();

            let items = items.collect::<Result<Vec<_>, _>>()?;

            // print pretty in terminal
            if is_tty {
                serde_json::to_writer_pretty(&mut stdout, &items)?;
            } else {
                serde_json::to_writer(&mut stdout, &items)?;
            }
        },
        OutputFormat::CSV | OutputFormat::CSVNull => {
            let mut writer = csv::WriterBuilder::new();

            // write headers only in tty
            writer.has_headers(is_tty);

            if matches!(cli_args.format, OutputFormat::CSVNull) {
                writer.delimiter(b'\0');
            } else {
                writer.delimiter(b',');
            }

            let mut writer = writer.from_writer(std::io::stdout());

            for item in items {
                writer.serialize(item?)?;
            }
        }
    }

    Ok(())
}

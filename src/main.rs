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
use zbus::blocking::fdo::DBusProxy;
use zbus::names::BusName;
use zbus::zvariant::{OwnedValue, Value};
use crate::dbus::dbusmenu::dbusmenuProxyBlocking;

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

#[derive(Debug, Clone, serde::Serialize)]
struct TrayItem {
    pub id: String,
    pub title: String,
    // pub tooltip: String, // TODO how do i get them on demand???
    // pub icon_name: String,
    // pub icon_theme_path: String,
    // pub icon_pixmap: String,
    pub item_is_menu: bool,
    pub menu: String,

    pub destination: String,
    pub pid: u32,
    pub exe: String,
}

impl TrayItem {
    pub fn from_proxy(destination: String, proxy: &StatusNotifierItemProxyBlocking) -> Self {
        let pid = DBUS_PROXY.get_connection_unix_process_id(BusName::try_from(destination.clone()).unwrap())
            .unwrap_or(0);

        let exe = if pid != 0 {
            get_exe_from_pid(pid)
        } else {
            "".to_owned()
        };

        Self {
            id: proxy.id().unwrap_or_default(),
            title: proxy.title().unwrap_or_default(),
            item_is_menu: proxy.item_is_menu().unwrap_or(false),
            menu: proxy.menu().map(|x| x.to_string()).unwrap_or_default(),

            destination,
            pid,
            exe
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct MenuNode {
    id: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    visible: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    toggle_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    toggle_state: Option<i32>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<Self>,
}

impl MenuNode {
    pub fn new(id: i32, props: HashMap<String, OwnedValue>, children: Vec<OwnedValue>) -> Result<Self> {
        macro_rules! get {
            ($name:literal, $type:ty, $err:literal) => {
                match props.get($name) {
                    Some(x) => Some(
                        TryInto::<$type>::try_into(
                            x.downcast_ref::<Value>()
                             .with_context(|| anyhow!($err))?
                        )?
                    ),
                    None => None,
                }
            };
        }

        let children = children.into_iter()
            .map(TryInto::<(i32, HashMap<String, OwnedValue>, Vec<OwnedValue>)>::try_into)
            .collect::<Result<Vec<_>, _>>()?;

        let children = children.into_iter()
            .map(|(id, props, children)| Self::new(id, props, children))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            id,
            label: get!("label", String, "label is not a string")
                // remove underscore markers
                .map(|x| x.replace("_", "")),
            enabled: get!("enabled", bool, "enabled is not a bool"),
            visible: get!("visible", bool, "visible is not a bool"),
            toggle_type: get!("toggle-type", String, "toggle_type is not a string"),
            toggle_state: get!("toggle-state", i32, "enabled is not a bool"),
            children,
        })
    }

    // TODO add a function to recursively go over all children but able to modify it

    #[allow(unused)]
    pub fn is_root(&self) -> bool {
        self.id == 0
    }

    #[allow(unused)]
    pub fn is_separator(&self) -> bool {
        self.label.is_none()
            && self.enabled.is_none()
            && self.label.is_none()
            && self.toggle_type.is_none()
            && self.toggle_state.is_none()
            && self.children.is_empty()
    }
}

/// Returns executable of the process
pub fn get_exe_from_pid(pid: u32) -> String {
    match std::fs::read_link(format!("/proc/{pid}/exe")) {
        Ok(x) => x
            .to_string_lossy()
            .to_string(),
        Err(_) => "".to_owned(),
    }
}

/// Splits path like `:1.298/org/ayatana/NotificationItem/TOA2Xeo1bQ` into
/// destination and path
fn split_path(input: &str) -> Option<(String, String)> {
    // just find first slash
    let (dest, path) = input.split_once('/')?;

    Some((dest.to_owned(), format!("/{path}")))
}

/// Returns registered tray items
fn get_registered_items() -> Result<Vec<String>> {
    let watcher_proxy = StatusNotifierWatcherProxyBlocking::builder(&CONN)
        .destination("org.kde.StatusNotifierWatcher")? // TODO fallback on org.freedesktop.*
        .path("/StatusNotifierWatcher")?
        .build()?;

    Ok(watcher_proxy.registered_status_notifier_items()?)
}

/// Get proxy for tray item
fn get_item_proxy<'a>(dest: &str, path: &str) -> Result<StatusNotifierItemProxyBlocking<'a>> {
    Ok(
        StatusNotifierItemProxyBlocking::builder(&CONN)
            .destination(dest.to_owned())?
            .path(path.to_owned())?
            .build()?
    )
}

fn get_item_menu_proxy<'a>(dest: &str, path: &str) -> Result<dbusmenuProxyBlocking<'a>> {
    Ok(
        dbusmenuProxyBlocking::builder(&CONN)
            .destination(dest.to_owned())?
            .path(path.to_owned())?
            .build()?
    )
}

fn get_item_menu_layout(proxy: &dbusmenuProxyBlocking) -> Result<MenuNode>
{
    let (_, (id, props, children)) = proxy.get_layout(0, -1, &[
        "type", "label", "toggle-type", "toggle-state", "enabled", "visible"
    ])?;

    Ok(MenuNode::new(id, props, children)?)
}

fn main() -> Result<()> {
    let mut args = cli::Cli::parse();
    let cmd = std::mem::replace(&mut args.cmd, cli::CliCommands::None);

    // dbg!(&args, &cmd);

    match cmd {
        cli::CliCommands::List(x) => cmd_list(args, x)?,
        cli::CliCommands::GetLayout(x) => cmd_layout(args, x)?,
        cli::CliCommands::None => unreachable!(),
        _ => todo!(),
    };

    Ok(())
}

fn cmd_list(_cli_args: cli::Cli, _cmd_args: cli::CmdList) -> Result<()> {
    let items = get_registered_items()?
        .into_iter()
        .map(|item| -> Result<TrayItem> {
            let (dest, path) = split_path(&item)
                .with_context(|| anyhow!("invalid path from registered items {item:?}"))?;

            let proxy = get_item_proxy(&dest, &path)?;

            Ok(TrayItem::from_proxy(dest.to_owned(), &proxy))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut stdout = std::io::stdout();

    // print pretty in terminal
    if *IS_TTY {
        serde_json::to_writer_pretty(&mut stdout, &items)?;

        // add newline
        println!();
    } else {
        serde_json::to_writer(&mut stdout, &items)?;
    }

    Ok(())
}

fn cmd_layout(_cli_args: cli::Cli, cmd_args: cli::CmdLayout) -> Result<()> {
    let (dest, path) = split_path(&cmd_args.path)
        .with_context(|| anyhow!("could not parse {:?} as destination or path", cmd_args.path))?;

    let proxy = get_item_menu_proxy(&dest, &path)?;

    let layout = get_item_menu_layout(&proxy)?;

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

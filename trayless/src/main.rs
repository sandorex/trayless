mod cli;
mod daemon;

use std::{io::IsTerminal, sync::LazyLock};
use anyhow::{Context, anyhow, Result};
use clap::{CommandFactory, Parser};
use zbus::{blocking::{Connection, fdo::DBusProxy}, zvariant::OwnedValue};
use libtrayless::{MenuNode, TrayItem, get_item_menu_layout, get_item_menu_proxy, get_item_proxy, get_items, split_path};

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

fn cmd_list(_cli_args: cli::Cli, cmd_args: cli::CmdList) -> Result<()> {
    use code_docs::DocumentedStruct;
    use std::fmt::Write;

    if cmd_args.help {
        let mut output = String::new();

        // convert some types to be easier to understand for non-rust users
        let convert_type = |x: &str| -> String { x.replace("Vec<", "Array<").replace("Option<", "Optional<") };

        // add properties from Config
        let iter = TrayItem::field_names()
            .into_iter()
            .zip(TrayItem::field_types().into_iter())
            .zip(TrayItem::field_docs().into_iter())
            .map(|((name, r#type), docs)| (name, r#type, docs));

        for (name, t, docs) in iter {
            // skip any that contains '@skip' in its docs
            if docs.join("\n").contains("@skip") {
                continue;
            }

            // format like rust docs
            for i in docs {
                let _ = writeln!(&mut output, "///{i}");
            }

            let _ = writeln!(&mut output, "{name}: {}\n", convert_type(t));
        }

        println!("{}\n--- Properties of a StatusNotifierItem ---\n{}\n", generate_list_help().ansi(), output.trim());

        return Ok(());
    }

    let items = get_items(&CONN, &DBUS_PROXY)?;

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

    let proxy = get_item_proxy(&CONN, &dest, &path)
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

    let proxy = get_item_proxy(&CONN, &dest, &path)
        .with_context(|| anyhow!("could not get item proxy from {:?}", cmd_args.path))?;

    proxy.scroll(cmd_args.delta, &cmd_args.orientation)?;

    Ok(())
}

fn cmd_layout(_cli_args: cli::Cli, cmd_args: cli::CmdLayout) -> Result<()> {
    let (dest, path) = split_path(&cmd_args.path)
        .with_context(|| anyhow!("could not parse destination {:?}", cmd_args.path))?;

    let proxy = get_item_menu_proxy(&CONN, &dest, &path)
        .with_context(|| anyhow!("could not get menu proxy from {:?}", cmd_args.path))?;

    let mut layout = get_item_menu_layout(&proxy)
        .with_context(|| anyhow!("could not get menu layout from {:?}", cmd_args.path))?;

    fn filter_nodes(root: &mut MenuNode, hidden: bool, disabled: bool) {
        for id in root.children.iter().map(|x| x.id).collect::<Vec<_>>() {
            let (visible, enabled, index) = {
                let index = root.children.iter().position(|x| x.id == id).unwrap();
                (root.children[index].visible.clone(), root.children[index].enabled.clone(), index)
            };

            // is it hidden
            match (hidden, visible) {
                (false, Some(false)) => {
                    root.children.swap_remove(index);
                    continue
                },
                _ => {}
            };

            // is it disabled
            match (disabled, enabled) {
                (false, Some(false)) => {
                    root.children.swap_remove(index);
                    continue
                },
                _ => {}
            };

            // it is visible so now recursively go into the children
            for child in &mut root.children[index].children {
                filter_nodes(child, hidden, disabled);
            }
        }
    }

    filter_nodes(&mut layout, cmd_args.hidden, cmd_args.disabled);

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

    let proxy = get_item_menu_proxy(&CONN, &dest, &path)
        .with_context(|| anyhow!("could not get menu proxy from {:?}", cmd_args.path))?;

    proxy.event(cmd_args.id, "clicked", cmd_args.data.as_ref().unwrap_or(&OwnedValue::from(0)), 0)?;

    Ok(())
}

/// Generates help for subcommand list
fn generate_list_help() -> clap::builder::StyledStr {
    cli::Cli::command().find_subcommand_mut("list").unwrap().render_long_help()
}

#[cfg(test)]
mod tests {
    use super::generate_list_help;

    #[test]
    fn generate_list_cmd_help() {
        // ensure there are no issues as this wont be caught during compilation
        assert!(!generate_list_help().to_string().is_empty());
    }
}

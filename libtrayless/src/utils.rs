use crate::{dbus::{dbusmenu::dbusmenuProxyBlocking, status_notifier_item::StatusNotifierItemProxyBlocking, status_notifier_watcher::StatusNotifierWatcherProxyBlocking}, menu_node::MenuNode, tray_item::TrayItem};
use anyhow::{Context, Result, anyhow};

/// Splits path like `:1.298/org/ayatana/NotificationItem/TOA2Xeo1bQ` into
/// destination `:1.298` and path `/org/ayatana/NotificationItem/TOA2Xeo1bQ`
pub fn split_path(input: &str) -> Option<(String, String)> {
    // just find first slash
    let (dest, path) = input.split_once('/')?;

    Some((dest.to_owned(), format!("/{path}")))
}

/// Returns registered tray items
pub fn get_registered_items(conn: &zbus::blocking::Connection) -> Result<Vec<String>> {
    // TODO do i need fallback to org.freedesktop.StatusNotifierWatcher?
    let watcher_proxy = StatusNotifierWatcherProxyBlocking::new(conn)?;
    Ok(watcher_proxy.registered_status_notifier_items()?)
}

/// Get proxy for tray item
pub fn get_item_proxy<'a>(conn: &zbus::blocking::Connection, dest: &str, path: &str) -> Result<StatusNotifierItemProxyBlocking<'a>> {
    Ok(
        StatusNotifierItemProxyBlocking::builder(conn)
            .destination(dest.to_owned())?
            .path(path.to_owned())?
            .build()?
    )
}

pub fn get_item_menu_proxy<'a>(conn: &zbus::blocking::Connection, dest: &str, path: &str) -> Result<dbusmenuProxyBlocking<'a>> {
    Ok(
        dbusmenuProxyBlocking::builder(conn)
            .destination(dest.to_owned())?
            .path(path.to_owned())?
            .build()?
    )
}

pub fn get_item_menu_layout(proxy: &dbusmenuProxyBlocking) -> Result<MenuNode>
{
    let (_, (id, props, children)) = proxy.get_layout(0, -1, &[
        "type", "label", "toggle-type", "toggle-state", "enabled", "visible"
    ])?;

    Ok(MenuNode::new(id, props, children)?)
}

pub fn get_items(conn: &zbus::blocking::Connection, dbus_proxy: &zbus::blocking::fdo::DBusProxy) -> Result<Vec<TrayItem>> {
    get_registered_items(conn)
        .with_context(|| "could not get registered items from StatusNotifierWatcher")?
        .into_iter()
        .map(|item| -> Result<TrayItem> {
            let (dest, path) = split_path(&item)
                .with_context(|| anyhow!("invalid path from registered items {item:?}"))?;

            let proxy = get_item_proxy(conn, &dest, &path)
                .with_context(|| anyhow!("could not get item proxy from {item:?}"))?;

            Ok(TrayItem::from_proxy(dest, path, &proxy, dbus_proxy))
        })
        .collect::<Result<Vec<_>, _>>()
}

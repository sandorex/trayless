use crate::{dbus::{dbusmenu::dbusmenuProxyBlocking, status_notifier_item::StatusNotifierItemProxyBlocking, status_notifier_watcher::StatusNotifierWatcherProxyBlocking}, menu_node::MenuNode};
use anyhow::Result;

/// Splits path like `:1.298/org/ayatana/NotificationItem/TOA2Xeo1bQ` into
/// destination `:1.298` and path `/org/ayatana/NotificationItem/TOA2Xeo1bQ`
pub fn split_path(input: &str) -> Option<(String, String)> {
    // just find first slash
    let (dest, path) = input.split_once('/')?;

    Some((dest.to_owned(), format!("/{path}")))
}

/// Returns registered tray items
pub fn get_registered_items() -> Result<Vec<String>> {
    // TODO do i need fallback to org.freedesktop.StatusNotifierWatcher?
    let watcher_proxy = StatusNotifierWatcherProxyBlocking::new(&crate::CONN)?;
    Ok(watcher_proxy.registered_status_notifier_items()?)
}

/// Get proxy for tray item
pub fn get_item_proxy<'a>(dest: &str, path: &str) -> Result<StatusNotifierItemProxyBlocking<'a>> {
    Ok(
        StatusNotifierItemProxyBlocking::builder(&crate::CONN)
            .destination(dest.to_owned())?
            .path(path.to_owned())?
            .build()?
    )
}

pub fn get_item_menu_proxy<'a>(dest: &str, path: &str) -> Result<dbusmenuProxyBlocking<'a>> {
    Ok(
        dbusmenuProxyBlocking::builder(&crate::CONN)
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

use crate::{MenuNode, dbus::{dbusmenu::dbusmenuProxyBlocking, status_notifier_item::StatusNotifierItemProxyBlocking, status_notifier_watcher::StatusNotifierWatcherProxyBlocking}};
use anyhow::Result;

/// Returns executable of the process
pub fn get_exe_from_pid(pid: u32) -> String {
    match std::fs::read_link(format!("/proc/{pid}/exe")) {
        Ok(x) => x
            .to_string_lossy()
            .to_string(),
        // TODO should this error out?
        Err(_) => "".to_owned(),
    }
}

/// Splits path like `:1.298/org/ayatana/NotificationItem/TOA2Xeo1bQ` into
/// destination `:1.298` and path `/org/ayatana/NotificationItem/TOA2Xeo1bQ`
pub fn split_path(input: &str) -> Option<(String, String)> {
    // just find first slash
    let (dest, path) = input.split_once('/')?;

    Some((dest.to_owned(), format!("/{path}")))
}

/// Returns registered tray items
pub fn get_registered_items() -> Result<Vec<String>> {
    let watcher_proxy = StatusNotifierWatcherProxyBlocking::builder(&crate::CONN)
        .destination("org.kde.StatusNotifierWatcher")? // TODO fallback on org.freedesktop.*
        .path("/StatusNotifierWatcher")?
        .build()?;

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

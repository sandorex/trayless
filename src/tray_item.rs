use zbus::names::BusName;
use crate::{DBUS_PROXY, dbus::status_notifier_item::StatusNotifierItemProxyBlocking};

// TODO skip serializing empty fields, just rework it like MenuNode
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrayItem {
    pub id: String,
    pub title: String,
    // pub tooltip: String, // TODO how do i get them on demand???
    // pub icon_name: String,
    // pub icon_theme_path: String,
    // pub icon_pixmap: String,
    pub item_is_menu: bool,
    pub menu: String,

    pub item: String,
    pub destination: String,
    pub pid: u32,
    pub exe: String,
}

impl TrayItem {
    pub fn from_proxy(destination: String, item: String, proxy: &StatusNotifierItemProxyBlocking) -> Self {
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

            item,
            destination,
            pid,
            exe
        }
    }
}

/// Returns executable of the process
fn get_exe_from_pid(pid: u32) -> String {
    match std::fs::read_link(format!("/proc/{pid}/exe")) {
        Ok(x) => x
            .to_string_lossy()
            .to_string(),
        // TODO should this error out?
        Err(_) => "".to_owned(),
    }
}

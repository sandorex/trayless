use std::fmt::Debug;

use zbus::names::BusName;
use crate::{DBUS_PROXY, dbus::status_notifier_item::StatusNotifierItemProxyBlocking};
use code_docs::{code_docs_struct, DocumentedStruct};

code_docs_struct! {
    #[derive(Clone, serde::Serialize)]
    pub struct TrayItem {
        /// Id of the application
        pub id: String,

        /// DBus `WellKnownName` of the item
        pub name: String,

        /// Relative dbus path to `com.canonical.dbusmenu`
        pub menu: String,

        /// Relative dbus path to `org.kde.StatusNotifierItem` (this object)
        pub item: String,

        /// Process id of the application
        pub pid: u32,

        /// Executable path of the application
        ///
        /// Can be used to deduce the application when it does not define `id` or `name` properly like
        /// electron apps often do
        ///
        /// Will not be defined for flatpak apps (as they use `xdg-dbus-proxy`)
        #[serde(skip_serializing_if = "Option::is_none")]
        pub exe: Option<String>,

        /// Defined if the application is a flatpak
        #[serde(skip_serializing_if = "Option::is_none")]
        pub flatpak_id: Option<String>,

        /// Notifier title specified by the application
        #[serde(skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,

        // NOTE there are many apps using pixmaps for icons so this is useless for rofi or something, so
        // im just not gonna output it
        /// Either a path to the icon or relative to `icon_theme_path`
        /// @skip
        #[serde(skip_serializing)]
        pub icon_name: Option<String>,

        /// Theme where the icon is stored
        /// @skip
        #[serde(skip_serializing)]
        pub icon_theme_path: Option<String>,

        /// The icon in multiple sizes (ARGB32 format)
        /// @skip
        #[serde(skip_serializing)]
        pub icon_pixmap: Option<Vec<(i32, i32, Vec<u8>)>>,
    }
}

// NOTE implementing debug cause of icon_pixmap, its too long and basically fills the terminal
impl Debug for TrayItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let icon_pixmaps = self.icon_pixmap
            .as_ref()
            .map(|x| { x.iter().map(|(w, h, _)| format!("{w}x{h}")).collect::<Vec<_>>() });

        f.debug_struct("TrayItem")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("menu", &self.menu)
            .field("item", &self.item)
            .field("pid", &self.pid)
            .field("exe", &self.exe)
            .field("title", &self.title)
            .field("icon_name", &self.icon_name)
            .field("icon_theme_path", &self.icon_theme_path)
            .field("icon_pixmap", &icon_pixmaps)
            .finish()
    }
}

impl TrayItem {
    pub fn from_proxy(destination: String, item: String, proxy: &StatusNotifierItemProxyBlocking) -> Self {
        let pid = DBUS_PROXY.get_connection_unix_process_id(BusName::try_from(destination.clone()).unwrap())
            .unwrap_or(0);

        let (exe, flatpak_id) = if pid != 0 {
            if let Some(exe) = get_exe_from_pid(pid) {
                (Some(exe), None)
            } else {
                (None, get_flatpak_id_from_pid(pid))
            }
        } else {
            (None, None)
        };

        Self {
            id: proxy.id().unwrap_or_default(),
            title: proxy.title().ok(),
            menu: proxy.menu().map(|x| x.to_string()).unwrap_or_default(),

            icon_name: proxy.icon_name().ok(),
            icon_theme_path: proxy.icon_theme_path().ok(),
            icon_pixmap: proxy.icon_pixmap().ok(),

            item,
            name: destination,
            pid,
            exe,
            flatpak_id,
        }
    }
}

fn parse_cgroup_flatpak(contents: &str) -> Option<String> {
    const PAT: &str = "app-flatpak-";
    let Some(start) = contents.find(PAT) else {
        return None;
    };

    let Some(end) = contents[start..].find(|x: char| x.is_ascii_digit()) else {
        return None;
    };

    // trim leftover dashes
    Some(contents[start + PAT.len()..start + end].trim_end_matches('-').to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_flatpak_cgroup() {
        assert_eq!(parse_cgroup_flatpak("0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-com.discordapp.Discord-884765513.scope"), Some("com.discordapp.Discord".to_owned()));
    }
}

fn get_flatpak_id_from_pid(pid: u32) -> Option<String> {
    match std::fs::read_to_string(format!("/proc/{pid}/cgroup")) {
        Ok(contents) => parse_cgroup_flatpak(&contents),
        Err(_) => None,
    }
}

/// Returns executable of the process
fn get_exe_from_pid(pid: u32) -> Option<String> {
    match std::fs::read_link(format!("/proc/{pid}/exe")) {
        // xdg dbus proxy is not the actual executable
        Ok(x) if x.file_name().unwrap_or_default() == "xdg-dbus-proxy" => None,
        Ok(x) => Some(
            x.to_string_lossy()
             .to_string()
        ),
        Err(_) => None,
    }
}

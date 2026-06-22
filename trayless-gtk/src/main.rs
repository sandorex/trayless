mod cli;
mod window;
mod main_window;
mod menu_window;

use std::sync::LazyLock;
use zbus::blocking::{Connection, fdo::DBusProxy};
use clap::Parser;
use gtk4::prelude::*;

static CONN: LazyLock<Connection> = LazyLock::new(|| {
    Connection::session().expect("could not connect to session bus")
});

static DBUS_PROXY: LazyLock<DBusProxy> = LazyLock::new(|| {
    DBusProxy::new(&CONN).expect("could not open dbus proxy")
});

fn main() {
    let args = cli::Cli::parse();

    // enable inspector
    if args.inspector {
        unsafe { std::env::set_var("GTK_DEBUG", "interactive") };
    }

    let app = gtk4::Application::new(
        Some("com.github.sandorex.trayless-gtk"),
        Default::default(),
    );

    app.connect_activate(move |win| main_window::activate(win, &args));
    app.run_with_args::<String>(&[]); // it parses arguments and breaks
}

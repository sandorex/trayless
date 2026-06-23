use std::path::Path;
use gtk4::{Application, ApplicationWindow, IconTheme, prelude::*};
use gtk4_layer_shell::LayerShell;

pub fn new_window(app: &Application, user_style: Option<&Path>) -> (ApplicationWindow, IconTheme) {
    let gtk_inspector = std::env::var("GTK_DEBUG").is_ok_and(|x| x == "interactive");

    // Create a normal window or ApplicationWindow
    let window = gtk4::ApplicationWindow::new(app);

    if !gtk4_layer_shell::is_supported() {
        eprintln!("Layer shell protocol not supported..\n  More information: https://wayland.app/protocols/wlr-layer-shell-unstable-v1");
    } else {
        window.init_layer_shell();

        // make sure the window is above all other apps
        window.set_layer(gtk4_layer_shell::Layer::Overlay);

        // get all key presses
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);
    }

    let display = gtk4::prelude::RootExt::display(&window);
    let icon_theme = IconTheme::for_display(&display);

    {
        let provider = gtk4::CssProvider::new();

        // load buitlin styling
        provider.load_from_data(include_str!("style.css"));
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION
        );

        // load user style
        if let Some(style_path) = &user_style {
            provider.load_from_path(style_path);

            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER
            );
        }
    }

    window.set_default_size(50, 50);
    window.set_resizable(false);

    // close window on focus loss (not when inspector is running)
    if !gtk_inspector {
        window.connect_is_active_notify(move |win| {
            if !win.is_active() {
                win.close();
            }
        });
    }

    // NOTE im pretty sure these are reference counted, im not actually cloning everything
    (window.clone(), icon_theme.clone())
}

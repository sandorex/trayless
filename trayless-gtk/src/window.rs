use std::path::Path;

// TODO abstract the windows more cause they are very simple layouts

pub fn setup_layer_shell(window: &gtk4::ApplicationWindow) {
    use gtk4_layer_shell::LayerShell;

    window.init_layer_shell();

    // make sure the window is above all other apps
    window.set_layer(gtk4_layer_shell::Layer::Overlay);

    // get all key presses
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);
}

pub fn load_style(display: &gtk4::gdk::Display, user_style: Option<&Path>) {
    let provider = gtk4::CssProvider::new();

    // load buitlin styling
    provider.load_from_data(include_str!("style.css"));
    gtk4::style_context_add_provider_for_display(
        display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    // load user style
    if let Some(style_path) = &user_style {
        provider.load_from_path(style_path);

        gtk4::style_context_add_provider_for_display(
            display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER
        );
    }
}

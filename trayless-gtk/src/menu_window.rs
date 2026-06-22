use std::path::Path;
use gtk4::{IconTheme, gdk::Key, prelude::*};
use libtrayless::{MenuNode, TrayItem};

pub fn activate(app: &gtk4::Application, style_file: Option<&Path>, item: TrayItem, menu: MenuNode) {
    let gtk_inspector = std::env::var("GTK_DEBUG").is_ok_and(|x| x == "interactive");

    // Create a normal window or ApplicationWindow
    let window = gtk4::ApplicationWindow::new(app);

    if !gtk4_layer_shell::is_supported() {
        eprintln!("Layer shell protocol not supported..\n  More information: https://wayland.app/protocols/wlr-layer-shell-unstable-v1");
    } else {
        crate::window::setup_layer_shell(&window);
    }

    let display = gtk4::prelude::RootExt::display(&window);
    let icon_theme = IconTheme::for_display(&display);

    crate::window::load_style(&display, style_file);

    window.add_css_class("menuWindow");
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

    setup_layout(&window, &icon_theme, item, menu);

    // Present the window
    window.present();
}

fn setup_layout(window: &gtk4::ApplicationWindow, _icon_theme: &gtk4::IconTheme, item: TrayItem, mut menu: MenuNode) {
    let box_container = gtk4::Box::new(gtk4::Orientation::Vertical, 10);

    // center horizontally
    box_container.set_halign(gtk4::Align::Center);

    // add margin around the buttons
    const MARGIN: i32 = 10;
    box_container.set_margin_top(MARGIN);
    box_container.set_margin_bottom(MARGIN);
    box_container.set_margin_start(MARGIN);
    box_container.set_margin_end(MARGIN);

    // remove hidden nodes, disabled will be shown but unclickable
    MenuNode::filter_nodes(&mut menu, false, true);

    // TODO icons
    // TODO nested menus?
    for node in menu.children {
        let widget: gtk4::Widget = if let Some(label) = node.label {
            let button = gtk4::Button::new();
            button.set_label(&label);

            if node.enabled.unwrap_or(true) {
                {
                    // TODO activate the menu item
                    let id = node.id.clone();
                    button.connect_clicked(move |_| {
                        println!("clicked id {id}");
                    });
                }
            } else {
                // button is visible but disabled
                button.set_sensitive(false);
            }

            button.upcast()
        } else {
            gtk4::Separator::new(gtk4::Orientation::Horizontal).upcast()
        };

        box_container.append(&widget);
    }

    window.set_child(Some(&box_container));

    let key_controller = gtk4::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);

    {
        let window = window.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            match key {
                Key::Escape => window.close(),
                _ => return false.into(),
            }

            true.into()
        });
    }

    window.add_controller(key_controller);
}

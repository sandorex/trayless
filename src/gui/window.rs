use gtk4::prelude::*;
use gtk4_layer_shell::{Layer, LayerShell};
use crate::cli::{Cli, CmdGui};

fn activate(app: &gtk4::Application) {
    // Create a normal window or ApplicationWindow
    let window = gtk4::ApplicationWindow::new(app);

    if !gtk4_layer_shell::is_supported() {
        eprintln!("Layer shell protocol not supported..\n  More information: https://wayland.app/protocols/wlr-layer-shell-unstable-v1");
    } else {
        window.init_layer_shell();

        // make sure the window is above all other apps
        window.set_layer(Layer::Overlay);

        // get all key presses
        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);
    }

    window.set_default_size(250, 250);
    window.set_resizable(false);

    let box_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    box_container.set_margin_end(10);

    // close window on focus loss
    window.connect_is_active_notify(move |win| {
        if !win.is_active() {
            win.close();
        }
    });

    let items = match crate::get_items() {
        Ok(x) => x,
        // TODO is panic the right thing to do here?
        Err(err) => panic!("{err}"),
    };

    for item in items {
        dbg!(&item);
        match (item.icon_name, item.icon_theme_path, item.icon_pixmap) {
            (Some(name), theme, _) if !name.is_empty() => {
                println!("picture {name}, {theme:?}");
                if let Some(theme) = theme {
                    let pic = gtk4::Picture::for_filename(format!("{theme}/{name}"));
                    box_container.append(&pic);
                } else {
                    let img = gtk4::Image::from_icon_name(&name);
                    box_container.append(&img);
                };
            },
            (None, _, Some(mut pixmaps)) => {
                use gtk4::gdk::{MemoryTexture, MemoryFormat};
                use gtk4::glib::Bytes;

                // TODO choose which pixmap to use
                let (width, height, pixels) = pixmaps.pop().unwrap();

                let rowstride = (width * 4) as usize; 
                let bytes = Bytes::from(&pixels);

                // Create a GPU-friendly texture directly
                let texture = MemoryTexture::new(
                    width,
                    height,
                    MemoryFormat::A8r8g8b8,
                    &bytes,
                    rowstride,
                );

                let pic = gtk4::Image::from_paintable(Some(&texture));

                box_container.append(&pic);
            }
            _ => {
                eprintln!("Error: item {}{} has no icon!", item.name, item.item);
                continue;
            }
        };
    }

    window.set_child(Some(&box_container));

    // Present the window
    window.present();
}

/// Start GUI with args passed to GTK
pub fn start(_args: Cli, _cmd_args: CmdGui) {
    let app = gtk4::Application::new(
        Some("com.github.sandorex.trayless"),
        Default::default(),
    );

    app.connect_activate(activate);
    app.run_with_args::<String>(&[]); // it parses arguments and breaks
}

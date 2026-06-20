use gtk4::{IconTheme, gdk::Key, prelude::*};
use gtk4_layer_shell::{Layer, LayerShell};
use crate::{cli::{Cli, CmdGui}, tray_item::TrayItem};

/// Tries supported extensions on path and returns first successful
fn find_first_image(path: &str) -> Option<String> {
    // TODO these are not confirmed just what google said is supported
    const EXTENSIONS: &[&str] = &[ ".svg", ".png", ".jpg", ".jpeg", ".gif", ".tiff" ];

    for ext in EXTENSIONS {
        let path = format!("{path}{ext}");
        if std::fs::exists(&path).unwrap_or(false) {
            return Some(path);
        }
    }

    None
}

fn widget_from_item(item: TrayItem, icon_theme: &IconTheme) -> Option<gtk4::Picture> {
    let pic = gtk4::Picture::new();

    // flatpak icon if available
    if let Some(flatpak_id) =  &item.flatpak_id {
        let paintable = icon_theme.lookup_icon(
            &flatpak_id,
            &[],
            256,
            1,
            gtk4::TextDirection::None,
            gtk4::IconLookupFlags::empty()
        );

        pic.set_paintable(Some(&paintable));
    } else {
        // TODO use flatpak id as the icon name when available
        match (item.icon_name, item.icon_theme_path, item.icon_pixmap) {
            (Some(name), theme, _) if !name.is_empty() => {
                if let Some(theme) = theme && !theme.is_empty() {
                    let Some(image_file_path) = find_first_image(&format!("{theme}/{name}")) else {
                        // TODO this should be a different error
                        return None;
                    };

                    pic.set_filename(Some(&image_file_path));
                } else {
                    let paintable = icon_theme.lookup_icon(&name, &[], 256, 1, gtk4::TextDirection::None, gtk4::IconLookupFlags::empty());
                    pic.set_paintable(Some(&paintable));
                }
            },
            (_, _, Some(mut pixmaps)) => {
                use gtk4::gdk::{MemoryTexture, MemoryFormat};
                use gtk4::glib::Bytes;

                assert!(pixmaps.len() > 0, "empty icon_pixmap");

                // TODO choose which pixmap to use
                let (width, height, pixels) = pixmaps.pop().unwrap();

                assert!(pixels.len() > 0, "no pixels in a pixmap");

                let rowstride = (width * 4) as usize;
                let bytes = Bytes::from(&pixels);

                let texture = MemoryTexture::new(
                    width,
                    height,
                    MemoryFormat::A8r8g8b8,
                    &bytes,
                    rowstride,
                );

                pic.set_paintable(Some(&texture));
            },
            _ => return None,
        }
    }

    Some(pic)
}

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

    let display = gtk4::prelude::RootExt::display(&window);
    let icon_theme = IconTheme::for_display(&display);

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

    let key_controller = gtk4::EventControllerKey::new();

    {
        let window = window.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            match key {
                Key::Escape => window.close(),
                Key::Tab => {
                },
                Key::Up => println!("up"),
                Key::Down => println!("down"),
                Key::Left => println!("left"),
                Key::Right => println!("right"),
                _ => {}
            }

            true.into()
        });
    }

    window.add_controller(key_controller);

    let items = match crate::get_items() {
        Ok(x) => x,
        // TODO is panic the right thing to do here?
        Err(err) => panic!("{err}"),
    };

    for item in items {
        #[cfg(debug_assertions)]
        dbg!(&item);

        let item_path = format!("{}{}", item.name, item.item);

        let Some(img) = widget_from_item(item, &icon_theme) else {
            eprintln!("Error: item {item_path} has no icon!");
            continue;
        };

        img.set_size_request(128, 128);
        img.set_margin_start(15);
        img.set_margin_end(15);

        box_container.append(&img);
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

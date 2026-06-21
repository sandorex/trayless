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

fn widget_from_item(item: &TrayItem, icon_theme: &IconTheme) -> Option<gtk4::Picture> {
    let pic = gtk4::Picture::new();

    match (&item.icon_name, &item.icon_theme_path, &item.icon_pixmap) {
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
        (_, _, Some(pixmaps)) => {
            use gtk4::gdk::{MemoryTexture, MemoryFormat};
            use gtk4::glib::Bytes;

            assert!(pixmaps.len() > 0, "empty icon_pixmap");

            // TODO choose which pixmap to use
            let (width, height, pixels) = pixmaps.last().cloned().unwrap();

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

    Some(pic)
}

// TODO reopen the window if the some tray icon gets added/removed
// TODO no item is visibly focused on startup
fn activate(app: &gtk4::Application, _args: &Cli, _cmd_args: &CmdGui) {
    let gtk_inspector = std::env::var("GTK_DEBUG").is_ok_and(|x| x == "interactive");

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

    // bundle css styling
    {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("window.css"));
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION
        );
    }

    window.set_default_size(100, 100);
    window.set_resizable(false);

    let box_container = gtk4::Box::new(gtk4::Orientation::Vertical, 10);

    // center horizontally
    box_container.set_halign(gtk4::Align::Center);

    // add margin around the buttons
    const MARGIN: i32 = 10;
    box_container.set_margin_top(MARGIN);
    box_container.set_margin_bottom(MARGIN);
    box_container.set_margin_start(MARGIN);
    box_container.set_margin_end(MARGIN);

    // close window on focus loss (not when inspector is running)
    if !gtk_inspector {
        window.connect_is_active_notify(move |win| {
            if !win.is_active() {
                win.close();
            }
        });
    }

    let key_controller = gtk4::EventControllerKey::new();

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

    let items = match crate::get_items() {
        Ok(x) => x,
        // TODO is panic the right thing to do here?
        Err(err) => panic!("{err}"),
    };

    // TODO this could be a message in the window
    if items.is_empty() {
        panic!("No tray items");
    }

    for item in items.into_iter() {
        // print each item in debug builds
        if cfg!(debug_assertions) {
            dbg!(&item);
        }

        let title = match (&item.id, &item.title, &item.tooltip_title) {
            // prefer title
            (_, Some(title), _) if !title.is_empty() => title,

            // fallback to tooltip title
            (_, _, Some(t_title)) => t_title,

            // fallback to id
            (id, _, _) => id,
        };

        let label = gtk4::Label::new(Some(title));

        // TODO use placeholder icon when there is none
        let Some(img) = widget_from_item(&item, &icon_theme) else {
            eprintln!("Error: item {}{} has no icon!", item.name, item.item);
            continue;
        };

        img.set_size_request(32, 32);

        let btn_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 15);
        btn_container.append(&img);
        btn_container.append(&label);

        let btn = gtk4::Button::new();
        btn.set_child(Some(&btn_container));

        {
            let window = window.clone();
            btn.connect_clicked(move |_| {
                println!("clicked: {:?}", item);
                window.close();
            });
        }

        box_container.append(&btn);
    }

    window.set_child(Some(&box_container));

    // Present the window
    window.present();

    // focus first entry
    box_container.first_child().unwrap().grab_focus();
}

/// Start GUI with args passed to GTK
pub fn start(args: Cli, cmd_args: CmdGui) {
    let app = gtk4::Application::new(
        Some("com.github.sandorex.trayless"),
        Default::default(),
    );

    // enable inspector
    if cmd_args.inspector {
        unsafe { std::env::set_var("GTK_DEBUG", "interactive") };
    }

    app.connect_activate(move |win| activate(win, &args, &cmd_args));
    app.run_with_args::<String>(&[]); // it parses arguments and breaks
}

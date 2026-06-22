use gtk4::{IconTheme, gdk::Key, prelude::*};
use libtrayless::{TrayItem, get_item_menu_proxy, get_item_proxy, get_items, get_item_menu_layout};
use anyhow::{Result, anyhow};

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

fn widget_from_item(item: &TrayItem, icon_theme: &IconTheme) -> Result<gtk4::Picture> {
    let pic = gtk4::Picture::new();

    match (&item.icon_name, &item.icon_theme_path, &item.icon_pixmap) {
        (Some(name), theme, _) if !name.is_empty() => {
            if let Some(theme) = theme && !theme.is_empty() {
                let Some(image_file_path) = find_first_image(&format!("{theme}/{name}")) else {
                    return Err(anyhow!("icon of item {} was not found ('{theme}/{name}')", item.name));
                };

                pic.set_filename(Some(&image_file_path));
            } else {
                let paintable = icon_theme.lookup_icon(&name, &[], 64, 1, gtk4::TextDirection::None, gtk4::IconLookupFlags::empty());
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
        _ => return Err(anyhow!("item {}{} has no icon!", item.name, item.item)),
    };

    Ok(pic)
}

// TODO reopen the window if the some tray icon gets added/removed
pub fn activate(app: &gtk4::Application, args: &crate::cli::Cli) {
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

    crate::window::load_style(&display, args.style.as_deref());

    window.add_css_class("mainWindow");
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

    setup_layout(&app, &window, &icon_theme);

    // Present the window
    window.present();
}

fn setup_layout(app: &gtk4::Application, window: &gtk4::ApplicationWindow, icon_theme: &gtk4::IconTheme) {
    let box_container = gtk4::Box::new(gtk4::Orientation::Vertical, 10);

    // center horizontally
    box_container.set_halign(gtk4::Align::Center);

    // add margin around the buttons
    const MARGIN: i32 = 10;
    box_container.set_margin_top(MARGIN);
    box_container.set_margin_bottom(MARGIN);
    box_container.set_margin_start(MARGIN);
    box_container.set_margin_end(MARGIN);

    let items = match get_items(&crate::CONN, &crate::DBUS_PROXY) {
        Ok(x) => x,
        // TODO is panic the right thing to do here?
        Err(err) => panic!("{err}"),
    };

    // TODO this could be a message in the window
    if items.is_empty() {
        panic!("No tray items");
    }

    for (i, item) in items.iter().enumerate() {
        let title = match (&item.id, &item.title, &item.tooltip_title) {
            // prefer title
            (_, Some(title), _) if !title.is_empty() => title,

            // fallback to tooltip title
            (_, _, Some(t_title)) => t_title,

            // fallback to id
            (id, _, _) => id,
        };

        let label = gtk4::Label::new(Some(title));

        let img = match widget_from_item(&item, &icon_theme) {
            Ok(img) => img,
            Err(err) => {
                eprintln!("Error: {err}");

                let img = gtk4::Picture::new();

                // load missing icon
                let paintable = icon_theme.lookup_icon("image-missing", &[], 64, 1, gtk4::TextDirection::None, gtk4::IconLookupFlags::empty());
                img.set_paintable(Some(&paintable));

                img
            },
        };

        img.set_size_request(32, 32);

        let btn_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 15);
        btn_container.append(&img);
        btn_container.append(&label);

        let btn = gtk4::Button::new();
        btn.set_child(Some(&btn_container));
        unsafe { btn.set_data("index", i) };

        box_container.append(&btn);
    }

    window.set_child(Some(&box_container));

    let key_controller = gtk4::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);

    {
        let window = window.clone();
        let app = app.clone();
        let box_container = box_container.clone();
        key_controller.connect_key_pressed(move |_, key, _, mod_type| {
            match key {
                Key::Escape => window.close(),
                Key::Return | Key::KP_Enter => {
                    // get focused child
                    let Some(child) = box_container.focus_child() else {
                        eprintln!("error could not get focused child");
                        return true.into();
                    };

                    let Some(index) = (unsafe { child.data::<usize>("index") }) else {
                        eprintln!("error could not read index from button");
                        return true.into();
                    };

                    let index = unsafe { *index.as_ptr() };

                    window.close();

                    let item = &items[index];

                    if mod_type.contains(gtk4::gdk::ModifierType::SHIFT_MASK) {
                        match get_item_menu_proxy(&crate::CONN, &item.name, &item.menu) {
                            Ok(proxy) => {
                                match get_item_menu_layout(&proxy) {
                                    Ok(layout) => {
                                        // NOTE make sure window is closed before opening a new one
                                        crate::menu_window::activate(&app, None, item.clone(), layout);
                                    },
                                    Err(err) => eprintln!("Error: {err}")
                                };
                            },
                            Err(err) => eprintln!("Error: {err}")
                        };
                    } else {
                        match get_item_proxy(&crate::CONN, &item.name, &item.item) {
                            Ok(proxy) => {
                                // TODO call ContextMenu if ItemIsMenu
                                match proxy.activate(0, 0) {
                                    Ok(_) => {},
                                    Err(err) => eprintln!("Error: {err}")
                                }
                            },
                            Err(err) => eprintln!("Error: {err}")
                        };
                    }
                },
                _ => return false.into(),
            }

            true.into()
        });
    }

    window.add_controller(key_controller);
}

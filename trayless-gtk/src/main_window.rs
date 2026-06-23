use std::rc::Rc;
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
    let (window, icon_theme) = crate::window::new_window(&app, args.style.as_deref());

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
        Ok(x) => Rc::new(x),
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
            (_, _, Some(t_title)) if !t_title.is_empty() => t_title,

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

        let action_menu = {
            let index = i;
            let app = app.clone();
            let items = Rc::clone(&items);
            Rc::new(move || {
                let item = &items[index];

                match get_item_menu_proxy(&crate::CONN, &item.name, &item.menu) {
                    Ok(proxy) => {
                        match get_item_menu_layout(&proxy) {
                            Ok(layout) => {
                                crate::menu_window::activate(&app, None, item.clone(), layout);
                            },
                            Err(err) => eprintln!("Error: {err}")
                        };
                    },
                    Err(err) => eprintln!("Error: {err}")
                };
            })
        };

        let action_activate = {
            let index = i;
            let items = Rc::clone(&items);
            Rc::new(move || {
                let item = &items[index];

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
            })
        };

        {
            let key_controller = gtk4::EventControllerKey::new();

            let action_menu = Rc::clone(&action_menu);
            let action_activate = Rc::clone(&action_activate);
            key_controller.connect_key_pressed(move |_, key, _, mod_type| {
                match key {
                    Key::Return | Key::KP_Enter => {
                        if mod_type.contains(gtk4::gdk::ModifierType::SHIFT_MASK) {
                            action_activate();
                        } else {
                            action_menu();
                        }
                    },
                    _ => return false.into(),
                }

                true.into()
            });

            btn.add_controller(key_controller);
        }

        {
            let controller = gtk4::GestureClick::new();
            controller.set_button(0); // capture all mouse clicks

            controller.connect_pressed(move |gesture, _, _, _| {
                match gesture.current_button() {
                    gtk4::gdk::BUTTON_PRIMARY => action_menu(),
                    gtk4::gdk::BUTTON_SECONDARY => action_activate(),
                    _ => { return; }
                }

                gesture.set_state(gtk4::EventSequenceState::Claimed);
            });

            btn.add_controller(controller);
        }

        box_container.append(&btn);
    }

    window.set_child(Some(&box_container));
}

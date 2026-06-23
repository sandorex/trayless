use std::{path::Path, rc::Rc};
use gtk4::{ApplicationWindow, gdk::Key, prelude::*};
use libtrayless::{MenuNode, TrayItem, get_item_menu_proxy};
use zbus::zvariant::OwnedValue;

pub fn activate(app: &gtk4::Application, style_file: Option<&Path>, item: TrayItem, menu: MenuNode) {
    let (window, _) = crate::window::new_window(&app, style_file.as_deref());

    setup_layout(&window, item, menu);

    // Present the window
    window.present();
}

fn setup_layout(window: &ApplicationWindow, item: TrayItem, mut menu: MenuNode) {
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

    let item = Rc::new(item);

    // TODO icons
    // TODO nested menus?
    // TODO toggle types
    for node in menu.children {
        let widget: gtk4::Widget = if let Some(label) = node.label {
            if !node.children.is_empty() {
                eprintln!("Nested menu items are not supported at the moment");
                continue
            }

            let button = gtk4::Button::new();
            button.set_label(&label);

            if node.enabled.unwrap_or(true) {
                let action_event = {
                    let window = window.clone();
                    let item = Rc::clone(&item);
                    let id = node.id;
                    Rc::new(move || {
                        match get_item_menu_proxy(&crate::CONN, &item.name, &item.menu) {
                            Ok(proxy) => {
                                match proxy.event(id, "clicked", &OwnedValue::from(0), 0) {
                                    Ok(_) => {},
                                    Err(err) => eprintln!("Error: {err}"),
                                }
                            },
                            Err(err) => eprintln!("Error: {err}"),
                        }

                        window.destroy();
                    })
                };


                {
                    let key_controller = gtk4::EventControllerKey::new();

                    let action_event = Rc::clone(&action_event);
                    // let window = window.clone();
                    key_controller.connect_key_pressed(move |_, key, _, _| {
                        match key {
                            Key::Return | Key::KP_Enter => {
                                action_event();
                            },
                            _ => return false.into(),
                        }

                        true.into()
                    });

                    button.add_controller(key_controller);
                }

                {
                    let controller = gtk4::GestureClick::new();
                    controller.set_button(0); // capture all mouse clicks

                    controller.connect_pressed(move |gesture, _, _, _| {
                        // let widget = gesture.widget().unwrap();
                        // let data = unsafe { widget.data::<ButtonData>(DATA_KEY).unwrap().as_ref() };

                        // i have no use for middle or right click
                        match gesture.current_button() {
                            gtk4::gdk::BUTTON_PRIMARY => {
                                gesture.set_state(gtk4::EventSequenceState::Claimed);
                                action_event();
                            },

                            _ => {}
                        }

                        gesture.set_state(gtk4::EventSequenceState::None);
                    });
                    button.add_controller(controller);
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
}

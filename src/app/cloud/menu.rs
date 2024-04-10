use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;

use crate::app::{
    action_bar::ActionBar,
    button::Button,
    cloud::{Actions, ListGlobalState},
    confirm::ConfirmVisible,
    dialog::DialogVisible,
    list_wrap_display_status::ScrollAction,
};

#[derive(Props, Clone, PartialEq)]
pub struct MenuProps {
    visible: Signal<DialogVisible>,
    actions: Signal<Option<(Vec<Actions>, String, String)>>,
    onaction: EventHandler<(Actions, String)>,
}

pub fn Menu(mut props: MenuProps) -> Element {
    let mut confirm_visible = use_context::<Signal<ConfirmVisible>>();
    // cloud state
    let ListGlobalState {
        mut menu_list_state,
        ..
    } = use_context::<ListGlobalState>();

    let is_pending = use_memo(move || {
        !props.visible.read().visible() || confirm_visible.read().dialog.read().is_show()
    });

    let mut do_action = move || {
        props.actions.read().as_ref().map(|actions| {
            let actions = actions.clone();
            let idx = menu_list_state.read().selected_idx;
            if let Some(action) = actions.0.get(idx as usize).map(|&a| a.clone()) {
                let tips = match action {
                    Actions::NewDir => {
                        format!("{}?", action)
                    }
                    _ => format!("{} {}?", action, actions.1),
                };
                confirm_visible.write().show(
                    tips,
                    Rc::new(RefCell::new(Box::new(move || {
                        props.onaction.call((action, actions.1.clone()));
                    }))),
                );
            }
        });
    };

    rsx! {
        div {
            "scale": 0.38,
            flex: 1,
            display: "flex",
            flex_direction: "column",
            position: "relative",
            onkeypress: move |e| {
                if is_pending() {
                    return;
                }
                match e.data.code() {
                    Code::ArrowUp => {
                        props.actions.read().as_ref().map(|(actions, _, _)| {
                            menu_list_state.write().do_scroll(actions.len() as i32, ScrollAction::Up);
                        });
                    }
                    Code::ArrowDown => {
                        props.actions.read().as_ref().map(|(actions, _, _)| {
                            menu_list_state.write().do_scroll(actions.len() as i32, ScrollAction::Down);
                        });
                    }
                    Code::KeyA => {
                        do_action();
                    }
                    Code::KeyB => {
                        props.visible.write().hide();
                    }
                    _ => {}
                }
            },

            div {
                flex: 1,
                display: "flex",
                flex_direction: "column",
                padding: 5.0,
                padding_top: 0.0,
                margin_top: 4.0,

                if let Some((actions, _, info)) = props.actions.read().as_ref().map(|l| l.clone()) {
                    div {
                        display: "flex",
                        height: 20.0,
                        align_items: "center",
                        justify_content: "center",
                        padding_top: 2.0,
                        padding_left: 5.0,
                        padding_right: 5.0,
                        margin_bottom: 5.0,
                        background_color: "selected_bg",

                        "{info}"
                    }

                    for (idx, action) in actions.iter().enumerate() {
                        div {
                            height: 20.0,
                            padding: 1,
                            background_color: if menu_list_state.read().selected_idx == idx as i32 {
                                "green"
                            } else {
                                "main_bg"
                            },

                            Button {
                                display: "flex",
                                height: 18.0,
                                align_items: "center",
                                padding_left: 5.0,
                                padding_right: 5.0,
                                bg_color: "main_bg",
                                bg_active_color: "selected_bg",
                                onclick: move |_| {
                                    menu_list_state.write().set_selected_idx(idx as i32);
                                    do_action();
                                },
                                "{&action}"
                            }
                        }
                    }
                }
            }

            ActionBar {
                tips: "(B) 关闭   (A) 选择"
            }
        }
    }
}

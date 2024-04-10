use std::{cell::RefCell, rc::Rc, time::Duration};

use dioxus::prelude::*;

use crate::{
    app::{button::Button, line::Line},
    constant::{SCREEN_BOTTOM_WIDTH, SCREEN_HEIGHT, SELECTED_BG_COLOR},
};

use super::dialog::{use_dialog, DialogVisible};

pub struct ConfirmState {
    title: String,
    qrcode: Option<String>,
    on_confirm: Rc<RefCell<Box<dyn FnMut()>>>,
}

#[derive(Props, Clone, PartialEq)]
pub struct ConfirmProps {
    visible: Signal<ConfirmVisible>,
}

#[derive(Clone, PartialEq)]
pub struct ConfirmVisible {
    pub dialog: Signal<DialogVisible>,
    pub confirm: Signal<ConfirmState>,
    pub action_fired: Signal<bool>,
}

impl ConfirmVisible {
    pub fn show(&mut self, title: String, on_confirm: Rc<RefCell<Box<dyn FnMut()>>>) {
        self.dialog.write().show();
        self.confirm.with_mut(|confirm| {
            confirm.title = title;
            confirm.qrcode = None;
            confirm.on_confirm = on_confirm;
        });
        *self.action_fired.write() = false;
    }

    pub fn show_qrcode(
        &mut self,
        title: String,
        qrcode: String,
        on_confirm: Rc<RefCell<Box<dyn FnMut()>>>,
    ) {
        self.dialog.write().show();
        self.confirm.with_mut(|confirm| {
            confirm.title = title;
            confirm.qrcode = Some(qrcode);
            confirm.on_confirm = on_confirm;
        });
        *self.action_fired.write() = false;
    }

    pub fn hide(&mut self) {
        self.dialog.write().hide();
    }
}

pub fn use_confirm() -> Signal<ConfirmVisible> {
    let visible = use_dialog(
        false,
        SCREEN_HEIGHT as f64,
        0.0,
        SCREEN_BOTTOM_WIDTH as f64,
        SCREEN_HEIGHT as f64,
        Some(Duration::from_millis(200)),
    );
    let res = ConfirmVisible {
        dialog: visible,
        action_fired: use_signal(|| false),
        confirm: use_signal(|| ConfirmState {
            title: "".to_string(),
            qrcode: None,
            on_confirm: Rc::new(RefCell::new(Box::new(|| {}))),
        }),
    };
    use_signal(move || res)
}

pub fn Confirm(mut props: ConfirmProps) -> Element {
    let mut cancel = move || {
        if *props.visible.read().action_fired.read() {
            return;
        }
        *props.visible.write().action_fired.write() = true;
        props.visible.write().dialog.write().hide();
    };

    let mut confirm = move || {
        if *props.visible.read().action_fired.read() {
            return;
        }
        *props.visible.write().action_fired.write() = true;
        props.visible.write().dialog.write().hide();
        {
            let mut confirm = props.visible.peek().confirm;
            let f = confirm.write().on_confirm.clone();
            f.borrow_mut()();
        }
    };

    rsx! {
        div {
            flex: 1,
            display: "flex",
            justify_content: "center",
            align_items: "center",

            div {
                display: "flex",
                flex_direction: "column",
                width: 260.0,
                height: 110.0,
                background_color: "selected_bg",
                onkeypress: move |e| {
                    if e.data.code() == Code::KeyA {
                        confirm();
                    } else if e.data.code() == Code::KeyB {
                        cancel();
                    }
                },

                div {
                    flex: 1,
                    display: "flex",
                    justify_content: "center",
                    align_items: "center",
                    max_width: 240.0,

                    "{props.visible.read().confirm.read().title}"
                }

                if let Some(qrcode) = props.visible.read().confirm.read().qrcode.as_ref().map(|c| c.clone()) {
                    div {
                        display: "flex",
                        justify_content: "center",
                        align_items: "center",
                        padding_bottom: 5.0,

                        img {
                            "scale": 0.46875,
                            "media": "qrcode",
                            src: qrcode,
                            width: 60.0,
                            height: 60.0,
                        }
                    }
                }

                Line {}

                div {
                    display: "flex",
                    align_items: "center",
                    height: 20.0,

                    Button {
                        flex: 1,
                        height: 20.0,
                        display: "flex",
                        align_items: "center",
                        justify_content: "center",
                        padding_top: 1.0,
                        onclick: move |_| {
                            cancel();
                        },

                        "(B) 取消"
                    }

                    div {
                        height: 20.0,
                        width: 1.0,
                        background_color: SELECTED_BG_COLOR,
                    }

                    Button {
                        flex: 1,
                        height: 20.0,
                        display: "flex",
                        align_items: "center",
                        justify_content: "center",
                        padding_top: 1.0,
                        onclick: move |_| {
                            confirm();
                        },
                        "(A) 确定"
                    }
                }
            }
        }
    }
}

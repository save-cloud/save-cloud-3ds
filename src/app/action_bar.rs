#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::app::line::Line;

#[derive(Props, Clone, PartialEq)]
pub struct ActionBarProps {
    #[props(default = false)]
    pub version: bool,
    pub tips: String,
    pub onkeypress: Option<EventHandler<KeyboardEvent>>,
}

pub fn ActionBar(props: ActionBarProps) -> Element {
    let ActionBarProps {
        tips,
        version,
        onkeypress,
    } = props;
    rsx! {
        Line {}

        div {
            display: "flex",
            justify_content: "space-between",
            align_items: "center",
            padding_right: 10,
            padding_left: 5,
            height: 20,
            color: "main-text",
            onkeypress: move |e| {
                onkeypress.as_ref().map(|h| h.call(e));
            },

            if version {
                div {
                    "scale": 0.3,
                    margin_top: 2,
                    "v2024.05.10"
                }
            }

            div {
                "scale": 0.38,
                margin_top: 2,

                {tips}
            }
        }
    }
}

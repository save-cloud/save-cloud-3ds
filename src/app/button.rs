use dioxus::prelude::*;

use crate::utils::sleep_micros;

#[derive(Props, Clone, PartialEq)]
pub struct ButtonProps {
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    #[props(default = String::from("selected_bg"))]
    pub bg_color: String,
    #[props(default = String::from("selected_bg_light"))]
    pub bg_active_color: String,
    pub onmousedown: Option<EventHandler<MouseEvent>>,
    pub onmouseup: Option<EventHandler<MouseEvent>>,
    pub onclick: Option<EventHandler<MouseEvent>>,
    pub children: Element,
}

#[component]
pub fn Button(props: ButtonProps) -> Element {
    let mut active = use_signal(|| false);

    rsx! {
        div {
            background_color: if *active.read() {
                props.bg_active_color
            } else {
                props.bg_color
            },
            onmousedown: move |e| {
                active.set(true);
                props.onmousedown.as_ref().map(|h| h.call(e));
            },
            onmouseup: move |e| {
                spawn(async move {
                    sleep_micros(100000).await;
                    active.set(false);
                });
                props.onmouseup.as_ref().map(|h| h.call(e));
            },
            onclick: move |e| {
                props.onclick.as_ref().map(|h| h.call(e));
            },
            ..props.attributes,

            {props.children}
        }
    }
}

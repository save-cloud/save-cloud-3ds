use dioxus::prelude::*;

pub fn NoData() -> Element {
    rsx! {
        div {
            flex: 1,
            display: "flex",
            justify_content: "center",
            align_items: "center",
            padding_right: 80.0,

            img {
                "scale": 1.0,
                src: 6,
                width: 48,
                height: 48,
            }
        }
    }
}

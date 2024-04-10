use dioxus::prelude::*;

use crate::constant::SELECTED_BG_COLOR;

pub fn Line() -> Element {
    rsx! {
        div {
            height: 1,
            background_color: SELECTED_BG_COLOR,
        }
    }
}

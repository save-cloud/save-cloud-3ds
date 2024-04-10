use dioxus::prelude::*;

use crate::{
    app::{
        list_wrap_display_status::{ListDisplayStatus, ScrollAction},
        titles::title_selected::TitleSelected,
    },
    resource::TitleInfo,
};

#[component]
pub fn TitleList(is_pending: bool) -> Element {
    let mut title_selected = use_context::<Signal<Option<TitleSelected>>>();
    let titles = use_context::<Signal<Vec<TitleInfo>>>();
    let titles_list_loading_percent = use_context::<SyncSignal<f64>>();
    let mut title_list_display = use_context::<Signal<ListDisplayStatus>>();
    let percent = format!("{:.0}", *titles_list_loading_percent.read());

    if *titles_list_loading_percent.read() < 100.0 {
        return rsx! {
            div {
                "deep_3d": -2.0,
                "scale": 0.9,
                flex: 1,
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                justify_content: "center",
                padding_bottom: 20.0,

                img {
                    "scale": 1,
                    src: 5,
                    width: 128.0,
                    height: 128.0,
                    margin_bottom: 20.0,
                }

                "正在加载 {percent}%"
            }
        };
    }

    let selected_idx = title_list_display.read().selected_idx;
    let top_row = title_list_display.read().top_row;
    rsx! {
        div {
            flex: 1,
            display: "flex",
            flex_wrap: "wrap",
            align_items: "flex-start",
            align_content: "flex-start",
            onkeypress: move |e| {
                if is_pending {
                    return;
                }
                let size = titles.read().len();
                if let Some(action) = match e.data().code() {
                    Code::ArrowLeft => Some(ScrollAction::Left),
                    Code::ArrowRight => Some(ScrollAction::Right),
                    Code::ArrowUp => Some(ScrollAction::Up),
                    Code::ArrowDown => Some(ScrollAction::Down),
                    _ => None
                } {
                    title_list_display.write().do_action(size, action);
                    // selected title
                    if let Some(&title) = titles.read().get(title_list_display.read().selected_idx) {
                        spawn(async move {
                            // replace new title
                            title_selected.write().replace(TitleSelected::new(title));
                        });
                    }
                }
            },

            for (idx, title) in titles.read().iter().enumerate().filter(|&(idx, _)| {
                if idx >= title_list_display.read().top_row * 8 && idx <= (title_list_display.read().top_row + 5) * 8{
                    return true
                }
                false
            }).take(40) {
                div {
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    width: 50,
                    height: 48,
                    position: "relative",

                    if is_pending && idx == selected_idx {
                        div {
                            "deep_3d": -1.0,
                            position: "absolute",
                            left: if selected_idx % 8 == 0 { 1 } else if selected_idx % 8 == 7 { -3 } else { -1 },
                            top: if selected_idx / 8 == top_row { 0 } else { -4 },
                            width: 52,
                            height: 52,
                            background_color: "green",
                            z_index: 0.5,
                        }

                        img {
                            "deep_3d": -1.0,
                            position: "absolute",
                            left: if selected_idx % 8 == 0 { 3 } else if selected_idx % 8 == 7 { -1 } else { 1 },
                            top: if selected_idx / 8 == top_row { 2 } else { -2 },
                            "scale": 1,
                            "media": title.media_str(),
                            src: "{title.id}",
                            width: 48,
                            height: 48,
                            z_index: 0.5,
                        }
                    } else {
                        if idx == selected_idx {
                            div {
                                "deep_3d": if idx == selected_idx { 1.0 } else { 2.0 },
                                position: "absolute",
                                left: 1,
                                top: 0,
                                width: 48,
                                height: 48,
                                background_color: "green",
                                z_index: 0.5,
                            }
                        }

                        img {
                            "deep_3d": if idx == selected_idx { 1.0 } else { 2.0 },
                            "scale": 0.916,
                            "media": title.media_str(),
                            src: "{title.id}",
                            width: 44,
                            height: 44,
                            z_index: if idx == selected_idx { 0.5 } else { 0.0 },
                        }
                    }
                }
            }
        }
    }
}

#![allow(non_snake_case)]

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use ctru::services::fs::MediaType;
use dioxus::prelude::*;

pub mod action_bar;
pub mod auth;
pub mod button;
pub mod cloud;
pub mod confirm;
pub mod dialog;
pub mod line;
pub mod list_display_status;
pub mod list_wrap_display_status;
pub mod loading;
pub mod no_data;
pub mod tips;
pub mod titles;
pub mod top_bar;

use crate::{
    api::Api,
    app::{
        cloud::{use_list_global_state, Cloud},
        confirm::use_confirm,
        list_wrap_display_status::ListDisplayStatus,
        loading::use_page_loading,
        tips::use_tips,
        titles::{title_selected::TitleSelected, Titles},
    },
    constant::{HOME_LOCAL_PATH_CACHE, SCREEN_HEIGHT},
    platform::get_title_list,
    resource::TitleInfo,
    utils::join_path,
};

/// u8: 0: not exit, 1: exit, 2: exit and call fbi
#[derive(Clone)]
pub struct AppExit(pub Arc<Mutex<(u64, Option<(MediaType, String)>)>>);

impl AppExit {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new((0, None))))
    }

    pub fn set_exit(&self) {
        *self.0.lock().unwrap() = (1, None);
    }

    pub fn inner_value(&self) -> (u64, Option<(MediaType, String)>) {
        self.0.lock().unwrap().clone()
    }

    pub fn is_exit(&self) -> bool {
        self.0.lock().unwrap().0 > 0
    }
}

pub struct AuthState(pub bool);

#[derive(Clone, Copy, PartialEq)]
pub enum Panel {
    Device,
    Cloud,
}

pub fn Main() -> Element {
    // device or cloud panel
    use_context_provider(|| {
        if Path::new(&join_path(HOME_LOCAL_PATH_CACHE, "recovery")).exists() {
            Signal::new(Panel::Cloud)
        } else {
            Signal::new(Panel::Device)
        }
    });
    // selected title
    use_context_provider::<Signal<Option<TitleSelected>>>(|| Signal::new(None));
    // title list
    use_context_provider(|| Signal::new(Vec::new() as Vec<TitleInfo>));
    // title list display status
    use_context_provider(|| Signal::new(ListDisplayStatus::new(8, 5)));
    // auth
    let auth_state = use_signal_sync(|| AuthState(Api::get_read().is_login()));
    use_context_provider(move || auth_state);
    // tips
    let tips_visible = use_tips(false, SCREEN_HEIGHT as f64, 190.0);
    use_context_provider(move || tips_visible);
    // page loading
    let page_loading_visible = use_page_loading(false, SCREEN_HEIGHT as f64, 174.0);
    use_context_provider(move || page_loading_visible);
    let confirm_visible = use_confirm();
    use_context_provider(move || confirm_visible);
    // title loading process
    let mut percent = use_signal_sync(|| 0.0f64);
    use_context_provider(|| percent);

    // cloud
    let list_global_state = use_list_global_state(auth_state.peek().0);
    use_context_provider(move || list_global_state);

    let selected_panel = use_context::<Signal<Panel>>();
    let mut title_list = use_context::<Signal<Vec<TitleInfo>>>();
    let mut title_selected = use_context::<Signal<Option<TitleSelected>>>();

    // loading title list
    use_future(move || async move {
        if let Ok(titles) = get_title_list(percent).await {
            // selected title
            if let Some(&title_info) = titles.first() {
                spawn(async move {
                    title_selected
                        .write()
                        .replace(TitleSelected::new(title_info));
                });
            }
            *title_list.write() = titles;
            percent.set(100.0);
        };
    });

    rsx! {
        div {
            "scale": 0.4,
            display: "block",
            width: 400,
            height: 240,
            color: "main-text",
            z_index: 0,

            if *selected_panel.read() == Panel::Device {
                Titles {}
            } else {
                Cloud {}
            }
        }
    }
}

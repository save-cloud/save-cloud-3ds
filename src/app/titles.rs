#![allow(non_snake_case)]

use std::{
    cell::RefCell,
    error::Error,
    fmt::{self, Display, Formatter},
    ops::Deref,
    rc::Rc,
};

use dioxus::prelude::*;

use crate::{
    api::Api,
    app::{
        action_bar::ActionBar,
        button::Button,
        confirm::{Confirm, ConfirmVisible},
        dialog::{use_dialog, Dialog},
        loading::{PageLoading, PageLoadingVisible},
        tips::{Tips, TipsVisible},
        titles::{
            menu::{
                backup_game_save_to_cloud, backup_game_save_to_local, get_game_local_backup_path,
                Menu,
            },
            title_list::TitleList,
            title_selected::{SaveTypes, TitleSaveTypes, TitleSelected},
        },
        top_bar::NavBar,
        AppExit, AuthState,
    },
    constant::{
        ABOUT_TEXT, HOME_PAGE_URL, INVALID_EAT_PANCAKE, SCREEN_BOTTOM_WIDTH, SCREEN_HEIGHT,
        SCREEN_TOP_WIDTH,
    },
    platform::{pl_is_homebrew, SMDH},
    resource::TitleInfo,
    utils::get_current_format_time,
};

pub mod menu;
pub mod title_list;
pub mod title_selected;

#[derive(Clone, Copy, PartialEq)]
pub enum SaveStoreType {
    Local,
    Cloud,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Actions {
    OpenTitle,
    BackupGameAllSaves,
    BackupAllGameAllSaves,
    BackupGameAllSavesToCloud,
    BackupAllGameAllSavesToCloud,
    About,
}

impl Deref for Actions {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Actions::OpenTitle => "打开游戏",
            Actions::BackupGameAllSaves => "备份【该】游戏所有存档",
            Actions::BackupAllGameAllSaves => "备份【所有】游戏所有存档",
            Actions::BackupGameAllSavesToCloud => "备份【该】游戏所有存档到【云端】",
            Actions::BackupAllGameAllSavesToCloud => "备份【所有】游戏所有存档到【云端】",
            Actions::About => "关于",
        }
    }
}

impl Display for Actions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}

pub fn Titles() -> Element {
    use_context_provider(|| Signal::new(SaveStoreType::Local));
    let app_exit = consume_context::<Rc<AppExit>>();
    let mut auth_state = use_context::<SyncSignal<AuthState>>();
    let app_exit_inner = app_exit.0.clone();
    let app_exit_inner = use_signal(|| app_exit_inner);
    let titles = use_context::<Signal<Vec<TitleInfo>>>();
    let title_selected = use_context::<Signal<Option<TitleSelected>>>();
    let mut tips_visible = use_context::<SyncSignal<TipsVisible>>();
    let mut loading = use_context::<SyncSignal<PageLoadingVisible>>();
    let mut confirm_visible = use_context::<Signal<ConfirmVisible>>();
    let mut dialog_visible = use_dialog(
        false,
        SCREEN_HEIGHT as f64,
        0.0,
        SCREEN_BOTTOM_WIDTH as f64,
        SCREEN_HEIGHT as f64,
        None,
    );
    let selected = use_signal(|| SaveStoreType::Local);
    let is_homebrew = use_signal(|| pl_is_homebrew());

    let is_pending = use_memo(move || {
        loading.try_read().is_ok_and(|l| l.visible())
            || dialog_visible.read().is_show()
            || confirm_visible.read().dialog.read().is_show()
    });

    let mut toast = move |text: String| {
        if let Ok(mut visible) = tips_visible.try_write() {
            visible.show(Some(text));
        }
    };

    let mut notify = move |title: Option<String>, desc: Option<String>| {
        if let Ok(mut visible) = loading.try_write() {
            visible.show_info(title, desc);
        }
    };

    let do_backup = move |store_type: SaveStoreType,
                          backup_name: String,
                          save_type: SaveTypes,
                          title: TitleInfo,
                          title_name: String|
          -> Result<(), Box<dyn Error>> {
        if store_type == SaveStoreType::Local {
            get_game_local_backup_path(title, save_type, title_name, backup_name).and_then(
                |backup_path| backup_game_save_to_local(backup_path, title, save_type, notify),
            )
        } else {
            backup_game_save_to_cloud(
                None,
                title,
                save_type,
                title_name,
                backup_name,
                toast,
                notify,
                false,
            )
        }
    };

    rsx! {
        div {
            "screen": "top",
            "bg_reset": "main_bg",
            "deep_3d": 2.0,
            display: "flex",
            width: SCREEN_TOP_WIDTH,
            height: SCREEN_HEIGHT,
            position: "relative",

            TitleList {
                is_pending: dialog_visible.read().visible(),
            }
        }

        div {
            "screen": "bottom",
            "bg_reset": "main_bg",
            position: "absolute",
            left: 0,
            top: 0,
            display: "flex",
            flex_direction: "column",
            width: SCREEN_BOTTOM_WIDTH,
            height: SCREEN_HEIGHT,
            onkeypress: move |e| {
                if e.data.code() == Code::KeyA && !is_pending() && title_selected.try_read().is_ok_and(|v| v.is_some()) {
                    dialog_visible.write().show();
                }
            },

            NavBar {
                is_pending: is_pending(),
                TitleSaveTypes {
                    visible: dialog_visible,
                    onclick: move |_| {
                        if !is_pending() && title_selected.try_read().is_ok_and(|v| v.is_some()) {
                            dialog_visible.write().show();
                        }
                    }
                }
            }

            div {
                flex: 1,
                display: "flex",
                flex_direction: "column",
                padding: 6.0,
                gap: 5.0,

                if title_selected.try_read().is_ok_and(|v| v.is_some()) {
                    for action in [
                        Actions::OpenTitle,
                        Actions::BackupGameAllSaves,
                        Actions::BackupAllGameAllSaves,
                        Actions::BackupGameAllSavesToCloud,
                        Actions::BackupAllGameAllSavesToCloud,
                        Actions::About
                    ] {
                        if !*is_homebrew.read() || action != Actions::OpenTitle {
                            Button {
                                flex: 1,
                                display: "flex",
                                align_items: "center",
                                justify_content: "space-between",
                                bg_color: "selected_bg_info",
                                bg_active_color: "selected_bg",
                                background_color: "selected_bg_info",
                                padding_top: 2.0,
                                padding_left: 10.0,
                                padding_right: 10.0,
                                onclick: move |_: Event<MouseData>| {
                                    if (action == Actions::BackupGameAllSavesToCloud || action == Actions::BackupAllGameAllSavesToCloud) && (!Api::get_read().is_login() || !Api::is_eat_pancake_valid()) {
                                        if !Api::get_read().is_login() {
                                            auth_state.write().0 = false;
                                            toast("未登录，请重新登录！".to_string());
                                        } else {
                                            confirm_visible.write().show_qrcode(
                                                INVALID_EAT_PANCAKE.to_string(),
                                                HOME_PAGE_URL.to_string(),
                                                Rc::new(RefCell::new(Box::new(move || {}))),
                                            );
                                        }
                                        return;
                                    }
                                    let list = titles.read().iter().map(|t| t.clone()).collect::<Vec<_>>();
                                    let size = list.len();
                                    let store_type = if action == Actions::BackupAllGameAllSaves || action == Actions::BackupGameAllSaves {
                                        SaveStoreType::Local
                                    } else {
                                        SaveStoreType::Cloud
                                    };
                                    match action {
                                        Actions::OpenTitle => {
                                            if let Some(title) = title_selected.read().as_ref().map(|s| s.title.clone()) {
                                                confirm_visible.write().show(
                                                    format!("{} ?", action),
                                                    Rc::new(RefCell::new(Box::new(move || {
                                                            *app_exit_inner.read().lock().unwrap() =
                                                                (title.id, Some((title.fs_media_type, String::new())));
                                                    })))
                                                );
                                            }
                                        }
                                        Actions::BackupGameAllSavesToCloud |
                                        Actions::BackupGameAllSaves => {
                                            if let Some((title, save_types)) = title_selected.read().as_ref().map(|s| (s.title, s.saves.clone())) {
                                                let size = save_types.len();
                                                if size == 0 {
                                                    toast("没有存档".to_string());
                                                    return;
                                                }
                                                confirm_visible.write().show(
                                                    format!("{} ?", action),
                                                    Rc::new(RefCell::new(Box::new(move || {
                                                        let save_types = save_types.clone();
                                                        loading.write().show();
                                                        tokio::task::spawn_blocking(move || {
                                                            let mut failed = 0;
                                                            for (idx, save_type) in save_types.into_iter().enumerate() {
                                                                if let Some(title_name) = SMDH::new(title.id, title.fs_media_type as u8).and_then(|s| s.short_desc()) {
                                                                        notify(Some(format!("正在备份 {}/{}: {}", idx + 1, size, save_type)), Some(title_name.to_string()));
                                                                        if let Err(err) = do_backup(
                                                                            store_type,
                                                                            format!("{}.zip", get_current_format_time()),
                                                                            save_type,
                                                                            title,
                                                                            title_name.to_string()
                                                                        ) {
                                                                            failed += 1;
                                                                            toast(format!("备份失败: {} ({})", err, &title_name));
                                                                        }
                                                                }
                                                            }
                                                            toast(format!("备份完成: {} 成功, {} 失败", size - failed, failed));
                                                            loading.write().hide();
                                                        });
                                                    })))
                                                );
                                            }
                                        }
                                        Actions::BackupAllGameAllSavesToCloud |
                                        Actions::BackupAllGameAllSaves => {
                                            confirm_visible.write().show(
                                                format!("{} ?", action),
                                                Rc::new(RefCell::new(Box::new(move || {
                                                    let list = list.clone();
                                                    loading.write().show();
                                                    tokio::task::spawn_blocking(move || {
                                                        let mut failed = 0;
                                                        for (idx, title) in list.into_iter().enumerate() {
                                                            if let Some(save_type) = SaveTypes::get_title_save_type(title.id) {
                                                                if save_type == 0 {
                                                                    continue;
                                                                }
                                                                let mut c = 0;
                                                                if let Some(title_name) = SMDH::new(title.id, title.fs_media_type as u8).and_then(|s| s.short_desc()) {
                                                                    notify(Some(format!("正在备份 {}/{}: {}", idx + 1, size, SaveTypes::User)), Some(title_name.to_string()));
                                                                    if save_type & 0b0001 != 0 {
                                                                        if let Err(err) = do_backup(
                                                                            store_type,
                                                                            format!("{}.zip", get_current_format_time()),
                                                                            SaveTypes::User,
                                                                            title,
                                                                            title_name.to_string()
                                                                        ) {
                                                                            c = 1;
                                                                            toast(format!("备份失败: {} ({})", err, &title_name));
                                                                        }
                                                                    }
                                                                    notify(Some(format!("正在备份 {}/{}: {}", idx + 1, size, SaveTypes::Ext)), Some(title_name.to_string()));
                                                                    if save_type & 0b0010 != 0 {
                                                                        if let Err(err) = do_backup(
                                                                            store_type,
                                                                            format!("{}.zip", get_current_format_time()),
                                                                            SaveTypes::Ext,
                                                                            title,
                                                                            title_name.to_string()
                                                                        ) {
                                                                            c = 1;
                                                                            toast(format!("备份失败: {} ({})", err, &title_name));
                                                                        }
                                                                    }
                                                                    notify(Some(format!("正在备份 {}/{}: {}", idx + 1, size, SaveTypes::Sys)), Some(title_name.to_string()));
                                                                    if save_type & 0b0100 != 0 {
                                                                        if let Err(err) = do_backup(
                                                                            store_type,
                                                                            format!("{}.zip", get_current_format_time()),
                                                                            SaveTypes::Sys,
                                                                            title,
                                                                            title_name.to_string()
                                                                        ) {
                                                                            c = 1;
                                                                            toast(format!("备份失败: {} ({})", err, &title_name));
                                                                        }
                                                                    }
                                                                    notify(Some(format!("正在备份 {}/{}: {}", idx + 1, size, SaveTypes::Boss)), Some(title_name.to_string()));
                                                                    if save_type & 0b1000 != 0 {
                                                                        if let Err(err) = do_backup(
                                                                            store_type,
                                                                            format!("{}.zip", get_current_format_time()),
                                                                            SaveTypes::Boss,
                                                                            title,
                                                                            title_name.to_string()
                                                                        ) {
                                                                            c = 1;
                                                                            toast(format!("备份失败: {} ({})", err, &title_name));
                                                                        }
                                                                    }
                                                                }
                                                                failed += c;
                                                            }
                                                        }
                                                        toast(format!("备份完成: {} 成功, {} 失败", size - failed, failed));
                                                        loading.write().hide();
                                                    });
                                                })))
                                            );
                                        }
                                        Actions::About => {
                                            confirm_visible.write().show_qrcode(
                                                ABOUT_TEXT.to_string(),
                                                HOME_PAGE_URL.to_string(),
                                                Rc::new(RefCell::new(Box::new(move || {})))
                                            );
                                        }
                                    }
                                },

                                "{action}"
                                "->"
                            }
                        }
                    }
                }
            }

            ActionBar {
                version: true,
                tips: "(START) 退出   (A) 备份/恢复",
                onkeypress: move |e: KeyboardEvent| {
                    if is_pending() || app_exit.is_exit() {
                        return;
                    }
                    if e.data().code() == Code::Enter {
                        app_exit.set_exit();
                    }
                }
            }

            if dialog_visible.read().is_show() {
                Dialog {
                    visible: dialog_visible,
                    Menu {
                        visible: dialog_visible,
                        selected
                    }
                }
            }

            if confirm_visible.read().dialog.read().is_show() {
                Dialog {
                    visible: confirm_visible.read().dialog,
                    background_color: "transparent",
                    Confirm {
                        visible: confirm_visible,
                    }
                }
            }


            if let Some(true) = loading.try_read().ok().map(|v| v.is_show()) {
                PageLoading {
                    visible: loading,
                }
            }

            if let Some(true) = tips_visible.try_read().ok().map(|v| v.is_show()) {
                Tips {
                    visible: tips_visible,
                }
            }
        }
    }
}

use std::{
    cell::RefCell,
    error::Error,
    fs::{self, create_dir_all},
    path::Path,
    rc::Rc,
    thread::sleep,
    time::{Duration, Instant},
};

use ctru::{applets::swkbd::Kind, services::fs::ArchiveID};
use dioxus::prelude::*;
use log::error;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::{
    api::{Api, SaveItem},
    app::{
        action_bar::ActionBar,
        auth::Auth,
        button::Button,
        confirm::ConfirmVisible,
        dialog::DialogVisible,
        line::Line,
        list_display_status::ListState,
        list_wrap_display_status::{ListDisplayStatus, ScrollAction},
        loading::PageLoadingVisible,
        tips::TipsVisible,
        titles::{title_selected::TitleSelected, SaveStoreType},
        AuthState,
    },
    constant::{
        GAME_SAVE_CLOUD_DIR, HOME_LOCAL_PATH_SAVE, HOME_PAGE_URL, INVALID_EAT_PANCAKE,
        SCREEN_TOP_WIDTH,
    },
    fsu,
    platform::{pl_commit_arch_data, pl_delete_arch_sv, pl_show_swkbd, SMDH},
    resource::{Resource, TitleInfo},
    utils::{
        backup_game_save, check_save_arch_is_empty, delete_dir_if_empty, get_current_format_time,
        get_local_dir_start_with, get_local_game_saves, join_path, normalize_path,
        restore_game_save,
    },
};

use super::title_selected::SaveTypes;

#[derive(Props, Clone, PartialEq)]
pub struct MenuProps {
    visible: Signal<DialogVisible>,
    selected: Signal<SaveStoreType>,
}

#[derive(Props, Clone, PartialEq)]
struct ItemProps {
    selected: bool,
    children: Element,
    pub onclick: Option<EventHandler<MouseEvent>>,
}

fn fetch_game_title_detail(title: TitleInfo) -> Option<(String, String, String)> {
    SMDH::new(title.id, title.fs_media_type as u8).and_then(|s| {
        s.short_desc()
            .map(|name| (title.id_hex_str(), name, title.product_code()))
    })
}

pub fn get_game_local_backup_path(
    title: TitleInfo,
    save_type: SaveTypes,
    title_name: String,
    backup_name: String,
) -> Result<String, Box<dyn Error>> {
    let game_backup_save_type_dir = join_path(HOME_LOCAL_PATH_SAVE, &save_type);
    let path = match get_local_dir_start_with(&game_backup_save_type_dir, &title.id_hex_str()) {
        Some(path) => join_path(&path, &backup_name),
        None => {
            let path = join_path(
                &game_backup_save_type_dir,
                &format!(
                    "{} {}",
                    &title.id_hex_str(),
                    normalize_path(title_name.trim())
                )
                .trim(),
            );
            create_dir_all(&path)?;
            join_path(&path, &backup_name)
        }
    };

    Ok(path)
}

fn get_game_cloud_backup_path(
    game_save_cloud_dir: Option<String>,
    title: TitleInfo,
    save_type: SaveTypes,
    title_name: String,
    toast: impl FnMut(String) + Copy,
) -> String {
    match game_save_cloud_dir {
        Some(dir) => dir,
        None => match Api::fetch_save_cloud_list(&title.id_hex_str(), &save_type, true, toast) {
            (Some(dir), _) => dir,
            _ => join_path(
                &join_path(GAME_SAVE_CLOUD_DIR, &save_type),
                &format!(
                    "{} {}",
                    &title.id_hex_str(),
                    normalize_path(title_name.trim())
                )
                .trim(),
            ),
        },
    }
}

fn fetch_game_save_local(
    title: TitleInfo,
    save_type: SaveTypes,
    mut list_local: SyncSignal<(ListState, Vec<String>, bool)>,
) {
    // update local list
    let target_path = join_path(HOME_LOCAL_PATH_SAVE, &save_type);
    if let Some(path) = get_local_dir_start_with(&target_path, &title.id_hex_str()) {
        let res = get_local_game_saves(&path);
        list_local.with_mut(|list| {
            list.0.update(res.len() as i32 + 1);
            list.1 = res;
            list.2 = true;
        });
    } else {
        list_local.with_mut(|list| {
            list.2 = true;
        });
    }
}

pub fn fetch_game_save_cloud(
    title: TitleInfo,
    save_type: SaveTypes,
    mut list_cloud: SyncSignal<(ListState, Vec<SaveItem>, Option<String>, bool)>,
    toast: impl FnMut(String) + Copy,
) {
    // update cloud list
    let (dir, res) = Api::fetch_save_cloud_list(&title.id_hex_str(), &save_type, false, toast);
    list_cloud.with_mut(|list_cloud| {
        res.map(|res| {
            list_cloud.0.update(res.len() as i32 + 1);
            list_cloud.1 = res;
        });
        list_cloud.2 = dir;
        list_cloud.3 = true;
    });
}

pub fn backup_game_save_to_local(
    backup_path: String,
    title: TitleInfo,
    save_type: SaveTypes,
    notify: impl FnMut(Option<String>, Option<String>) + Copy,
) -> Result<(), Box<dyn Error>> {
    if let (Ok(arch_from), Ok(arch_to)) = (
        fsu::arch(
            save_type.arch_id(),
            title.fs_media_type,
            title.high_id(),
            title.low_id(),
        ),
        fsu::arch(
            ArchiveID::Sdmc,
            title.fs_media_type,
            title.high_id(),
            title.low_id(),
        ),
    ) {
        if !check_save_arch_is_empty("/", &arch_from) {
            backup_game_save(("/", &arch_from), (&backup_path, &arch_to), notify)?;
        } else {
            return Err("存档数据为空！".into());
        }
    }

    Ok(())
}

pub fn backup_game_save_to_cloud(
    game_save_cloud_dir: Option<String>,
    title: TitleInfo,
    save_type: SaveTypes,
    title_name: String,
    backup_name: String,
    toast: impl FnMut(String) + Copy,
    notify: impl FnMut(Option<String>, Option<String>) + Copy,
    is_overwrite: bool,
) -> Result<(), Box<dyn Error>> {
    // backup to local
    let local_backup_path =
        get_game_local_backup_path(title, save_type, title_name.clone(), backup_name.clone())
            .and_then(|local_backup_path| {
                backup_game_save_to_local(local_backup_path.clone(), title, save_type, notify)
                    .map(|_| local_backup_path)
            })?;

    // get cloud dir
    let cloud_dir =
        get_game_cloud_backup_path(game_save_cloud_dir, title, save_type, title_name, toast);

    // upload to cloud
    let res = Api::upload_to_cloud(
        &cloud_dir,
        &backup_name,
        &local_backup_path,
        is_overwrite,
        notify,
    );

    // remove local backup after upload
    if Path::new(&local_backup_path).exists() {
        if let Err(err) = fs::remove_file(&local_backup_path) {
            error!(
                "remove {} failed after backup upload: {}",
                local_backup_path, err
            );
        }
        if let Some(parent) = Path::new(&local_backup_path).parent() {
            let _ = delete_dir_if_empty(parent);
        }
    }

    res
}

fn restore_backup(
    title: TitleInfo,
    save_type: SaveTypes,
    backup_path: String,
    mut toast: impl FnMut(String) + Copy,
    notify: impl FnMut(Option<String>, Option<String>) + Copy,
) -> Result<(), Box<dyn Error>> {
    if let (Ok(arch_from), Ok(arch_to)) = (
        fsu::arch(
            ArchiveID::Sdmc,
            title.fs_media_type,
            title.high_id(),
            title.low_id(),
        ),
        fsu::arch(
            save_type.arch_id(),
            title.fs_media_type,
            title.high_id(),
            title.low_id(),
        ),
    ) {
        // restore
        restore_game_save((&backup_path, &arch_from), ("/", &arch_to), notify)?;
        // commit data
        if !pl_commit_arch_data(&arch_to) {
            toast("提交数据失败！".to_string());
            error!("提交数据失败！");
        }
        // delete arch security value
        if !pl_delete_arch_sv(&arch_to, title.low_id() >> 8) {
            toast("删除安全值失败！".to_string());
            error!("删除安全值失败！");
        }
    }

    Ok(())
}

fn Item(props: ItemProps) -> Element {
    rsx! {
        div {
            height: 20.0,
            padding: 1,
            background_color: if props.selected {
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
                onclick: move |e| {
                    props.onclick.as_ref().map(|h| h.call(e));
                },
                {props.children}
            }
        }
    }
}

pub fn Menu(mut props: MenuProps) -> Element {
    let resource = consume_context::<Rc<Resource>>();
    let title_selected = use_context::<Signal<Option<TitleSelected>>>();
    let title_list_display = use_context::<Signal<ListDisplayStatus>>();
    let mut auth_state = use_context::<SyncSignal<AuthState>>();
    let mut store_type = use_context::<Signal<SaveStoreType>>();
    let mut loading = use_context::<SyncSignal<PageLoadingVisible>>();
    let mut list_local = use_signal_sync(|| (ListState::new(9), vec![] as Vec<String>, false));
    let mut list_cloud =
        use_signal_sync(|| (ListState::new(9), vec![] as Vec<SaveItem>, None, false));
    let mut tips_visible = use_context::<SyncSignal<TipsVisible>>();
    let mut title_detail = use_signal_sync::<Option<(String, String, String)>>(|| None);
    let mut confirm_visible = use_context::<Signal<ConfirmVisible>>();

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

    use_effect(move || {
        if let Some((title, save_type)) = title_selected
            .peek()
            .as_ref()
            .map(|s| (s.title.clone(), s.save_type.clone()))
        {
            let is_local_init = list_local.peek().2;
            let is_cloud_init = list_cloud.peek().3;
            let is_local = *store_type.read() == SaveStoreType::Local && !is_local_init;
            let mut is_cloud =
                auth_state.read().0 && *store_type.read() == SaveStoreType::Cloud && !is_cloud_init;

            if is_cloud && !Api::get_read().is_login() {
                is_cloud = false;
                auth_state.write().0 = false;
            }
            if !is_local && !is_cloud {
                return;
            }
            loading.write().show();
            tokio::task::spawn_blocking(move || {
                let now = Instant::now();
                if title_detail.peek().is_none() {
                    fetch_game_title_detail(title).map(|(id, name, product_code)| {
                        title_detail.set(Some((id, name, product_code)));
                    });
                }
                if now.elapsed() < Duration::from_millis(300) {
                    sleep(Duration::from_millis(300) - now.elapsed());
                }
                if is_local {
                    save_type.map(|save_type| {
                        fetch_game_save_local(title, save_type, list_local);
                    });
                } else {
                    save_type.map(|save_type| {
                        fetch_game_save_cloud(title, save_type, list_cloud, toast);
                    });
                }
                loading.write().hide();
            });
        }
    });

    let selected_idx = title_list_display.read().selected_idx;
    let top_row = title_list_display.read().top_row;

    let is_pending = use_memo(move || {
        loading.read().visible()
            || !props.visible.read().visible()
            || confirm_visible.read().dialog.read().is_show()
    });

    rsx! {
        if let Some((id, name, product_code)) = title_detail.read().as_ref() {
            div {
                "screen": "top",
                "deep_3d": 1.0,
                "scale": 0.7,
                color: "white",
                display: "flex",
                justify_content: "center",
                align_items: if selected_idx / 8 == top_row || selected_idx / 8 == top_row + 1 { "flex-end" } else { "flex-start" },
                position: "absolute",
                width: SCREEN_TOP_WIDTH as f64,
                padding_top: 10.0,
                padding_bottom: 10.0,
                top: 0,
                left: 0,
                right: 0,
                bottom: 0,

                div {
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    justify_content: "center",
                    padding_top: 10.0,
                    padding_bottom: 8.0,
                    padding_left: 20.0,
                    padding_right: 20.0,
                    background_color: "selected_bg",

                    "{name}"

                    div {
                        display: "flex",
                        "scale": 0.4,

                         if let Some((save_type, media)) = title_selected.read().as_ref().map(|s| (s.save_type, s.title.media_str())) {
                             if let Some(save_type) = save_type {
                                 "[{save_type}] "
                             }

                             "[{id}] [{media}] [{product_code}]"
                         }
                    }
                }
            }
        }
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
                    Code::KeyL => {
                        if *store_type.read() != SaveStoreType::Local {
                            store_type.set(SaveStoreType::Local);
                        }
                    }
                    Code::KeyR => {
                        if *store_type.read() != SaveStoreType::Cloud {
                            store_type.set(SaveStoreType::Cloud);
                        }
                    }
                    Code::ArrowUp => {
                        if *store_type.read() == SaveStoreType::Local {
                            let size = list_local.read().1.len() as i32;
                            list_local.write().0.do_scroll(size + 1, ScrollAction::Up);
                        } else {
                            let size = list_cloud.read().1.len() as i32;
                            list_cloud.write().0.do_scroll(size + 1, ScrollAction::Up);
                        }
                    }
                    Code::ArrowDown => {
                        if *store_type.read() == SaveStoreType::Local {
                            let size = list_local.read().1.len() as i32;
                            list_local.write().0.do_scroll(size + 1, ScrollAction::Down);
                        } else {
                            let size = list_cloud.read().1.len() as i32;
                            list_cloud.write().0.do_scroll(size + 1, ScrollAction::Down);
                        }
                    }
                    // backup
                    Code::KeyA => {
                        let mut do_backup = move |is_overwrite: bool, backup_name: String, save_type: SaveTypes, title: TitleInfo, title_name: String| {
                            if *store_type.read() == SaveStoreType::Local {
                                loading.write().show();
                                tokio::task::spawn_blocking(move || {
                                    notify(Some("正在备份到本地".to_string()), None);
                                    // backup
                                    if let Err(err) = get_game_local_backup_path(
                                        title,
                                        save_type,
                                        title_name,
                                        backup_name,
                                    ).and_then(|backup_path| {
                                        backup_game_save_to_local(
                                            backup_path,
                                            title,
                                            save_type,
                                            notify,
                                        )
                                    }) {
                                        toast(format!("本地备份失败: {}", err));
                                    } else {
                                        // update local list
                                        fetch_game_save_local(title, save_type, list_local);
                                        toast("本地备份完成！".to_string());
                                    }
                                    loading.write().hide();
                                });
                            } else {
                                loading.write().show();
                                tokio::task::spawn_blocking(move || {
                                    notify(Some("正在备份到云端".to_string()), None);
                                    // backup
                                    let cloud_dir = { list_cloud.read().2.clone() };
                                    if let Err(err) = backup_game_save_to_cloud(
                                        cloud_dir,
                                        title,
                                        save_type,
                                        title_name,
                                        backup_name,
                                        toast,
                                        notify,
                                        is_overwrite,
                                    ) {
                                        toast(format!("云端备份失败: {}", err));
                                    } else {
                                        fetch_game_save_cloud(title, save_type, list_cloud, toast);
                                        toast("云端备份完成！".to_string());
                                    }
                                    loading.write().hide();
                                });
                            }
                        };

                        if let Some((Some(save_type), selected, Some(title_name))) = title_selected.read()
                                .as_ref()
                                .map(|s| (s.save_type, s.clone(), title_detail.read().as_ref().map(|t| t.1.clone())))
                        {
                            if *store_type.read() == SaveStoreType::Cloud && (!Api::get_read().is_login() || !Api::is_eat_pancake_valid()) {
                                if !Api::get_read().is_login() {
                                    toast("未登录，请重新登录！".to_string());
                                    auth_state.write().0 = false;
                                } else {
                                    confirm_visible.write().show_qrcode(
                                        INVALID_EAT_PANCAKE.to_string(),
                                        HOME_PAGE_URL.to_string(),
                                        Rc::new(RefCell::new(Box::new(move || {}))),
                                    );
                                }
                                return;
                            }

                            let is_new_backup = (*store_type.read() == SaveStoreType::Local && list_local.read().0.selected_idx == 0) ||
                                (*store_type.read() == SaveStoreType::Cloud && list_cloud.read().0.selected_idx == 0);

                            if !is_new_backup {
                                if let Some(backup_name) = {
                                    if *store_type.read() == SaveStoreType::Local {
                                        list_local.read().1.get(list_local.read().0.selected_idx as usize - 1).map(|s| s.to_string())
                                    } else {
                                        list_cloud.read().1.get(list_cloud.read().0.selected_idx as usize - 1).map(|s| s.name.to_string())
                                    }
                                } {
                                    confirm_visible.write().show("覆盖当前备份？".to_string(), Rc::new(RefCell::new(Box::new(move || {
                                        do_backup(true, backup_name.clone(), save_type, selected.title, title_name.clone());
                                    }))));
                                }
                            } else {
                                if let Some(backup_name) = pl_show_swkbd(Kind::Normal, &resource, &get_current_format_time()).map(|name| {
                                    if name.ends_with(".zip") { name } else { format!("{}.zip", name) }
                                }) {
                                    do_backup(false, backup_name, save_type, selected.title, title_name);
                                } else {
                                    toast("备份取消！".to_string());
                                }
                            }
                        } else {
                            toast("没有存档！".to_string());
                        }
                    }
                    // 删除
                    Code::KeyX => {
                        if let Some((Some(save_type), selected, Some((backup_name, cloud_dir)))) = title_selected.read()
                            .as_ref()
                            .map(|s|
                                (
                                    s.save_type,
                                    s.clone(),
                                    if *store_type.read() == SaveStoreType::Local {
                                        list_local.read().1.get(list_local.read().0.selected_idx as usize - 1)
                                            .map(|item| (item.to_string(), None))
                                    } else {
                                        let cloud = list_cloud.read();
                                        cloud.1.get(list_cloud.read().0.selected_idx as usize - 1)
                                            .map(|item| (item.name.to_string(), cloud.2.clone()))
                                    }
                                )
                            )
                        {
                            if *store_type.read() == SaveStoreType::Cloud && (!Api::get_read().is_login() || !Api::is_eat_pancake_valid()) {
                                if !Api::get_read().is_login() {
                                    toast("未登录，请重新登录！".to_string());
                                    auth_state.write().0 = false;
                                } else {
                                    confirm_visible.write().show_qrcode(
                                        INVALID_EAT_PANCAKE.to_string(),
                                        HOME_PAGE_URL.to_string(),
                                        Rc::new(RefCell::new(Box::new(move || {}))),
                                    );
                                }
                                return;
                            }
                            confirm_visible.write().show(format!("确定删除 {} ?", backup_name), Rc::new(RefCell::new(Box::new(move || {
                                if *store_type.read() == SaveStoreType::Local {
                                    let backup_name = backup_name.clone();
                                    loading.write().show();
                                    tokio::task::spawn_blocking(move || {
                                        let target_path = join_path(HOME_LOCAL_PATH_SAVE, &save_type);
                                        if let Some(path) = get_local_dir_start_with(&target_path, &selected.title.id_hex_str()) {
                                            // delete local backup
                                            match fs::remove_file(join_path(&path, &backup_name)) {
                                                Ok(_) => {
                                                    fetch_game_save_local(selected.title, save_type, list_local);
                                                    delete_dir_if_empty(&path).ok();
                                                    toast("删除成功！".to_string());
                                                }
                                                Err(err) => {
                                                    toast(format!("删除失败: {}", err));
                                                }
                                            }
                                        }
                                        loading.write().hide();
                                    });
                                } else {
                                    let backup_name = backup_name.clone();
                                    let cloud_dir = cloud_dir.clone();
                                    loading.write().show();
                                    notify(Some("正在删除云端备份".to_string()), Some(backup_name.clone()));
                                    tokio::task::spawn_blocking(move || {
                                        if let Some(cloud_dir) = cloud_dir.clone() {
                                            let target_path = join_path(&cloud_dir, &backup_name);
                                            match Api::start_file_manager(
                                                &utf8_percent_encode(&target_path, NON_ALPHANUMERIC).to_string(),
                                                None,
                                                None,
                                                crate::api::ApiOperates::Delete,
                                            ) {
                                                Ok(_) => {
                                                    fetch_game_save_cloud(selected.title, save_type, list_cloud, toast);
                                                    // delete cloud backup dir if empty
                                                    if list_cloud.read().1.is_empty() {
                                                        tokio::task::spawn_blocking(move || {
                                                            Api::start_file_manager(
                                                                &utf8_percent_encode(&cloud_dir, NON_ALPHANUMERIC).to_string(),
                                                                None,
                                                                None,
                                                                crate::api::ApiOperates::Delete,
                                                            ).ok();
                                                        });
                                                    }
                                                    toast("删除成功！".to_string());
                                                }
                                                Err(err) => {
                                                    toast(format!("删除失败: {}", err));
                                                }
                                            }
                                        }
                                        loading.write().hide();
                                    });
                                }
                            }))));
                        }
                    }
                    // 上传/下载
                    Code::ShiftLeft => {
                        if let Some((Some(save_type), selected, Some(title_name), Some((backup_name, cloud_dir, fs_id)))) = title_selected.read()
                            .as_ref()
                            .map(|s|
                                (
                                    s.save_type,
                                    s.clone(),
                                    title_detail.read().as_ref().map(|t| t.1.clone()),
                                    if *store_type.read() == SaveStoreType::Local {
                                        list_local.read().1.get(list_local.read().0.selected_idx as usize - 1)
                                            .map(|item| (item.to_string(), None, 0))
                                    } else {
                                        let cloud = list_cloud.read();
                                        cloud.1.get(list_cloud.read().0.selected_idx as usize - 1)
                                            .map(|item| (item.name.to_string(), cloud.2.clone(), item.fs_id.clone()))
                                    }
                                )
                            )
                        {
                            if !Api::get_read().is_login() || !Api::is_eat_pancake_valid() {
                                if !Api::get_read().is_login() {
                                    toast("未登录，请重新登录！".to_string());
                                    auth_state.write().0 = false;
                                } else {
                                    confirm_visible.write().show_qrcode(
                                        INVALID_EAT_PANCAKE.to_string(),
                                        HOME_PAGE_URL.to_string(),
                                        Rc::new(RefCell::new(Box::new(move || {}))),
                                    );
                                }
                                return;
                            }
                            let title_tips = if *store_type.read() == SaveStoreType::Local {
                                format!("上传备份 {}?", &backup_name)
                            } else {
                                format!("下载备份 {}?", &backup_name)
                            };
                            confirm_visible.write().show(title_tips, Rc::new(RefCell::new(Box::new(move || {
                                if *store_type.read() == SaveStoreType::Local {
                                    let title_name = title_name.clone();
                                    let backup_name = backup_name.clone();
                                    let cloud_dir = cloud_dir.clone();
                                    loading.write().show();
                                    tokio::task::spawn_blocking(move || {
                                        notify(Some("正在上传".to_string()), Some(backup_name.clone()));
                                        let target_path = join_path(HOME_LOCAL_PATH_SAVE, &save_type);
                                        if let Some(path) = get_local_dir_start_with(&target_path, &selected.title.id_hex_str()) {
                                            let cloud_dir = get_game_cloud_backup_path(cloud_dir.clone(), selected.title, save_type, title_name.clone(), toast);
                                            match Api::upload_to_cloud(
                                                &cloud_dir,
                                                &backup_name,
                                                &join_path(&path, &backup_name),
                                                false,
                                                notify,
                                            ) {
                                                Ok(_) => {
                                                    fetch_game_save_cloud(selected.title, save_type, list_cloud, toast);
                                                    toast("备份上传完成！".to_string());
                                                }
                                                Err(err) => {
                                                    toast(format!("备份上传失败: {}", err));
                                                }
                                            }
                                            loading.write().hide();
                                        }
                                    });
                                } else {
                                    let backup_name = backup_name.clone();
                                    let title_name = title_name.clone();
                                    loading.write().show();
                                    tokio::task::spawn_blocking(move || {
                                        notify(Some("正在下载".to_string()), Some(backup_name.clone()));
                                        if let Err(err) = get_game_local_backup_path(
                                            selected.title,
                                            save_type,
                                            title_name.clone(),
                                            backup_name.clone(),
                                        ).and_then(|backup_path| {
                                            Api::start_download(fs_id, &backup_path, None)
                                        }) {
                                            toast(format!("备份下载失败: {}", err));
                                        } else {
                                            fetch_game_save_local(selected.title, save_type, list_local);
                                            toast("备份下载完成！".to_string());
                                        }
                                        loading.write().hide();
                                    });
                                }
                            }))));
                        }

                    }
                    // 恢复存档
                    Code::KeyY => {
                        if let Some((Some(save_type), selected, Some(title_name), Some((backup_name, fs_id)))) = title_selected.read()
                            .as_ref()
                            .map(|s|
                                (
                                    s.save_type,
                                    s.clone(),
                                    title_detail.read().as_ref().map(|t| t.1.clone()),
                                    if *store_type.read() == SaveStoreType::Local {
                                        list_local.read().1.get(list_local.read().0.selected_idx as usize - 1)
                                            .map(|item| (item.to_string(), 0))
                                    } else {
                                        let cloud = list_cloud.read();
                                        cloud.1.get(list_cloud.read().0.selected_idx as usize - 1)
                                            .map(|item| (item.name.to_string(), item.fs_id.clone()))
                                    }
                                )
                            )
                        {
                            if *store_type.read() == SaveStoreType::Cloud && (!Api::get_read().is_login() || !Api::is_eat_pancake_valid()) {
                                if !Api::get_read().is_login() {
                                    toast("未登录，请重新登录！".to_string());
                                    auth_state.write().0 = false;
                                } else {
                                    confirm_visible.write().show_qrcode(
                                        INVALID_EAT_PANCAKE.to_string(),
                                        HOME_PAGE_URL.to_string(),
                                        Rc::new(RefCell::new(Box::new(move || {}))),
                                    );
                                }
                                return;
                            }
                            confirm_visible.write().show(format!("恢复存档 {}?", &backup_name), Rc::new(RefCell::new(Box::new(move || {
                                if *store_type.read() == SaveStoreType::Local {
                                    let backup_name = backup_name.clone();
                                    loading.write().show();
                                    tokio::task::spawn_blocking(move || {
                                        notify(Some("正在恢复存档".to_string()), Some(backup_name.clone()));
                                        let target_path = join_path(HOME_LOCAL_PATH_SAVE, &save_type);
                                        if let Some(path) = get_local_dir_start_with(&target_path, &selected.title.id_hex_str()) {
                                            let backup_path = join_path(&path, &backup_name);
                                            match restore_backup(selected.title, save_type, backup_path, toast, notify) {
                                                Ok(_) => {
                                                    // update local backup list
                                                    fetch_game_save_local(selected.title, save_type, list_local);
                                                    toast("存档恢复完成！".to_string());
                                                }
                                                Err(err) => {
                                                    toast(format!("存档恢复失败: {}", err));
                                                }
                                            }
                                            loading.write().hide();
                                        }
                                    });
                                } else {
                                    let backup_name = backup_name.clone();
                                    let title_name = title_name.clone();
                                    loading.write().show();
                                    tokio::task::spawn_blocking(move || {
                                        notify(Some("正在下载".to_string()), Some(backup_name.clone()));
                                        if let Err(err) = get_game_local_backup_path(
                                            selected.title,
                                            save_type,
                                            title_name.clone(),
                                            backup_name.clone(),
                                        ).and_then(|backup_path| {
                                            Api::start_download(fs_id, &backup_path, None).and_then(|_| {
                                                // restore download backup
                                                let res = restore_backup(selected.title, save_type, backup_path.clone(), toast, notify);
                                                // remove download backup after restore
                                                fs::remove_file(&backup_path).ok();
                                                // update local backup list
                                                fetch_game_save_local(selected.title, save_type, list_local);
                                                // remove local backup dir if empty
                                                if let Some(parent) = Path::new(&backup_path).parent() {
                                                    delete_dir_if_empty(parent).ok();
                                                }
                                                res
                                            })
                                        }) {
                                            toast(format!("存档恢复失败: {}", err));
                                        } else {
                                            toast("存档恢复完成！".to_string());
                                        }
                                        loading.write().hide();
                                    });
                                }
                            }))));
                        }

                    }
                    Code::KeyB => {
                        props.visible.write().hide();
                    }
                    _ => {}
                }
            },

            div {
                display: "flex",
                height: 20.0,
                position: "relative",

                div {
                    "scale": 0.32,
                    position: "absolute",
                    left: 5,
                    right: 5,
                    display: "flex",
                    height: 20.0,
                    align_items: "center",
                    justify_content: "space-between",

                    div {
                        "L ←"
                    }

                    div {
                        "→ R"
                    }
                }

                div {
                    display: "flex",
                    flex: 1,
                    align_items: "center",
                    justify_content: "center",
                    background_color: if *store_type.read() == SaveStoreType::Local {
                        "selected_bg"
                    } else {
                        "transparent"
                    },
                    onclick: move |_| {
                        if is_pending() {
                            return;
                        }
                        if *store_type.read() == SaveStoreType::Cloud {
                            store_type.set(SaveStoreType::Local);
                        }
                    },

                    "本地备份"
                }

                div {
                    display: "flex",
                    flex: 1,
                    align_items: "center",
                    justify_content: "center",
                    background_color: if *store_type.read() == SaveStoreType::Cloud {
                        "selected_bg"
                    } else {
                        "transparent"
                    },
                    onclick: move |_| {
                        if is_pending() {
                            return;
                        }
                        if *store_type.read() == SaveStoreType::Local {
                            store_type.set(SaveStoreType::Cloud);
                        }
                    },
                    "云端备份"
                }
            }

            Line {}

            div {
                flex: 1,
                display: "flex",
                flex_direction: "column",
                padding: 5.0,
                margin_top: 4.0,

                if *store_type.read() == SaveStoreType::Local {
                    for idx in 0i32..9i32 {
                        if list_local.read().0.top_row + idx == 0 {
                            Item {
                                selected: list_local.read().0.selected_idx == 0,
                                onclick: move |_: Event<MouseData>| {
                                    if is_pending() {
                                        return;
                                    }
                                    list_local.write().0.set_selected_idx(0);
                                },
                                "新建本地备份"
                            }
                        } else if let Some(save) = list_local.read().1.get((list_local.read().0.top_row + idx - 1) as usize){
                            Item {
                                selected: list_local.read().0.selected_idx == list_local.read().0.top_row + idx,
                                onclick: move |_: Event<MouseData>| {
                                    let idx = list_local.read().0.top_row + idx;
                                    list_local.write().0.set_selected_idx(idx);
                                },
                                "{save}"
                            }
                        }
                    }
                } else {
                    if auth_state.try_read().is_ok_and(|s| s.0) {
                        for idx in 0i32..9i32 {
                            if list_cloud.read().0.top_row + idx == 0 {
                                Item {
                                    selected: list_cloud.read().0.selected_idx == 0,
                                    onclick: move |_: Event<MouseData>| {
                                        if is_pending() {
                                            return;
                                        }
                                        list_cloud.write().0.set_selected_idx(0);
                                    },
                                    "新建云端备份"
                                }
                            } else if let Some(save) = list_cloud.read().1.get((list_cloud.read().0.top_row + idx - 1) as usize){
                                Item {
                                    selected: list_cloud.read().0.selected_idx == list_cloud.read().0.top_row + idx,
                                    onclick: move |_: Event<MouseData>| {
                                        let idx = list_cloud.read().0.top_row + idx;
                                        list_cloud.write().0.set_selected_idx(idx);
                                    },
                                    "{save.name}"
                                }
                            }
                        }
                    } else {
                        div {
                            flex: 1,
                            display: "flex",
                            align_items: "center",
                            justify_content: "center",

                            Auth {}
                        }
                    }
                }
            }

            ActionBar {
                tips:if *store_type.read() == SaveStoreType::Local
                  { "(SELECT) 上传   (Y) 恢复   (X) 删除   (B) 关闭   (A) 选择" }
                  else
                  { "(SELECT) 下载   (Y) 恢复   (X) 删除   (B) 关闭   (A) 选择" }
            }
        }
    }
}

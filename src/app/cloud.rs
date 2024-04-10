#![allow(non_snake_case)]

use std::{
    cell::RefCell,
    env,
    error::Error,
    fmt::{Display, Formatter},
    fs,
    io::{Read, Write},
    ops::Deref,
    os::unix::fs::MetadataExt,
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};

use ctru::{
    applets::swkbd::Kind,
    services::fs::{ArchiveID, MediaType},
};
use dioxus::prelude::*;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use serde_json::to_string;

use crate::{
    api::{Api, ApiOperates},
    app::{
        action_bar::ActionBar,
        auth::Auth,
        cloud::menu::Menu,
        confirm::{Confirm, ConfirmVisible},
        dialog::{use_dialog, Dialog},
        loading::{PageLoading, PageLoadingVisible},
        no_data::NoData,
        tips::{Tips, TipsVisible},
        top_bar::NavBar,
        AppExit, AuthState,
    },
    constant::{
        FBI_SC_TITLE_ID, HOME_LOCAL_PATH_CACHE, HOME_PAGE_URL, INVALID_EAT_PANCAKE,
        SCREEN_BOTTOM_WIDTH, SCREEN_HEIGHT, SCREEN_TOP_WIDTH,
    },
    fsu,
    loader::loader_file,
    platform::{pl_is_fbi_title_exists, pl_is_homebrew, pl_show_swkbd, pl_storage_info},
    resource::Resource,
    utils::{
        copy_dir_all, copy_file, create_parent_if_not_exists, ease_out_expo,
        get_current_format_time, join_path, sleep_micros, storage_size_to_info, zip_dir,
        zip_extract, zip_file,
    },
};

use super::{list_display_status::ListState, list_wrap_display_status::ScrollAction};

pub mod menu;

#[derive(Clone, Copy, PartialEq)]
enum Actions {
    NewDir,
    Delete,
    Rename,
    Copy,
    Move,
    Upload,
    Download,
    Zip,
    Unzip,
    ZipAndUpload,
    InstallWithFBI,
}

impl Deref for Actions {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::NewDir => "新建文件夹",
            Self::Delete => "删除",
            Self::Rename => "重命名",
            Self::Copy => "复制",
            Self::Move => "移动",
            Self::Upload => "上传",
            Self::Download => "下载",
            Self::Zip => "压缩",
            Self::Unzip => "解压",
            Self::ZipAndUpload => "压缩并上传",
            Self::InstallWithFBI => "调用 FBI 安装",
        }
    }
}

impl Display for Actions {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.deref())
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Panels {
    Local,
    Cloud,
    LocalRight,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ChildItem {
    Local(String, bool),
    Cloud(String, u64, bool, u64),
}

impl ChildItem {
    pub fn is_local(&self) -> bool {
        match self {
            ChildItem::Local(_, _) => true,
            ChildItem::Cloud(_, _, _, _) => false,
        }
    }

    pub fn is_dir(&self) -> bool {
        match self {
            ChildItem::Local(_, is_dir) => *is_dir,
            ChildItem::Cloud(_, _, is_dir, _) => *is_dir,
        }
    }
}

impl AsRef<str> for ChildItem {
    fn as_ref(&self) -> &str {
        match self {
            ChildItem::Local(s, _) => s,
            ChildItem::Cloud(s, _, _, _) => s,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListItem {
    path: String,
    children: Vec<ChildItem>,
    list_state: ListState,
}

pub struct List {
    items: Vec<ListItem>,
}

impl List {
    pub fn new() -> Self {
        Self { items: vec![] }
    }

    fn pop(&mut self) -> bool {
        if self.items.len() > 1 {
            self.items.pop();
            return true;
        }
        false
    }

    fn get(&self, idx: usize) -> Option<ChildItem> {
        if self.items.is_empty() {
            return None;
        }
        self.items.last().and_then(|item| {
            item.children
                .get(item.list_state.top_row as usize + idx)
                .map(|c| c.clone())
        })
    }

    fn is_exists(&self, name: &str) -> bool {
        self.items.last().is_some_and(|list| {
            list.children
                .iter()
                .any(|c| c.as_ref().to_lowercase() == name.to_lowercase())
        })
    }

    fn is_selected_item_dir(&self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        self.items.last().is_some_and(|item| {
            item.children
                .get(item.list_state.selected_idx as usize)
                .is_some_and(|c| c.is_dir())
        })
    }

    fn selected_item(&self) -> Option<ChildItem> {
        if self.items.is_empty() {
            return None;
        }
        self.items.last().and_then(|item| {
            item.children
                .get(item.list_state.selected_idx as usize)
                .map(|c| c.clone())
        })
    }

    fn current_idx(&self) -> i32 {
        if self.items.is_empty() {
            return 0;
        }
        self.items
            .last()
            .map(|item| item.list_state.selected_idx + 1)
            .unwrap_or(0)
    }

    fn total_items(&self) -> i32 {
        if self.items.is_empty() {
            return 0;
        }
        self.items
            .last()
            .map(|item| item.children.len() as i32)
            .unwrap_or(0)
    }

    fn is_selected(&self, idx: i32) -> bool {
        if self.items.is_empty() {
            return false;
        }
        if let Some(item) = self.items.last() {
            item.list_state.selected_idx == item.list_state.top_row + idx
        } else {
            false
        }
    }

    fn list_do_scroll(&mut self, action: ScrollAction) {
        if self.items.is_empty() {
            return;
        }
        self.items.last_mut().map(|item| {
            item.list_state
                .do_scroll(item.children.len() as i32, action);
        });
    }

    fn is_not_init(&self) -> bool {
        self.items.is_empty()
    }

    fn current_selected_abs_path(&self) -> String {
        join_path(
            &self.current_abs_path(),
            &self
                .selected_item()
                .map(|item| {
                    if item.as_ref().ends_with('/') {
                        item.as_ref().to_string()
                    } else {
                        format!("{}/", item.as_ref())
                    }
                })
                .unwrap(),
        )
    }

    fn current_abs_path(&self) -> String {
        self.items
            .last()
            .map(|item| item.path.clone())
            .unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize)]
pub struct RecoveryData {
    local_list: String,
    local_list_right: String,
    cloud_list: String,
    storage_info: (Option<(f64, f64)>, Option<(f64, f64)>),
}

pub fn remove_recovery_data() {
    let path = join_path(HOME_LOCAL_PATH_CACHE, "recovery");
    if Path::new(&path).exists() {
        fs::remove_file(&path).ok();
    }
}

pub fn create_recovery_data(
    local_list: SyncSignal<List>,
    local_list_right: SyncSignal<List>,
    cloud_list: SyncSignal<List>,
    storage_info: SyncSignal<(Option<(f64, f64)>, Option<(f64, f64)>)>,
) -> Result<(), Box<dyn Error>> {
    let data = RecoveryData {
        local_list: to_string(&local_list.read().items)?,
        local_list_right: to_string(&local_list_right.read().items)?,
        cloud_list: to_string(&cloud_list.read().items)?,
        storage_info: *storage_info.read(),
    };

    let data = to_string(&data)?;

    let path = join_path(HOME_LOCAL_PATH_CACHE, "recovery");
    create_parent_if_not_exists(&path).ok();
    let mut file = fs::File::create(path)?;
    file.write_all(data.as_bytes())?;

    Ok(())
}

pub fn get_recovery_data() -> Result<RecoveryData, Box<dyn Error>> {
    let path = join_path(HOME_LOCAL_PATH_CACHE, "recovery");
    if !Path::new(&path).exists() {
        return Err("recovery data not found".into());
    }

    let mut file = fs::File::open(&path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    let data: RecoveryData = serde_json::from_str(&data)?;
    drop(file);

    remove_recovery_data();

    Ok(data)
}

#[derive(Clone)]
pub struct ListGlobalState {
    local_list: SyncSignal<List>,
    local_list_right: SyncSignal<List>,
    cloud_list: SyncSignal<List>,
    selected_panel: SyncSignal<(Panels, Instant)>,
    right_panel: SyncSignal<Panels>,
    right_panel_left: SyncSignal<f64>,
    menu_list_state: SyncSignal<ListState>,
    storage_info: SyncSignal<(Option<(f64, f64)>, Option<(f64, f64)>)>,
}

pub fn use_list_global_state(is_auth: bool) -> ListGlobalState {
    let data = get_recovery_data();
    let local_list = use_signal_sync(|| {
        if let Ok(data) = &data {
            if let Ok(items) = serde_json::from_str::<Vec<ListItem>>(&data.local_list) {
                return List { items };
            }
        }
        List::new()
    });
    let local_list_right = use_signal_sync(|| {
        if let Ok(data) = &data {
            if let Ok(items) = serde_json::from_str::<Vec<ListItem>>(&data.local_list_right) {
                return List { items };
            }
        }
        List::new()
    });
    let cloud_list = use_signal_sync(|| {
        if let Ok(data) = &data {
            if let Ok(items) = serde_json::from_str::<Vec<ListItem>>(&data.cloud_list) {
                return List { items };
            }
        }
        List::new()
    });
    let selected_panel = use_signal_sync(|| {
        (
            if !data.is_ok() && is_auth {
                Panels::Local
            } else {
                Panels::Cloud
            },
            Instant::now(),
        )
    });
    let right_panel = use_signal_sync(|| Panels::Cloud);
    let right_panel_left = use_signal_sync(|| {
        if !data.is_ok() && is_auth {
            320.0
        } else {
            80.0
        }
    });
    let menu_list_state = use_signal_sync(|| ListState::new(9));
    let storage_info = use_signal_sync::<(Option<(f64, f64)>, Option<(f64, f64)>)>(|| {
        if let Ok(data) = &data {
            return data.storage_info.clone();
        }
        (None, None)
    });

    ListGlobalState {
        local_list,
        local_list_right,
        cloud_list,
        selected_panel,
        right_panel,
        right_panel_left,
        menu_list_state,
        storage_info,
    }
}

fn get_dir_list(path: impl AsRef<Path>) -> Vec<(String, bool)> {
    let mut list = vec![];
    if !path.as_ref().exists() {
        return list;
    }
    if let Ok(dirs) = path.as_ref().read_dir() {
        for entry in dirs.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                list.push((name.to_string(), path.is_dir()));
            }
        }
    }

    list
}

fn cloud_enter_dir(
    mut list: SyncSignal<List>,
    path: &str,
    is_refresh: bool,
    mut auth_state: SyncSignal<AuthState>,
) -> Result<(), Box<dyn Error>> {
    if !Api::get_read().is_login() {
        auth_state.write().0 = false;
        return Ok(());
    }
    let api_type = Api::get_read().api_type;
    let url = Api::get_read().get_file_list_url(path, 0);
    match Api::start_fetch_dir_list(&url, api_type) {
        Ok(res) => {
            let children = res
                .into_iter()
                .map(|item| {
                    ChildItem::Cloud(item.server_filename, item.fs_id, item.isdir == 1, item.size)
                })
                .collect::<Vec<ChildItem>>();
            if let Ok(mut list) = list.try_write() {
                if is_refresh {
                    let list_state = list.items.pop().map(|mut item| {
                        item.list_state.update(children.len() as i32);
                        item.list_state
                    });
                    list.items.push(ListItem {
                        path: path.to_string(),
                        children,
                        list_state: list_state.unwrap_or(ListState::new(12)),
                    });
                } else {
                    list.items.push(ListItem {
                        path: path.to_string(),
                        children,
                        list_state: ListState::new(12),
                    });
                }
            }
            Ok(())
        }
        Err(e) => {
            return Err(e);
        }
    }
}

fn local_enter_dir(mut list: SyncSignal<List>, path: &str, is_refresh: bool) {
    let mut children = get_dir_list(path)
        .into_iter()
        .map(|(name, is_dir)| ChildItem::Local(name, is_dir))
        .collect::<Vec<ChildItem>>();
    children.sort_by(|a, b| {
        if a.is_dir() && !b.is_dir() {
            return std::cmp::Ordering::Less;
        } else if !a.is_dir() && b.is_dir() {
            return std::cmp::Ordering::Greater;
        }
        a.as_ref().to_lowercase().cmp(&b.as_ref().to_lowercase())
    });
    if let Ok(mut list) = list.try_write() {
        if is_refresh {
            let list_state = list.items.pop().map(|mut item| {
                item.list_state.update(children.len() as i32);
                item.list_state
            });
            list.items.push(ListItem {
                path: path.to_string(),
                children,
                list_state: list_state.unwrap_or(ListState::new(12)),
            });
        } else {
            list.items.push(ListItem {
                path: path.to_string(),
                children,
                list_state: ListState::new(12),
            });
        }
    }
}

pub fn Cloud() -> Element {
    let resource = consume_context::<Rc<Resource>>();
    let app_exit = consume_context::<Rc<AppExit>>();
    let mut auth_state = use_context::<SyncSignal<AuthState>>();
    let mut tips_visible = use_context::<SyncSignal<TipsVisible>>();
    let mut loading = use_context::<SyncSignal<PageLoadingVisible>>();
    let mut confirm_visible = use_context::<Signal<ConfirmVisible>>();
    let mut actions = use_signal::<Option<(Vec<Actions>, String, String)>>(|| None);
    let mut dialog_visible = use_dialog(
        false,
        SCREEN_HEIGHT as f64,
        0.0,
        SCREEN_BOTTOM_WIDTH as f64,
        SCREEN_HEIGHT as f64,
        None,
    );
    let mut pending = use_signal_sync(|| false);

    // cloud state
    let ListGlobalState {
        mut local_list,
        mut local_list_right,
        mut cloud_list,
        mut selected_panel,
        mut right_panel,
        mut right_panel_left,
        mut menu_list_state,
        mut storage_info,
    } = use_context::<ListGlobalState>();

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

    let mut start_animation = move || {
        let (panel, at) = *selected_panel.read();
        let right = *right_panel_left.peek();
        let to_right = panel == Panels::Local && right < 320.0;
        let to_left = panel != Panels::Local && right > 80.0;
        if to_left || to_right {
            pending.set(true);
            tokio::spawn(async move {
                loop {
                    let from = if to_left { 320.0 } else { 80.0 };
                    let to = if to_left { 80.0 } else { 320.0 };
                    let now_left = ease_out_expo(
                        Instant::now().duration_since(at),
                        Duration::from_millis(300),
                        from,
                        to,
                    );
                    if *right_panel_left.peek() != now_left {
                        right_panel_left.set(now_left);
                    }
                    if *right_panel_left.peek() == to {
                        break;
                    }
                    sleep_micros(16000).await;
                }
                pending.set(false);
            });
        }
    };

    use_effect(move || {
        let is_right_local = *right_panel.peek() == Panels::LocalRight;
        let local_is_not_init = local_list.peek().is_not_init();
        let local_right_is_not_init = is_right_local && local_list_right.peek().is_not_init();
        let cloud_is_not_init =
            auth_state.read().0 && !is_right_local && cloud_list.peek().is_not_init();
        let is_storage_local_info_not_init = storage_info.peek().0.is_none();
        let is_storage_cloud_info_not_init = auth_state.read().0 && storage_info.peek().1.is_none();
        if local_is_not_init || local_right_is_not_init || cloud_is_not_init {
            loading.write().show();
            tokio::task::spawn_blocking(move || {
                if local_is_not_init {
                    local_enter_dir(local_list, "/", false);
                }

                if is_storage_local_info_not_init {
                    let (free, total) = pl_storage_info();
                    storage_info.with_mut(|s| {
                        s.0 = Some((free as f64, total as f64));
                    })
                }
                if is_storage_cloud_info_not_init {
                    let (used, total) = Api::fetch_quota_info();
                    storage_info.with_mut(|s| {
                        s.1 = Some((total - used, total as f64));
                    })
                }

                if local_right_is_not_init {
                    local_enter_dir(local_list_right, "/", false)
                }

                if cloud_is_not_init {
                    if let Err(err) = cloud_enter_dir(cloud_list, "/", false, auth_state) {
                        toast(format!("获取云端文件列表失败: {}", err));
                    }
                }

                loading.write().hide();
            });
        }
    });

    let is_pending = use_memo(move || {
        loading.try_read().is_ok_and(|l| l.visible())
            || dialog_visible.read().is_show()
            || confirm_visible.read().dialog.read().is_show()
            || *pending.read()
    });

    let app_exit_inner = app_exit.0.clone();
    let do_action = move |(action, name): (Actions, String)| {
        let (from_panel, _) = *selected_panel.read();
        let to_panel = if from_panel == Panels::Local {
            *right_panel.read()
        } else {
            Panels::Local
        };
        let from_list = match from_panel {
            Panels::Local => local_list,
            Panels::LocalRight => local_list_right,
            Panels::Cloud => cloud_list,
        };
        let to_list = match to_panel {
            Panels::Local => local_list,
            Panels::LocalRight => local_list_right,
            Panels::Cloud => cloud_list,
        };
        let is_from_local = from_panel == Panels::Local || from_panel == Panels::LocalRight;
        let is_to_local = to_panel == Panels::Local || to_panel == Panels::LocalRight;
        let (from_dir, from_is_dir, fs_id) = {
            let list = from_list.read();
            (
                list.current_abs_path(),
                list.is_selected_item_dir(),
                if is_from_local {
                    None
                } else {
                    list.selected_item().and_then(|item| match item {
                        ChildItem::Cloud(_, fs_id, _, _) => Some(fs_id),
                        _ => None,
                    })
                },
            )
        };
        let to_dir = to_list.read().current_abs_path();
        match action {
            Actions::NewDir
            | Actions::Delete
            | Actions::Rename
            | Actions::Upload
            | Actions::Download
            | Actions::ZipAndUpload
            | Actions::InstallWithFBI
                if (!is_from_local
                    || action == Actions::Upload
                    || action == Actions::ZipAndUpload)
                    && (!Api::get_read().is_login() || !Api::is_eat_pancake_valid()) =>
            {
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
            _ => {}
        }
        match action {
            Actions::NewDir => {
                if let Some(input_name) = pl_show_swkbd(Kind::Normal, &resource, "") {
                    if from_list.read().is_exists(&input_name) {
                        toast("文件夹已存在！".to_string());
                    } else {
                        loading.write().show();
                        tokio::task::spawn_blocking(move || {
                            let new_name_path = join_path(&from_dir, &input_name);
                            if is_from_local {
                                if let Err(err) = fs::create_dir_all(&new_name_path) {
                                    toast(format!("新建文件夹失败: {}", err));
                                } else {
                                    local_enter_dir(from_list, &from_dir, true);
                                    if from_dir == to_dir && is_to_local {
                                        local_enter_dir(to_list, &to_dir, true);
                                    }
                                    toast("新建文件夹成功！".to_string());
                                }
                            } else {
                                notify(
                                    Some("正在新建文件夹".to_string()),
                                    Some(input_name.clone()),
                                );
                                match Api::start_create_dir(&from_dir, &input_name) {
                                    Ok(_) => {
                                        cloud_enter_dir(from_list, &from_dir, true, auth_state)
                                            .ok();
                                        toast("新建文件夹成功！".to_string());
                                    }
                                    Err(e) => {
                                        toast(format!("新建文件夹失败: {}", e));
                                    }
                                }
                            }
                            loading.write().hide();
                        });
                    }
                } else {
                    toast("新建文件夹取消！".to_string());
                }
            }
            Actions::Delete => {
                loading.write().show();
                tokio::task::spawn_blocking(move || {
                    if is_from_local {
                        if let Err(err) = if from_is_dir {
                            fs::remove_dir_all(&join_path(&from_dir, &name))
                        } else {
                            fs::remove_file(&join_path(&from_dir, &name))
                        } {
                            toast(format!("删除失败: {}", err));
                        } else {
                            local_enter_dir(from_list, &from_dir, true);
                            if from_dir == to_dir && is_to_local {
                                local_enter_dir(to_list, &to_dir, true);
                            }
                            toast("删除成功！".to_string());
                        }
                    } else {
                        notify(Some("正在删除".to_string()), Some(name.to_string()));
                        match Api::start_file_manager(
                            &utf8_percent_encode(&join_path(&from_dir, &name), NON_ALPHANUMERIC)
                                .to_string(),
                            None,
                            None,
                            crate::api::ApiOperates::Delete,
                        ) {
                            Ok(_) => {
                                cloud_enter_dir(from_list, &from_dir, true, auth_state).ok();
                                toast("删除成功！".to_string());
                            }
                            Err(e) => {
                                toast(format!("删除失败: {}", e));
                            }
                        }
                    }
                    loading.write().hide();
                });
            }
            Actions::Rename => {
                if let Some(input_name) = pl_show_swkbd(Kind::Normal, &resource, &name) {
                    if from_list.read().is_exists(&input_name) {
                        toast("已存在同名文件！".to_string());
                    } else {
                        loading.write().show();
                        tokio::task::spawn_blocking(move || {
                            let from_path = join_path(&from_dir, &name);
                            if is_from_local {
                                if let Err(err) =
                                    fs::rename(&from_path, join_path(&from_dir, &input_name))
                                {
                                    toast(format!("重命名失败: {}", err));
                                } else {
                                    local_enter_dir(from_list, &from_dir, true);
                                    if from_dir == to_dir && is_to_local {
                                        local_enter_dir(to_list, &to_dir, true);
                                    }
                                    toast("重命名成功！".to_string());
                                }
                            } else {
                                notify(Some("正在重命名".to_string()), Some(name.to_string()));
                                match Api::start_file_manager(
                                    &utf8_percent_encode(&from_path, NON_ALPHANUMERIC).to_string(),
                                    None,
                                    Some(
                                        &utf8_percent_encode(&input_name, NON_ALPHANUMERIC)
                                            .to_string(),
                                    ),
                                    ApiOperates::Rename,
                                ) {
                                    Ok(_) => {
                                        cloud_enter_dir(from_list, &from_dir, true, auth_state)
                                            .ok();
                                        toast("重命名成功！".to_string());
                                    }
                                    Err(e) => {
                                        toast(format!("重命名失败: {}", e));
                                    }
                                }
                            }
                            loading.write().hide();
                        });
                    }
                } else {
                    toast("重命名取消！".to_string());
                }
            }
            Actions::Copy => {
                if is_from_local && is_to_local {
                    loading.write().show();
                    tokio::task::spawn_blocking(move || {
                        if to_list.read().is_exists(&name) {
                            toast("已存在同名文件！".to_string());
                        } else {
                            if let Err(err) = if from_is_dir {
                                copy_dir_all(
                                    &join_path(&from_dir, &name),
                                    &join_path(&to_dir, &name),
                                )
                            } else {
                                copy_file(&join_path(&from_dir, &name), &join_path(&to_dir, &name))
                            } {
                                toast(format!("复制失败: {}", err));
                            } else {
                                local_enter_dir(from_list, &from_dir, true);
                                local_enter_dir(to_list, &to_dir, true);
                                toast("复制成功！".to_string());
                            }
                        }
                        loading.write().hide();
                    });
                }
            }
            Actions::Move => {
                if is_from_local && is_to_local {
                    loading.write().show();
                    tokio::task::spawn_blocking(move || {
                        if to_list.read().is_exists(&name) {
                            toast("已存在同名文件！".to_string());
                        } else {
                            if let Err(err) =
                                fs::rename(join_path(&from_dir, &name), join_path(&to_dir, &name))
                            {
                                toast(format!("移动失败: {}", err));
                            } else {
                                local_enter_dir(from_list, &from_dir, true);
                                local_enter_dir(to_list, &to_dir, true);
                                toast("移动成功！".to_string());
                            }
                        }
                        loading.write().hide();
                    });
                }
            }
            Actions::Upload => {
                if is_from_local && !is_to_local {
                    loading.write().show();
                    tokio::task::spawn_blocking(move || {
                        if to_list.read().is_exists(&name) {
                            toast("已存在同名文件！".to_string());
                        } else {
                            notify(Some("正在上传".to_string()), Some(name.to_string()));
                            let from_path = join_path(&from_dir, &name);
                            if let Err(err) =
                                Api::upload_to_cloud(&to_dir, &name, &from_path, false, notify)
                            {
                                toast(format!("上传失败: {}", err));
                            } else {
                                cloud_enter_dir(to_list, &to_dir, true, auth_state).ok();
                                toast("上传成功！".to_string());
                            }
                        }
                        loading.write().hide();
                    });
                }
            }
            Actions::Download => {
                if let Some(fs_id) = fs_id {
                    let progress = loading.read().download_progress;
                    loading.write().show();
                    tokio::task::spawn_blocking(move || {
                        if to_list.read().is_exists(&name) {
                            toast("已存在同名文件！".to_string());
                        } else {
                            notify(Some("正在下载".to_string()), Some(name.to_string()));
                            if let Err(err) = Api::start_download(
                                fs_id,
                                &join_path(&to_dir, &name),
                                Some(progress),
                            ) {
                                local_enter_dir(to_list, &to_dir, true);
                                toast(format!("下载失败: {}", err));
                            } else {
                                local_enter_dir(to_list, &to_dir, true);
                                toast("下载成功！".to_string());
                            }
                        }
                        loading.write().hide();
                    });
                }
            }
            Actions::Zip => {
                let new_name = format!("{}.zip", name);
                if is_from_local {
                    if let Some(zip_name) = if from_list.read().is_exists(&new_name) {
                        pl_show_swkbd(Kind::Normal, &resource, &name)
                            .map(|name| format!("{}.zip", name))
                    } else {
                        Some(new_name)
                    } {
                        if from_list.read().is_exists(&zip_name) {
                            toast("已存在同名文件！".to_string());
                        } else {
                            loading.write().show();
                            tokio::task::spawn_blocking(move || {
                                let zip_name_path = join_path(&from_dir, &zip_name);
                                notify(Some("正在压缩".to_string()), Some(name.to_string()));
                                if let Err(err) = if from_is_dir {
                                    match fsu::arch(ArchiveID::Sdmc, MediaType::Sd, 0, 0) {
                                        Ok(arch) => zip_dir(
                                            (&join_path(&from_dir, &name), &arch),
                                            (&zip_name_path, &arch),
                                            &[],
                                            notify,
                                        ),
                                        Err(err) => Err(format!("{}", err).into()),
                                    }
                                } else {
                                    zip_file(&from_dir, &name, &zip_name_path)
                                } {
                                    if Path::new(&zip_name_path).exists() {
                                        fs::remove_file(&zip_name_path).ok();
                                    }
                                    toast(format!("压缩失败: {}", err));
                                } else {
                                    local_enter_dir(from_list, &from_dir, true);
                                    if from_dir == to_dir {
                                        local_enter_dir(to_list, &to_dir, true);
                                    }
                                    toast("压缩成功！".to_string());
                                }
                                loading.write().hide();
                            });
                        }
                    }
                }
            }
            Actions::Unzip => {
                let new_name = name[0..name.len() - 4].to_string();
                if is_from_local {
                    if let Some(new_name) = if from_list.read().is_exists(&new_name) {
                        pl_show_swkbd(Kind::Normal, &resource, &new_name)
                    } else {
                        Some(new_name)
                    } {
                        if from_list.read().is_exists(&new_name) {
                            toast("已存在同名文件！".to_string());
                        } else {
                            loading.write().show();
                            tokio::task::spawn_blocking(move || {
                                notify(Some("正在解压".to_string()), Some(name.to_string()));
                                let new_name_path = join_path(&from_dir, &new_name);
                                if let Err(err) =
                                    match fsu::arch(ArchiveID::Sdmc, MediaType::Sd, 0, 0) {
                                        Ok(arch) => zip_extract(
                                            (&join_path(&from_dir, &name), &arch),
                                            (&new_name_path, &arch),
                                            notify,
                                        ),
                                        Err(err) => Err(format!("{}", err).into()),
                                    }
                                {
                                    toast(format!("解压失败: {}", err));
                                } else {
                                    local_enter_dir(from_list, &from_dir, true);
                                    if from_dir == to_dir {
                                        local_enter_dir(to_list, &to_dir, true);
                                    }
                                    toast("解压成功！".to_string());
                                }
                                loading.write().hide();
                            });
                        }
                    }
                }
            }
            Actions::ZipAndUpload => {
                if is_from_local && !is_to_local {
                    let zip_name = format!("{}.zip", name);
                    if to_list.read().is_exists(&zip_name) {
                        toast("已存在同名文件！".to_string());
                    } else {
                        let zip_path =
                            join_path(&from_dir, &format!("{}.zip", get_current_format_time()));
                        loading.write().show();
                        tokio::task::spawn_blocking(move || {
                            notify(Some("正在压缩".to_string()), Some(name.to_string()));
                            if let Err(err) = (if from_is_dir {
                                match fsu::arch(ArchiveID::Sdmc, MediaType::Sd, 0, 0) {
                                    Ok(arch) => zip_dir(
                                        (&join_path(&from_dir, &name), &arch),
                                        (&zip_path, &arch),
                                        &[],
                                        notify,
                                    ),
                                    Err(err) => Err(format!("{}", err).into()),
                                }
                            } else {
                                zip_file(&from_dir, &name, &zip_path)
                            })
                            .map(|_| {
                                notify(Some("正在上传".to_string()), Some(zip_path.to_string()));
                                Api::upload_to_cloud(&to_dir, &zip_name, &zip_path, false, notify)
                            }) {
                                toast(format!("压缩上传失败: {}", err));
                            } else {
                                cloud_enter_dir(to_list, &to_dir, true, auth_state).ok();
                                toast("压缩上传成功！".to_string());
                            }
                            if Path::new(&zip_path).exists() {
                                fs::remove_file(&zip_path).ok();
                            }
                            loading.write().hide();
                        });
                    }
                }
            }
            Actions::InstallWithFBI => {
                let app_exit_inner = app_exit_inner.clone();
                if let Some(fs_id) = fs_id {
                    loading.write().show();
                    tokio::task::spawn_blocking(move || {
                        match Api::fetch_download_link(fs_id) {
                            Ok(link) => {
                                let path = join_path(HOME_LOCAL_PATH_CACHE, "url");
                                create_parent_if_not_exists(&path).ok();
                                if let Err(err) = fs::File::create(&path)
                                    .map(|mut file| file.write_all(link.as_bytes()))
                                {
                                    toast(format!("调用 FBI 失败: {}", err));
                                } else {
                                    if pl_is_homebrew() {
                                        // save cloud path
                                        let sc_path = env::args()
                                            .collect::<Vec<String>>()
                                            .get(0)
                                            .map(|s| {
                                                if s.starts_with("sdmc:/") {
                                                    s.to_string()
                                                } else if s.starts_with("/") {
                                                    format!("sdmc:{}", s)
                                                } else {
                                                    "sdmc:/3ds/save-cloud.3dsx".to_string()
                                                }
                                            })
                                            .unwrap_or("sdmc:/3ds/save-cloud.3dsx".to_string());

                                        if let Some(fbi_path) =
                                            if Path::new("/3ds/fbi-sc.3dsx").exists() {
                                                Some("sdmc:/3ds/fbi-sc.3dsx")
                                            } else if Path::new("/3ds/fbi.3dsx").exists() {
                                                Some("sdmc:/3ds/fbi.3dsx")
                                            } else {
                                                None
                                            }
                                        {
                                            let _ = create_recovery_data(
                                                local_list,
                                                local_list_right,
                                                cloud_list,
                                                storage_info,
                                            );
                                            if loader_file(&fbi_path, &path, &sc_path) {
                                                *app_exit_inner.lock().unwrap() = (1, None);
                                            } else {
                                                remove_recovery_data();
                                                toast("调用 FBI 失败".to_string());
                                            }
                                        } else {
                                            toast("未找到 FBI: /3ds/fbi-sc.3dsx".to_string());
                                        }
                                    } else {
                                        if pl_is_fbi_title_exists() {
                                            let _ = create_recovery_data(
                                                local_list,
                                                local_list_right,
                                                cloud_list,
                                                storage_info,
                                            );
                                            let path = format!("sc:{}", path);
                                            *app_exit_inner.lock().unwrap() =
                                                (FBI_SC_TITLE_ID, Some((MediaType::Sd, path)));
                                        } else {
                                            toast("未安装 FBI".to_string());
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                toast(format!("获取下载链接失败: {}", err));
                            }
                        }
                        loading.write().hide();
                    });
                }
            }
        }
        dialog_visible.write().hide();
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
            onkeypress: move |e| {
                if is_pending() {
                    return;
                }
                let (panel, _) = *selected_panel.read();
                match e.data.code() {
                    Code::ArrowLeft if panel != Panels::Local => {
                        selected_panel.set((Panels::Local, Instant::now()));
                        start_animation();
                    }
                    Code::ArrowRight if panel == Panels::Local => {
                        selected_panel.set((*right_panel.read(), Instant::now()));
                        start_animation();
                    }
                    Code::ArrowUp => {
                        if panel == Panels::Local {
                            local_list.with_mut(|list| {
                                list.list_do_scroll(ScrollAction::Up);
                            });
                        } else if panel == Panels::LocalRight {
                            local_list_right.with_mut(|list| {
                                list.list_do_scroll(ScrollAction::Up);
                            });
                        } else {
                            cloud_list.with_mut(|list| {
                                list.list_do_scroll(ScrollAction::Up);
                            });
                        }
                    }
                    Code::ArrowDown => {
                        if panel == Panels::Local {
                            local_list.with_mut(|list| {
                                list.list_do_scroll(ScrollAction::Down);
                            });
                        } else if panel == Panels::LocalRight {
                            local_list_right.with_mut(|list| {
                                list.list_do_scroll(ScrollAction::Down);
                            });
                        } else {
                            cloud_list.with_mut(|list| {
                                list.list_do_scroll(ScrollAction::Down);
                            });
                        }
                    }
                    Code::KeyY => {
                        if *right_panel.read() == Panels::LocalRight {
                            right_panel.set(Panels::Cloud);
                            if panel != Panels::Local {
                                selected_panel.set((Panels::Cloud, Instant::now()));
                            }
                            if cloud_list.read().is_not_init() {
                                loading.write().show();
                                tokio::task::spawn_blocking(move || {
                                    if let Err(err) = cloud_enter_dir(cloud_list, "/", false, auth_state) {
                                        toast(format!("获取云端文件列表失败: {}", err));
                                    }
                                    loading.write().hide();
                                });
                            }
                        } else {
                            right_panel.set(Panels::LocalRight);
                            if panel != Panels::Local {
                                selected_panel.set((Panels::LocalRight, Instant::now()));
                            }
                            if local_list_right.read().is_not_init() {
                                loading.write().show();
                                tokio::task::spawn_blocking(move || {
                                    local_enter_dir(local_list_right, "/", false);
                                    loading.write().hide();
                                });
                            }
                        }
                    }
                    Code::KeyA => {
                        let (list, path) = {
                            let list = match panel {
                                Panels::Local => local_list,
                                Panels::LocalRight => local_list_right,
                                Panels::Cloud => cloud_list,
                            };
                            let r = list.read();
                            if !r.is_selected_item_dir() {
                                return;
                            }
                            (list, r.current_selected_abs_path())
                        };
                        loading.write().show();
                        tokio::task::spawn_blocking(move || {
                            if panel != Panels::Cloud {
                                local_enter_dir(list, &path, false);
                            } else {
                                if let Err(err) = cloud_enter_dir(list, &path, false, auth_state) {
                                    toast(format!("获取云端文件列表失败: {}", err));
                                }
                            }
                            loading.write().hide();
                        });
                    }
                    Code::KeyB => {
                        let mut list = match panel {
                            Panels::Local => local_list,
                            Panels::LocalRight => local_list_right,
                            Panels::Cloud => cloud_list,
                        };
                        if list.write().pop() && (panel == Panels::Local || panel == Panels::LocalRight) {
                            loading.write().show();
                            tokio::task::spawn_blocking(move || {
                                let path = list.read().current_abs_path();
                                local_enter_dir(list, &path, true);
                                loading.write().hide();
                            });
                        }
                    }
                    Code::KeyX => {
                        let list = match panel {
                            Panels::Local => local_list.read(),
                            Panels::LocalRight => local_list_right.read(),
                            Panels::Cloud => cloud_list.read(),
                        };
                        // 列表为空，只有新建文件夹
                        if !list.is_not_init() && list.total_items() == 0 {
                            let actions_list = vec![Actions::NewDir];
                            menu_list_state.write().update(actions_list.len() as i32);
                            actions.set(Some((actions_list, String::new(), "空文件夹".to_string())));
                        } else {
                            list.selected_item().map(|item| {
                                let actions_list = match &item {
                                    ChildItem::Local(name, is_dir) => {
                                        let mut res = vec![Actions::NewDir, Actions::Rename, Actions::Delete];
                                        if !*is_dir && name.to_lowercase().ends_with(".zip") {
                                            res.push(Actions::Unzip);
                                        } else {
                                            res.push(Actions::Zip);
                                        }
                                        if right_panel.try_read().is_ok_and(|r| *r == Panels::LocalRight) {
                                            res.push(Actions::Copy);
                                            res.push(Actions::Move);
                                        } else {
                                            if !*is_dir {
                                                res.push(Actions::Upload);
                                                if !name.to_lowercase().ends_with(".zip") {
                                                    res.push(Actions::ZipAndUpload);
                                                }
                                            } else {
                                                res.push(Actions::ZipAndUpload);
                                            }
                                        }
                                        if *is_dir {
                                            (res, name.to_string(), "文件夹".to_string())
                                        } else {
                                            let abs_path = list.current_selected_abs_path();
                                            if let Ok(size) = fs::File::open(abs_path).and_then(|f| f.metadata()).map(|m| m.size()) {
                                                let (p, unit) = storage_size_to_info(size as f64);
                                                (res, name.to_string(), format!("文件：{:.2} {}", size as f64 / p, unit))
                                            } else {
                                                (res, name.to_string(), "文件".to_string())
                                            }
                                        }
                                    }
                                    ChildItem::Cloud(name, _fs_id, is_dir, size) => {
                                        let mut res = vec![Actions::NewDir, Actions::Rename, Actions::Delete];
                                        if !is_dir {
                                            res.push(Actions::Download);
                                            if name.to_lowercase().ends_with(".cia") {
                                                res.push(Actions::InstallWithFBI);
                                            }
                                        }
                                        if *is_dir {
                                            (res, name.to_string(), "文件夹".to_string())
                                        } else {
                                            let (p, unit) = storage_size_to_info(*size as f64);
                                            (res, name.to_string(), format!("文件：{:.2} {}", *size as f64 / p, unit))
                                        }
                                    }
                                };
                                menu_list_state.write().update(actions_list.0.len() as i32);
                                actions.set(Some(actions_list));
                            });
                        }
                        dialog_visible.write().show();
                    }
                    _ => {}
                }
            },

            div {
                display: "flex",
                width: SCREEN_TOP_WIDTH,
                height: SCREEN_HEIGHT,
                padding: 2.0,
                background_color: "selected_bg",

                div {
                    flex: 1,
                    display: "flex",
                    flex_direction: "column",
                    padding: 5.0,
                    background_color: "main_bg",

                    if let Ok(list) = local_list.try_read() {
                        if list.total_items() > 0 {
                            for idx in 0i32..12i32 {
                                if let Some(item) = list.get(idx as usize) {
                                    div {
                                        display: "flex",
                                        align_items: "center",
                                        height: 18.83,
                                        padding_left: 1.0,
                                        background_color: if list.is_selected(idx) { "green" } else { "main_bg" },

                                        div {
                                            flex: 1,
                                            display: "flex",
                                            align_items: "center",
                                            height: 16.83,
                                            padding_left: 5.0,
                                            padding_top: 1.0,
                                            color: if item.is_dir() { "dir" } else { "main-text" },
                                            background_color: "main_bg",
                                            "{item.as_ref()}"
                                        }
                                    }
                                }
                            }
                        } else if !list.is_not_init() {
                            NoData {}
                        }
                    }
                }
            }

            div {
                "deep_3d": 0.0,
                position: "absolute",
                left: *right_panel_left.read(),
                top: 0,
                display: "flex",
                width: SCREEN_TOP_WIDTH,
                height: SCREEN_HEIGHT,
                padding: 2.0,
                background_color: "gray",

                div {
                    flex: 1,
                    display: "flex",
                    flex_direction: "column",
                    padding: 5.0,
                    background_color: "selected_bg_dark",

                    if let Ok(right_panel) = right_panel.try_read() {
                        if *right_panel == Panels::LocalRight {
                            if let Ok(list) = local_list_right.try_read() {
                                if list.total_items() > 0 {
                                    for idx in 0i32..12i32 {
                                        if let Some(item) = list.get(idx as usize) {
                                            div {
                                                display: "flex",
                                                align_items: "center",
                                                height: 18.83,
                                                padding_left: 1.0,
                                                background_color: if list.is_selected(idx) { "green" } else { "selected_bg_dark" },

                                                div {
                                                    flex: 1,
                                                    display: "flex",
                                                    align_items: "center",
                                                    height: 16.83,
                                                    padding_left: 5.0,
                                                    padding_top: 1.0,
                                                    color: if item.is_dir() { "dir" } else { "main-text" },
                                                    background_color: "selected_bg_dark",
                                                    "{item.as_ref()}"
                                                }
                                            }
                                        }
                                    }
                                } else if !list.is_not_init() {
                                    NoData {}
                                }
                            }
                        } else if let Ok(list) = cloud_list.try_read() {
                            if auth_state.read().0 {
                                if list.total_items() > 0 {
                                    for idx in 0i32..12i32 {
                                        if let Some(item) = list.get(idx as usize) {
                                            div {
                                                display: "flex",
                                                align_items: "center",
                                                height: 18.83,
                                                padding_left: 1.0,
                                                background_color: if list.is_selected(idx) { "green" } else { "selected_bg_dark" },

                                                div {
                                                    flex: 1,
                                                    display: "flex",
                                                    align_items: "center",
                                                    height: 16.83,
                                                    padding_left: 5.0,
                                                    padding_top: 1.0,
                                                    color: if item.is_dir() { "dir" } else { "main-text" },
                                                    background_color: "selected_bg_dark",
                                                    "{item.as_ref()}"
                                                }
                                            }
                                        }
                                    }
                                } else if !list.is_not_init() {
                                    NoData {}
                                }
                            } else {
                                div {
                                    flex: 1,
                                    display: "flex",
                                    align_items: "center",
                                    justify_content: "center",
                                    padding_right: 80.0,

                                    Auth {}
                                }
                            }
                        }


                    }
                }
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

            NavBar {
                is_pending: is_pending(),

                div {
                    margin_left: 20.0,

                    if selected_panel.read().0 != Panels::Cloud {
                        if let Some((free, total, unit)) = storage_info.read().0.map(|(free, total)| {
                            let (p, unit) = storage_size_to_info(total);
                            (free as f64 / p, total as f64 / p, unit)
                        }) {
                            "本地: {free:.2} / {total:.2} {unit}"
                        }
                    } else if let Some((free, total, unit)) = storage_info.read().1.map(|(free, total)| {
                            let (p, unit) = storage_size_to_info(total);
                            (free as f64 / p, total as f64 / p, unit)
                        }) {
                            "云盘: {free:.2} / {total:.2} {unit}"
                    }

                }
            }

            div {
                flex: 1,
                display: "flex",
                flex_direction: "column",
                padding: 6.0,
                gap: 5.0,

                div {
                    flex: 1,
                    padding: 10.0,
                    background_color: "panel_bg",
                    max_width: SCREEN_BOTTOM_WIDTH as f64 - 12.0,

                    if let Ok(list) = local_list.try_read() {
                        "左：本地  → {list.current_idx()} / {list.total_items()}"

                        div {
                            margin_top: 5.0,
                            "{list.current_abs_path()}"
                        }
                    }

                }

                if let Ok(panel) = right_panel.try_read() {
                    div {
                        flex: 1,
                        padding: 10.0,
                        background_color: "panel_bg",
                        max_width: SCREEN_BOTTOM_WIDTH as f64 - 12.0,

                        if *panel == Panels::LocalRight {
                            if let Ok(list) = local_list_right.try_read() {
                                "右：本地  → {list.current_idx()} / {list.total_items()}"

                                div {
                                    margin_top: 5.0,
                                    "{list.current_abs_path()}"
                                }
                            }
                        } else {
                            if let Ok(list) = cloud_list.try_read() {
                               "右：云盘  → {list.current_idx()} / {list.total_items()}"

                                div {
                                    margin_top: 5.0,
                                    "{list.current_abs_path()}"
                                }
                            }
                        }
                    }
                }
            }

            ActionBar {
                version: false,
                tips: "(START) 退出   (X) 操作   (Y) 切换   (B) 返回   (A) 选择",
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
                        actions,
                        onaction: do_action,
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

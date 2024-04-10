use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    ops::Deref,
    sync::{Mutex, OnceLock},
};

use ctru::services::fs::ArchiveID;
use dioxus::prelude::*;

use crate::{
    app::dialog::DialogVisible,
    platform::{
        is_exists_boss_game_save, is_exists_ext_game_save, is_exists_sys_game_save,
        is_exists_user_game_save,
    },
    resource::TitleInfo,
};

static TITLE_SAVE_TYPES_DATA: OnceLock<Mutex<HashMap<u64, u8>>> = OnceLock::new();

#[derive(Clone, Copy, PartialEq)]
pub enum SaveTypes {
    User,
    Ext,
    Sys,
    Boss,
}

impl SaveTypes {
    pub fn get_titles_save_types() -> &'static Mutex<HashMap<u64, u8>> {
        TITLE_SAVE_TYPES_DATA.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn get_title_save_type(title_id: u64) -> Option<u8> {
        if let Ok(lock) = Self::get_titles_save_types().lock() {
            lock.get(&title_id).copied()
        } else {
            None
        }
    }

    pub fn set_title_save_type(title_id: u64, save_type: u8) {
        if let Ok(mut lock) = Self::get_titles_save_types().lock() {
            lock.insert(title_id, save_type);
        }
    }

    pub fn arch_id(&self) -> ArchiveID {
        match self {
            SaveTypes::User => ArchiveID::UserSavedata,
            SaveTypes::Ext => ArchiveID::Extdata,
            SaveTypes::Sys => ArchiveID::SystemSavedata,
            SaveTypes::Boss => ArchiveID::BossExtdata,
        }
    }
}

impl Deref for SaveTypes {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            SaveTypes::User => "user",
            SaveTypes::Ext => "ext",
            SaveTypes::Sys => "sys",
            SaveTypes::Boss => "boss",
        }
    }
}

impl Display for SaveTypes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}

#[derive(Clone)]
pub struct TitleSelected {
    pub title: TitleInfo,
    pub save_type: Option<SaveTypes>,
    pub saves: Vec<SaveTypes>,
}

#[derive(Props, Clone, PartialEq)]
pub struct TitleSelectedProps {
    visible: Signal<DialogVisible>,
    onclick: EventHandler<()>,
}

impl TitleSelected {
    pub fn new(title: TitleInfo) -> Self {
        let mut res = TitleSelected {
            title,
            save_type: None,
            saves: vec![],
        };
        if let Some(save_type) = SaveTypes::get_title_save_type(title.id) {
            if save_type & 0b0001 != 0 {
                res.saves.push(SaveTypes::User);
            }
            if save_type & 0b0010 != 0 {
                res.saves.push(SaveTypes::Ext);
            }
            if save_type & 0b0100 != 0 {
                res.saves.push(SaveTypes::Sys);
            }
            if save_type & 0b1000 != 0 {
                res.saves.push(SaveTypes::Boss);
            }
            if !res.saves.is_empty() {
                res.save_type = Some(res.saves[0]);
            }
        } else {
            if is_exists_user_game_save(
                res.title.high_id(),
                res.title.low_id(),
                res.title.fs_media_type,
            ) {
                res.saves.push(SaveTypes::User);
            }
            if is_exists_ext_game_save(
                res.title.high_id(),
                res.title.low_id(),
                res.title.fs_media_type,
            ) {
                res.saves.push(SaveTypes::Ext);
            }
            if is_exists_sys_game_save(
                res.title.high_id(),
                res.title.low_id(),
                res.title.fs_media_type,
            ) {
                res.saves.push(SaveTypes::Sys);
            }
            if is_exists_boss_game_save(
                res.title.high_id(),
                res.title.low_id(),
                res.title.fs_media_type,
            ) {
                res.saves.push(SaveTypes::Boss);
            }
            if !res.saves.is_empty() {
                res.save_type = Some(res.saves[0]);
            }
        }
        res
    }
}

pub fn TitleSaveTypes(props: TitleSelectedProps) -> Element {
    let mut title_selected = use_context::<Signal<Option<TitleSelected>>>();

    rsx! {
        if let Some(info) = title_selected.read().as_ref().map(|s| s.clone()){
            div {
                display: "flex",
                padding_left: 21.0,
                align_items: "center",
                position: "relative",
                onkeypress: move |e| {
                    if props.visible.read().is_show() || (e.data.code() != Code::ControlLeft && e.data.code() != Code::ControlRight) {
                        return;
                    }
                    if let Some(idx) = info.saves
                                .iter()
                                .enumerate()
                                .find(|&(_, s)| info.save_type.is_some_and(|st| *s == st))
                                .map(|(idx, _)| {
                                    if e.data.code() == Code::ControlLeft {
                                        idx as i32 - 1
                                    } else {
                                        idx as i32 + 1
                                    }
                                })
                    {
                        if idx >= 0 && idx < info.saves.len() as i32 {
                            title_selected.with_mut(|selected| {
                                selected.as_mut().map(|selected| {
                                    selected.save_type = info.saves.get(idx as usize).map(|s| *s);
                                });
                            });
                        }
                    }
                },

                if info.saves.is_empty() {
                    div {
                        "没有存档数据"
                    }
                } else if info.saves.len() > 1 {
                    div {
                        "scale": 0.28,
                        color: "tips",
                        position: "absolute",
                        left: 12,
                        right: -11,
                        top: 21,
                        display: "flex",
                        justify_content: "space-between",

                        div {
                            display: "flex",
                            "ZL "
                            div {
                                margin_top: 2,
                                "←"
                            }
                        }

                        div {
                            display: "flex",

                            div {
                                margin_top: 2,
                                "→"
                            }
                            " ZR"
                        }
                    }
                }

                for (idx, save) in info.saves.iter().map(|s| s.clone()).enumerate() {
                    div {
                        display: "flex",
                        height: 24.0,
                        background_color: if info.save_type.is_some_and(|selected| selected == save)
                            { "selected_bg_light"}
                            else { "selected_bg" },
                        padding_left: 10.0,
                        padding_right: 10.0,
                        align_items: "center",
                        margin_left: if idx == 0 { 0.0 } else { 4.0 },
                        onclick: move |_| {
                            if props.visible.read().is_show() {
                                return;
                            }
                            title_selected.with_mut(|selected| {
                                selected.as_mut().map(|selected| {
                                    selected.save_type = Some(save);
                                });
                            });
                            props.onclick.call(());
                        },
                        "{save}"
                    }
                }
            }
        }
    }
}

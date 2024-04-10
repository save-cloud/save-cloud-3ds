use std::{
    collections::HashMap,
    error::Error,
    ffi::{c_char, c_int, c_uchar, c_uint, c_ulonglong, c_ushort, c_void, CStr},
    fs,
    io::Read,
    path::Path,
    slice,
};

use ctru::{
    applets::swkbd::{Button, ButtonConfig, CallbackResult, Kind, SoftwareKeyboard},
    error::ResultCode,
    services::{
        self,
        am::Am,
        fs::{ArchiveID, MediaType},
        romfs::RomFS,
        soc::Soc,
    },
};
use ctru_sys::{FS_CardType, CARD_CTR};
use dioxus::prelude::*;
use tokio::task::JoinError;

use crate::{
    app::titles::title_selected::SaveTypes,
    constant::{CACHE_ICON_NAME, FBI_SC_TITLE_ID, HOME_LOCAL_PATH_CACHE, INVALID_CHARS},
    fsu::{self, Archive},
    render::image_data_set::get_image_raw_buf,
    resource::{Resource, TitleInfo},
    utils::{join_path, str_to_c_null_term_bytes},
};

extern "C" {
    fn pl_get_smdh(id: c_ulonglong, media: u8) -> *const c_void;
    fn pl_free(smdh: *const c_void);
    fn pl_get_smdh_short_desc(smdh: *const c_void) -> *const c_char;
    /**
     * @brief Returns the Wifi signal strength.
     *
     * Valid values are 0-3:
     * - 0 means the singal strength is terrible or the 3DS is disconnected from
     *   all networks.
     * - 1 means the signal strength is bad.
     * - 2 means the signal strength is decent.
     * - 3 means the signal strength is good.
     *
     * Values outside the range of 0-3 should never be returned.
     *
     * These values correspond with the number of wifi bars displayed by Home Menu.
     *
     * @return the Wifi signal strength
     */
    fn pl_get_wifi_strength() -> c_uchar;
    fn pl_get_icon_buffer_from_smdh(smdh: *const c_void) -> *const c_void;
    fn pl_is_n3ds() -> bool;
    fn pl_commit_data(arch_id: c_uint, arch: c_ulonglong) -> c_int;
    fn pl_delete_sv(arch_id: c_uint, unique_id: c_uint) -> c_int;
    fn pl_open_title(title_id: c_ulonglong, media: u8, path: *const c_char) -> c_int;
    fn pl_env_is_homebrew() -> bool;
    fn pl_get_storage_info(free: *mut c_ulonglong, total: *mut c_ulonglong) -> c_int;
    // os function
    fn osSetSpeedupEnable(enable: bool);
}

pub struct SMDH {
    pub ptr: *const c_void,
}

impl SMDH {
    pub fn new(id: u64, media: u8) -> Option<Self> {
        unsafe {
            let ptr = pl_get_smdh(id, media);

            if ptr.is_null() {
                return None;
            }

            Some(Self { ptr })
        }
    }

    pub fn short_desc(&self) -> Option<String> {
        unsafe {
            let c_str = CStr::from_ptr(pl_get_smdh_short_desc(self.ptr));
            match c_str.to_str() {
                Ok(s) => Some(s.to_string()),
                Err(_) => None,
            }
        }
    }

    pub fn get_icon_buffer(&self) -> Vec<u16> {
        unsafe {
            let buffer = pl_get_icon_buffer_from_smdh(self.ptr);
            let icon_buffer = slice::from_raw_parts(buffer as *const c_ushort, 0x900);
            icon_buffer.to_vec()
        }
    }
}

impl Drop for SMDH {
    fn drop(&mut self) {
        unsafe {
            pl_free(self.ptr);
        }
    }
}

pub fn setup_log_redirect(soc: &mut Soc) -> Option<()> {
    // if !cfg!(debug_assertions) {
    //     return None;
    // }
    // Set the output to be redirected to the `3dslink` server.
    soc.redirect_to_3dslink(true, true).ok()
}

pub fn setup_romfs() -> Result<RomFS, Box<dyn Error>> {
    services::romfs::RomFS::new().map_err(|e| e.into())
}

pub fn is_exists_user_game_save(high_id: u32, low_id: u32, media: MediaType) -> bool {
    if media != MediaType::GameCard && media != MediaType::Sd {
        return false;
    }
    fsu::arch(ArchiveID::UserSavedata, media, high_id, low_id).is_ok()
}

pub fn is_exists_ext_game_save(high_id: u32, low_id: u32, media: MediaType) -> bool {
    fsu::arch(ArchiveID::Extdata, media, high_id, low_id).is_ok()
}

pub fn is_exists_sys_game_save(high_id: u32, low_id: u32, media: MediaType) -> bool {
    if media != MediaType::Nand {
        return false;
    }
    fsu::arch(ArchiveID::SystemSavedata, media, high_id, low_id).is_ok()
}

pub fn is_exists_boss_game_save(high_id: u32, low_id: u32, media: MediaType) -> bool {
    if media != MediaType::Nand {
        return false;
    }
    fsu::arch(ArchiveID::BossExtdata, media, high_id, low_id).is_ok()
}

pub fn is_nand_title_has_game_save(high_id: u32, low_id: u32) -> bool {
    is_exists_ext_game_save(high_id, low_id, MediaType::Nand)
        || is_exists_sys_game_save(high_id, low_id, MediaType::Nand)
        || is_exists_boss_game_save(high_id, low_id, MediaType::Nand)
}

pub fn is_sd_title_has_game_save(high_id: u32, low_id: u32) -> bool {
    is_exists_user_game_save(high_id, low_id, MediaType::Sd)
        || is_exists_ext_game_save(high_id, low_id, MediaType::Sd)
}

pub fn get_title_id_list_from_device(count: u32, mediatype: MediaType) -> ctru::Result<Vec<u64>> {
    if count == 0 {
        return Ok(vec![]);
    }
    let mut ids = vec![0; count as usize];
    let mut read_amount = 0;

    unsafe {
        ResultCode(ctru_sys::AM_GetTitleList(
            &mut read_amount,
            mediatype.into(),
            count,
            ids.as_mut_ptr(),
        ))?;
    }

    Ok(ids)
}

pub fn get_titles_save_types(ids: Vec<(u64, MediaType)>) {
    for (id, media) in ids.into_iter() {
        let (high_id, low_id) = ((id >> 32) as u32, id as u32);
        let mut res = 0u8;
        if is_exists_user_game_save(high_id, low_id, media) {
            res = res | 1;
        }
        if is_exists_ext_game_save(high_id, low_id, media) {
            res = res | 2;
        }
        if is_exists_sys_game_save(high_id, low_id, media) {
            res = res | 4;
        }
        if is_exists_boss_game_save(high_id, low_id, media) {
            res = res | 8;
        }
        SaveTypes::set_title_save_type(id, res);
    }
}

pub async fn get_title_list(mut percent: SyncSignal<f64>) -> Result<Vec<TitleInfo>, JoinError> {
    let titles = tokio::task::spawn_blocking(move || {
        // read titles
        let mut titles_ids = HashMap::new();
        let mut titles = vec![];
        if let Ok(am) = Am::new() {
            let card_type =
                pl_get_card_type().and_then(|t| if t == CARD_CTR { Some(t) } else { None });
            let mut current_count = 0;
            let card_count = if card_type.is_some() { 1 } else { 0 };
            let sd_count = am.title_count(MediaType::Sd).unwrap_or(0);
            let nand_count = am.title_count(MediaType::Nand).unwrap_or(0);
            let total_count = card_count + sd_count + nand_count;
            let card_ids = match card_type {
                Some(_) => get_title_id_list_from_device(card_count, MediaType::GameCard)
                    .unwrap_or_else(|_| {
                        current_count = card_count;
                        percent.set((current_count as f64 / total_count as f64) * 50.0);
                        vec![]
                    }),
                _ => vec![],
            };
            let sd_ids =
                get_title_id_list_from_device(sd_count, MediaType::Sd).unwrap_or_else(|_| {
                    current_count = sd_count;
                    percent.set((current_count as f64 / total_count as f64) * 50.0);
                    vec![]
                });
            let nand_ids = get_title_id_list_from_device(nand_count, MediaType::Nand)
                .unwrap_or_else(|_| {
                    percent.set(100.0);
                    vec![]
                });

            for id in card_ids {
                current_count += 1;
                if current_count % 2 == 0 {
                    percent.set((current_count as f64 / total_count as f64) * 50.0);
                }
                if TitleInfo::is_application(id) {
                    titles_ids.insert(id, true);
                    titles.push(TitleInfo::new(id, MediaType::GameCard));
                }
            }

            for id in sd_ids {
                current_count += 1;
                if current_count % 2 == 0 {
                    percent.set((current_count as f64 / total_count as f64) * 50.0);
                }
                if TitleInfo::is_application(id) {
                    titles_ids.insert(id, true);
                    titles.push(TitleInfo::new(id, MediaType::Sd));
                }
            }

            for id in nand_ids {
                current_count += 1;
                if current_count % 2 == 0 {
                    percent.set((current_count as f64 / total_count as f64) * 50.0);
                }
                if TitleInfo::is_application(id)
                    && is_nand_title_has_game_save((id >> 32) as u32, id as u32)
                {
                    titles_ids.insert(id, true);
                    titles.push(TitleInfo::new(id, MediaType::Nand));
                }
            }
        }

        // read icon cache
        let mut image_bufs: HashMap<u64, Vec<u16>> = HashMap::new();
        let icons_cache_path = join_path(HOME_LOCAL_PATH_CACHE, CACHE_ICON_NAME);
        if Path::new(&icons_cache_path).exists() {
            if let Ok(mut file) = fs::File::open(icons_cache_path) {
                let size = file.metadata().ok().map(|m| m.len()).unwrap_or(0);
                let mut read_size = 0;
                loop {
                    let mut id = [0u8; 8];
                    let mut buf = [0u8; 0x900 * 2];
                    if file.read_exact(&mut id).is_ok() && file.read_exact(&mut buf).is_ok() {
                        let id = u64::from_be_bytes(id);
                        // only read the icon of the title that exists
                        if titles_ids.contains_key(&id) {
                            unsafe {
                                let (_, buf, _) = buf.align_to::<u16>();
                                image_bufs.insert(id, buf.to_vec());
                            }
                        }
                        read_size += 8 + 0x900 * 2;
                        if size > 0 && (read_size / (8 + 0x900 * 2)) % 4 == 0 {
                            percent.set((read_size as f64 / size as f64) * 50.0 + 50.0);
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        if !image_bufs.is_empty() {
            if let Ok(mut m) = get_image_raw_buf().write() {
                *m = image_bufs;
            }
        }

        titles
    })
    .await;

    // get titles save types
    let _ = titles
        .as_ref()
        .map(|titles| {
            titles
                .iter()
                .map(|t| (t.id, t.fs_media_type))
                .collect::<Vec<(u64, MediaType)>>()
        })
        .map(|ids| {
            tokio::task::spawn_blocking(move || {
                get_titles_save_types(ids);
            });
        });

    titles
}

pub fn enable_hight_performance_for_new_3ds() {
    unsafe { osSetSpeedupEnable(true) }
}

pub fn get_wifi_strength() -> u8 {
    unsafe { pl_get_wifi_strength() }
}

pub fn is_new_3ds() -> bool {
    unsafe { pl_is_n3ds() }
}

pub fn pl_show_swkbd(kind: Kind, resource: &Resource, initial_text: &str) -> Option<String> {
    // Prepares a software keyboard with two buttons: one to cancel input and one
    // to accept it. You can also use `SoftwareKeyboard::new()` to launch the keyboard
    // with different configurations.
    let mut keyboard = SoftwareKeyboard::new(kind, ButtonConfig::LeftRight);

    // Custom filter callback to handle the given input.
    // Using this callback it's possible to integrate the applet
    // with custom error messages when the input is incorrect.
    keyboard.set_filter_callback(Some(Box::new(move |str| {
        for c in INVALID_CHARS.iter() {
            if str.contains(*c) {
                return (
                    CallbackResult::Retry,
                    Some(r#"不能包含此类字符: \ /:*?"'<>|"#.into()),
                );
            }
        }

        (CallbackResult::Ok, None)
    })));

    keyboard.set_initial_text(Some(initial_text));

    // Launch the software keyboard. You can perform different actions depending on which
    // software button the user pressed.
    match keyboard.launch(&resource.apt, &resource.c2d.gfx) {
        Ok((text, Button::Right)) => {
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        Ok((_, Button::Left)) => None,
        Ok((_, Button::Middle)) => None,
        Err(_) => None,
    }
}

pub fn pl_commit_arch_data(arch: &Archive) -> bool {
    unsafe { pl_commit_data(arch.id.into(), arch.handle) >= 0 }
}

pub fn pl_delete_arch_sv(arch: &Archive, unique_id: u32) -> bool {
    unsafe { pl_delete_sv(arch.id.into(), unique_id) >= 0 }
}

pub fn pl_open_the_title(title_id: u64, media: u8, path: &str) -> bool {
    let path = str_to_c_null_term_bytes(path);
    unsafe { pl_open_title(title_id, media, path.as_ptr()) >= 0 }
}

pub fn pl_is_homebrew() -> bool {
    unsafe { pl_env_is_homebrew() }
}

pub fn pl_storage_info() -> (u64, u64) {
    let mut free = 0;
    let mut total = 0;
    unsafe {
        pl_get_storage_info(&mut free, &mut total);
    }
    (free, total)
}

pub fn pl_is_fbi_title_exists() -> bool {
    SaveTypes::get_title_save_type(FBI_SC_TITLE_ID).is_some()
}

pub fn pl_get_card_type() -> Option<FS_CardType> {
    unsafe {
        let mut card_type = CARD_CTR;
        if ctru_sys::FSUSER_GetCardType(&mut card_type) >= 0 {
            Some(card_type)
        } else {
            None
        }
    }
}

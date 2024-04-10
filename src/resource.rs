use std::{cell::RefCell, error::Error, rc::Rc};

use ctru::{
    prelude::*,
    services::{am::Am, fs::MediaType},
};

use crate::{
    c2d::C2D,
    http::HttpContext,
    platform::{enable_hight_performance_for_new_3ds, is_new_3ds, setup_log_redirect},
};

#[derive(Clone, Copy, PartialEq)]
pub struct TitleInfo {
    pub id: u64,
    pub fs_media_type: MediaType,
}

impl TitleInfo {
    pub fn new(id: u64, fs_media_type: MediaType) -> Self {
        TitleInfo { id, fs_media_type }
    }

    pub fn media_str(&self) -> &'static str {
        if self.fs_media_type == MediaType::Sd {
            "sd"
        } else {
            "nand"
        }
    }

    pub fn high_id(&self) -> u32 {
        (self.id >> 32) as u32
    }

    pub fn low_id(&self) -> u32 {
        self.id as u32
    }

    pub fn id_hex_str(&self) -> String {
        format!("{:#X}", self.id)
    }

    pub fn category(&self) -> &'static str {
        match self.high_id() {
            // Application
            0x00040000 => "App",
            // Demo
            0x00040002 => "Demo",
            // System application
            0x00040010 => "System App",
            _ => "Unknown",
        }
    }

    pub fn product_code(&self) -> String {
        let mut buf: [u8; 16] = [0; 16];

        // This operation is safe as long as the title was correctly obtained via [`Am::title_list()`].
        if let Ok(_am) = Am::new() {
            unsafe {
                let _ = ctru_sys::AM_GetTitleProductCode(
                    self.fs_media_type.into(),
                    self.id,
                    buf.as_mut_ptr(),
                );
            }
        }

        String::from_utf8_lossy(
            &buf[0..buf
                .into_iter()
                .enumerate()
                .find(|&(_, u)| u == 0)
                .map(|(idx, _)| idx)
                .unwrap_or(buf.len())],
        )
        .to_string()
    }

    pub fn is_application(id: u64) -> bool {
        let high_id = (id >> 32) as u32;
        match high_id {
            // Application or Demo
            0x00040000 | 0x00040002 => true,
            // System application
            0x00040010 => match id as u32 {
                // system transfer
                0x00020A00 | 0x00021A00 | 0x00022A00 | 0x00027A00 | 0x00028A00 => false,
                // Instruction Manual
                0x00008602 | 0x00009202 | 0x00009B02 | 0x0000A402 | 0x0000AC02 | 0x0000B402 => {
                    false
                }
                // Internet Browser
                0x00008802 | 0x00009402 | 0x00009D02 | 0x0000A602 | 0x0000AE02 | 0x0000B602
                | 0x20008802 | 0x20009402 | 0x20009D02 | 0x2000AE02 => false,
                // system update
                0x00020F00 | 0x00021F00 | 0x00022F00 | 0x00026F00 | 0x00027F00 | 0x00028F00 => {
                    false
                }
                // eShop
                0x00020900 | 0x00021900 | 0x00022900 | 0x00027900 | 0x00028900 => false,
                // sd card manager
                0x20023100 | 0x20024100 | 0x20025100 => false,
                // Health and Safety Information
                0x20020300 | 0x20021300 | 0x20022300 | 0x20027300 => false,
                // Nintendo Network ID Settings
                0x0002BF00 | 0x0002C000 | 0x0002C100 => false,
                // download play
                0x00020100 | 0x00021100 | 0x00022100 | 0x00026100 | 0x00027100 | 0x00028100 => {
                    false
                }
                // Notifications digital manual
                0x2002CA00 | 0x002D300 | 0x2002D400 => false,
                // other system applications
                _ => true,
            },
            _ => false,
        }
    }
}

pub struct Resource {
    pub soc: Soc,
    pub hid: RefCell<Hid>,
    pub c2d: Rc<C2D>,
    pub apt: Apt,
    _http_context: HttpContext,
}

impl Resource {
    pub fn new(enable_log_redirect: bool) -> Result<Rc<Self>, Box<dyn Error>> {
        // enable high performance for new 3ds
        if is_new_3ds() {
            enable_hight_performance_for_new_3ds();
        }
        // enable socket for network
        let mut soc = Soc::new()?;
        // set log redirect
        if enable_log_redirect {
            setup_log_redirect(&mut soc);
        }
        // applet init
        let apt = Apt::new()?;
        // hid init
        let hid = Hid::new()?;
        // c2d init
        let c2d = Rc::new(C2D::new()?);
        // http init
        let http = HttpContext::new();

        Ok(Rc::new(Self {
            soc,
            hid: RefCell::new(hid),
            c2d,
            apt,
            _http_context: http,
        }))
    }

    pub fn main_loop(&self) -> bool {
        self.apt.main_loop()
    }
}

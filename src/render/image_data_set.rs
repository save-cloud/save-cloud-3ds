use std::{
    collections::HashMap,
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::Path,
    rc::Rc,
    sync::{Arc, OnceLock, RwLock},
};

use ctru::services::fs::MediaType;
use log::error;

use crate::{
    c2d::{c2d_load_icon_from_buffer, c2d_load_qrcode_from_buffer, C2dImage, C2dImageTrait},
    constant::{CACHE_ICON_NAME, HOME_LOCAL_PATH_CACHE},
    platform::SMDH,
    utils::join_path,
};

static IMAGE_RAW_BUFS: OnceLock<RwLock<HashMap<u64, Vec<u16>>>> = OnceLock::new();

pub fn get_image_raw_buf() -> &'static RwLock<HashMap<u64, Vec<u16>>> {
    IMAGE_RAW_BUFS.get_or_init(|| RwLock::new(HashMap::new()))
}

pub struct ImageDataSet {
    data: HashMap<u64, Option<Rc<C2dImage>>>,
    data_pending: Arc<RwLock<Vec<(u64, u8, Option<Vec<u16>>)>>>,
    // (use count, data)
    qrcode: (u16, HashMap<String, Rc<C2dImage>>),
    need_update: Arc<RwLock<bool>>,
}

impl ImageDataSet {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            data_pending: Arc::new(RwLock::new(vec![])),
            qrcode: (0, HashMap::new()),
            need_update: Arc::new(RwLock::new(false)),
        }
    }

    pub fn is_update(&mut self) -> bool {
        if let Ok(mut need_update) = self.need_update.write() {
            if *need_update {
                *need_update = false;
                return true;
            }
        }

        false
    }

    pub fn add_qrcode(&mut self, id: String, image: Rc<C2dImage>) {
        self.qrcode.1.insert(id, image);
    }

    pub fn get_qrcode(&mut self, id: &str) -> Option<Box<Rc<dyn C2dImageTrait>>> {
        self.qrcode.0 += 1;
        if let Some(image) = self.qrcode.1.get(id) {
            Some(Box::new(image.clone()))
        } else if let Ok(buf) =
            qrcode_generator::to_image(id, qrcode_generator::QrCodeEcc::Low, 128)
        {
            let image = Rc::new(c2d_load_qrcode_from_buffer(&buf));
            self.add_qrcode(id.to_string(), Rc::clone(&image));
            Some(Box::new(image as Rc<dyn C2dImageTrait>))
        } else {
            None
        }
    }

    pub fn release_qrcode(&mut self) {
        if self.qrcode.0 == 0 && !self.qrcode.1.is_empty() {
            self.qrcode.1.clear();
        }
        self.qrcode.0 = 0;
    }

    pub fn loading_missing_image(&mut self) {
        if Arc::strong_count(&self.data_pending) > 1 {
            return;
        }

        {
            if let Ok(data_pending) = self.data_pending.read() {
                if data_pending.is_empty() {
                    return;
                }
            }
        }

        let data_pending = Arc::clone(&self.data_pending);
        let need_update = Arc::clone(&self.need_update);
        tokio::task::spawn_blocking(move || {
            loop {
                if Arc::strong_count(&data_pending) == 1 {
                    break;
                }
                let mut item: Option<(u64, u8, Option<Vec<u16>>)> = None;
                {
                    if let Ok(data_pending) = data_pending.read() {
                        for (id, media, buf) in data_pending.iter() {
                            if buf.is_some() {
                                continue;
                            }
                            item = Some((*id, *media, None));
                            break;
                        }
                    }
                }
                if let Some((id, media, _)) = item {
                    let smdh = SMDH::new(id, media);
                    let mut image_buf = None;
                    {
                        if let Ok(mut data_pending) = data_pending.write() {
                            for (item_id, _media, buf) in data_pending.iter_mut() {
                                if id == *item_id {
                                    buf.replace(if let Some(smdh) = smdh {
                                        let buf = smdh.get_icon_buffer();
                                        image_buf = Some(buf.clone());
                                        buf
                                    } else {
                                        vec![]
                                    });
                                    break;
                                }
                            }
                        }
                    }
                    // trigger update
                    if let Ok(mut need_update) = need_update.write() {
                        *need_update = true;
                    }
                    if let Some(buf) = image_buf {
                        // write icon buf to cache
                        if !Path::new(HOME_LOCAL_PATH_CACHE).exists() {
                            create_dir_all(HOME_LOCAL_PATH_CACHE).ok();
                        }
                        let cache_file_path = join_path(HOME_LOCAL_PATH_CACHE, CACHE_ICON_NAME);
                        match OpenOptions::new()
                            .write(true)
                            .append(true)
                            .create(!Path::new(&cache_file_path).exists())
                            .open(cache_file_path)
                        {
                            Ok(mut file) => unsafe {
                                let (_, m, _) = buf.align_to::<u8>();
                                if file.write_all(&id.to_be_bytes()).is_ok() {
                                    file.write_all(m).ok();
                                }
                            },
                            Err(e) => {
                                error!("write icon buf to cache: {:?}", e);
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        });
    }

    pub fn get_image(&mut self, id: &str, media: &str) -> Option<Box<Rc<dyn C2dImageTrait>>> {
        if let Ok(id) = id.parse::<u64>() {
            if let Some(item) = self.data.get(&id) {
                item.as_ref()
                    .map(|image| Box::new(image.clone() as Rc<dyn C2dImageTrait>))
            } else {
                let mut find_img = (false, None);
                {
                    if let Ok(mut m) = get_image_raw_buf().write() {
                        if let Some(buf) = m.remove(&id) {
                            let image = Rc::new(c2d_load_icon_from_buffer(&buf));
                            self.data.insert(id, Some(image.clone()));
                            find_img = (true, Some(Box::new(image as Rc<dyn C2dImageTrait>)));
                        }
                    }
                }
                if !find_img.0 {
                    if let Ok(mut data_pending) = self.data_pending.write() {
                        let mut find_idx = None;
                        for (idx, (item_id, _, buf)) in data_pending.iter().enumerate() {
                            if id != *item_id {
                                continue;
                            }
                            find_idx = Some(idx);
                            if let Some(buf) = buf {
                                let item = if !buf.is_empty() {
                                    Some(Rc::new(c2d_load_icon_from_buffer(buf)))
                                } else {
                                    None
                                };
                                find_img = match &item {
                                    Some(item) => (
                                        true,
                                        Some(Box::new(Rc::clone(item) as Rc<dyn C2dImageTrait>)),
                                    ),
                                    None => (true, None),
                                };
                                self.data.insert(id, item);
                            }
                            break;
                        }
                        if let Some(find_idx) = find_idx {
                            if find_img.0 {
                                data_pending.remove(find_idx);
                            }
                        } else {
                            data_pending.push((
                                id,
                                if media == "sd" {
                                    MediaType::Sd as u8
                                } else {
                                    MediaType::Nand as u8
                                },
                                None,
                            ));
                        }
                    }
                }
                find_img.1
            }
        } else {
            None
        }
    }
}

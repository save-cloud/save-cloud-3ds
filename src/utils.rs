use std::{
    error::Error,
    ffi::{c_char, CStr},
    fmt::{Display, Formatter},
    fs,
    io::{self, Read, Write},
    ops::Deref,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose, Engine as _};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use zip::ZipWriter;

use crate::{
    c2d::rgba,
    constant::INVALID_CHARS,
    fsu::{self, Archive},
};

extern "C" {
    fn get_format_time() -> *mut c_char;
    fn free_c_str(data: *mut c_char);
}

pub struct CInputStr(*mut c_char);

impl Deref for CInputStr {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        unsafe {
            if self.0.is_null() {
                return "";
            }
            let c_str = CStr::from_ptr(self.0);
            c_str.to_str().unwrap()
        }
    }
}

impl Display for CInputStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.deref())
    }
}

impl Drop for CInputStr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { free_c_str(self.0) }
        }
    }
}

pub fn get_current_format_time() -> CInputStr {
    unsafe { CInputStr(get_format_time()) }
}

pub fn current_time() -> u128 {
    let start = SystemTime::now();
    start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}

pub fn normalize_path(path: &str) -> String {
    let mut path = path.to_string();
    for c in INVALID_CHARS.iter() {
        path = path.replace(*c, "_");
    }
    path.trim().to_string()
}

pub fn str_to_c_null_term_bytes(data: &str) -> Vec<u8> {
    format!("{}\0", data).into_bytes()
}

pub fn ease_out_expo(elapsed: Duration, duration: Duration, start: f64, end: f64) -> f64 {
    if elapsed >= duration {
        return end;
    }
    start
        + (end - start)
            * (1.0 - 2.0_f64.powf(-10.0 * elapsed.as_millis() as f64 / duration.as_millis() as f64))
}

pub fn get_active_color() -> u32 {
    let from = (168, 254, 255) as (i32, i32, i32);
    let to = (0, 168, 255) as (i32, i32, i32);
    let mut current = (0, 0, 0) as (i32, i32, i32);
    let p = (current_time() % 1000) as i32;

    if p < 400 {
        current.0 = from.0 + (to.0 - from.0) * p / 400;
        current.1 = from.1 + (to.1 - from.1) * p / 400;
        current.2 = from.2 + (to.2 - from.2) * p / 400;
    } else {
        current.0 = from.0 + (to.0 - from.0) * (1000 - p) / 600;
        current.1 = from.1 + (to.1 - from.1) * (1000 - p) / 600;
        current.2 = from.2 + (to.2 - from.2) * (1000 - p) / 600;
    }

    // TODO
    0
}

/// # get game save list of local dir
pub fn get_local_dir_start_with(path: &str, prefix: &str) -> Option<String> {
    let path = Path::new(path);
    if !path.exists() {
        return None;
    }
    if let Ok(dirs) = path.read_dir() {
        for entry in dirs.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = match path.file_name() {
                    Some(name) => name.to_str(),
                    None => None,
                } {
                    if name.starts_with(prefix) {
                        return Some(path.display().to_string());
                    }
                }
            }
        }
    }
    None
}

/// # get game save list of local dir
pub fn get_local_game_saves(path: &str) -> Vec<String> {
    let game_save_dir = Path::new(path);
    let mut list = vec![];
    if !game_save_dir.exists() {
        return list;
    }
    if let Ok(dirs) = game_save_dir.read_dir() {
        for entry in dirs.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = match path.file_name() {
                    Some(name) => name.to_str(),
                    None => None,
                } {
                    if name.ends_with(".zip") {
                        list.push(name.to_string());
                    }
                }
            }
        }
    }
    list.sort_by(|a, b| b.cmp(a));

    list
}

pub fn zip_dir_with(
    zip: &mut ZipWriter<fsu::File>,
    input_path: &Path,
    prefix: &str,
    arch: &Archive,
    back_list: &[&str],
    mut notify: impl FnMut(Option<String>, Option<String>) + Copy,
) -> Result<(), Box<dyn Error>> {
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for entry in fsu::read_dir(arch, input_path)?.flatten() {
        let path = entry.path();
        let name = path.strip_prefix(Path::new(prefix)).unwrap();
        if back_list
            .iter()
            .any(|&x| name.to_str().is_some_and(|name| x == name))
        {
            continue;
        }
        notify(None, Some(entry.file_name().to_string_lossy().to_string()));
        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if entry.metadata()?.is_file() {
            #[allow(deprecated)]
            zip.start_file_from_path(name, options)?;
            let mut input_file = fsu::File::open(arch, path)?;
            loop {
                let mut buffer = vec![0; 1024 * 512];
                let size = input_file.read(&mut buffer)?;
                if size == 0 {
                    break;
                }
                zip.write_all(&buffer[0..size])?;
            }
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options)?;
            zip_dir_with(zip, path.as_path(), prefix, arch, back_list, notify)?;
        }
    }

    Ok(())
}

pub fn zip_dir(
    from: (&str, &Archive),
    to: (&str, &Archive),
    back_list: &[&str],
    notify: impl FnMut(Option<String>, Option<String>) + Copy,
) -> Result<(), Box<dyn Error>> {
    let (from, from_arch) = from;
    let (to, to_arch) = to;
    let from = if from.ends_with('/') {
        from.to_string()
    } else {
        format!("{}/", from)
    };
    let output_path = Path::new(to);
    if !output_path.parent().unwrap().exists() {
        fsu::create_dir_all(to_arch, output_path.parent().unwrap())?;
    }
    let mut zip = zip::ZipWriter::new(fsu::File::create(&to_arch, output_path, None)?);
    let res = zip_dir_with(
        &mut zip,
        Path::new(&from),
        &from,
        from_arch,
        back_list,
        notify,
    )
    .map(|_| zip.finish());
    if let Err(e) = res {
        fs::remove_file(&output_path).ok();
        return Err(e);
    }
    Ok(())
}

pub fn zip_file(from: &str, name: &str, to: &str) -> Result<(), Box<dyn Error>> {
    let from_path = Path::new(from).join(name);
    let mut zip = zip::ZipWriter::new(fs::File::create(to)?);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut buffer = vec![0; 1024 * 512];
    #[allow(deprecated)]
    zip.start_file_from_path(Path::new(name), options)?;
    let mut input_file = fs::File::open(from_path)?;
    loop {
        let size = input_file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        zip.write_all(&buffer[0..size])?;
    }
    zip.finish()?;
    Ok(())
}

pub fn zip_extract(
    from: (impl AsRef<Path>, &Archive),
    to: (impl AsRef<Path>, &Archive),
    mut notify: impl FnMut(Option<String>, Option<String>) + Copy,
) -> Result<(), Box<dyn Error>> {
    let mut zip = zip::ZipArchive::new(fsu::File::open(&from.1, from.0)?)?;
    for i in 0..zip.len() {
        notify(Some(format!("正在解压 {}/{}", i + 1, zip.len())), None);
        let mut file_name = zip.by_index(i)?;
        let output_path = match file_name.enclosed_name() {
            Some(file_name) => {
                notify(None, Some(file_name.to_string_lossy().to_string()));
                to.0.as_ref().join(file_name).to_owned()
            }
            None => continue,
        };

        if (*file_name.name()).ends_with('/') {
            if !output_path.exists() {
                fsu::create_dir_all(&to.1, &output_path)?;
            }
        } else {
            if let Some(p) = output_path.parent() {
                if !p.exists() {
                    fsu::create_dir_all(&to.1, p)?;
                }
            }
            let mut output_file = fsu::File::create(&to.1, &output_path, Some(file_name.size()))?;
            copy_buf(&mut file_name, &mut output_file)?;
        }
    }

    Ok(())
}

pub fn copy_buf(from: &mut impl Read, to: &mut impl Write) -> io::Result<u64> {
    let mut total = 0;
    let mut cache = vec![0; 1024 * 512];
    let mut current_cache_size = 0;
    let mut buf = vec![0; 1024 * 128];
    loop {
        let size = from.read(&mut buf)?;
        if size == 0 {
            break;
        }
        total += size as u64;
        if current_cache_size + size > cache.len() {
            // fill gap
            let gap = cache.len() - current_cache_size;
            cache[current_cache_size..current_cache_size + gap].copy_from_slice(&buf[0..gap]);
            current_cache_size += gap;
            // write data
            to.write_all(&cache[0..current_cache_size])?;
            current_cache_size = 0;
            // fill rest
            cache[0..size - gap].copy_from_slice(&buf[gap..size]);
            current_cache_size += size - gap;
        } else {
            cache[current_cache_size..current_cache_size + size].copy_from_slice(&buf[0..size]);
            current_cache_size += size;
        }
        if current_cache_size == cache.len() {
            to.write_all(&cache[0..current_cache_size])?;
            current_cache_size = 0;
        }
    }
    if current_cache_size > 0 {
        to.write_all(&cache[0..current_cache_size])?;
    }
    Ok(total)
}

pub fn copy_file(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    let mut input_file = fs::File::open(from)?;
    let mut output_file = fs::File::create(to)?;
    copy_buf(&mut input_file, &mut output_file)
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<u64> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            copy_file(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(0)
}

pub fn join_path(base: &str, path: &str) -> String {
    if base.is_empty() || base.ends_with('/') {
        format!("{}{}", base, path)
    } else {
        format!("{}/{}", base, path)
    }
}

pub fn check_save_arch_is_empty(path: impl AsRef<Path>, arch: &Archive) -> bool {
    if let Ok(dirs) = fsu::read_dir(arch, path) {
        for entry in dirs.flatten() {
            if let Ok(m) = entry.metadata() {
                if m.is_file() {
                    return false;
                } else if m.is_dir() {
                    if !check_save_arch_is_empty(entry.path(), arch) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

pub fn backup_game_save(
    from: (&str, &Archive),
    to: (&str, &Archive),
    notify: impl FnMut(Option<String>, Option<String>) + Copy,
) -> Result<(), Box<dyn Error>> {
    zip_dir(from, to, &[], notify)
}

pub fn restore_game_save(
    from: (&str, &Archive),
    to: (&str, &Archive),
    mut notify: impl FnMut(Option<String>, Option<String>) + Copy,
) -> Result<(), Box<dyn Error>> {
    if !check_save_arch_is_empty("/", &to.1) {
        if let Some(from_parent) = Path::new(from.0).parent() {
            if let Some(auto_backup_path) = from_parent
                .join(format!("{} auto.zip", get_current_format_time()))
                .to_str()
            {
                notify(Some("正在自动备份".to_string()), None);
                let _ = backup_game_save(to, (auto_backup_path, from.1), notify);
            }
        }
    }

    // clear save dir
    if let Ok(dirs) = fsu::read_dir(&to.1, "/") {
        for entry in dirs.flatten() {
            if let Ok(m) = entry.metadata() {
                if m.is_file() {
                    fsu::remove_file(&to.1, entry.path())?;
                } else if m.is_dir() {
                    fsu::remove_dir_all(&to.1, entry.path())?;
                }
            }
        }
    }

    notify(Some("正在恢复存档".to_string()), None);

    // extract zip
    zip_extract(from, to, notify)
}

pub fn url_encode(param: &str) -> String {
    utf8_percent_encode(param, NON_ALPHANUMERIC).to_string()
}

pub fn base64_encode(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

pub fn base64_decode(data: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(general_purpose::STANDARD.decode(data)?)
}

pub fn get_str_md5(data: &[u8]) -> String {
    format!("{:x}", md5::compute(data))
}

pub fn delete_dir_if_empty(path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let path = path.as_ref();
    if path.exists() && path.is_dir() && path.read_dir()?.next().is_none() {
        fs::remove_dir(path)?;
    }
    Ok(())
}

pub fn create_parent_if_not_exists(path: &str) -> Result<(), Box<dyn Error>> {
    match Path::new(path).parent() {
        Some(parent) => {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        None => {}
    }
    Ok(())
}

pub fn color_name_rgba(color: &str) -> u32 {
    match color {
        // if there is a color tag, translate it
        "red" => rgba(0xff, 0x00, 0x00, 0xff),
        "green" => rgba(0x00, 0xff, 0x00, 0xff),
        "blue" => rgba(0x00, 0x00, 0xff, 0xff),
        "dir" => rgba(0x00, 0xb4, 0xd8, 255),
        "white" => rgba(0xff, 0xff, 0xff, 0xff),
        "gray" => rgba(0xbb, 0xbb, 0xbb, 0xff),
        "black" => rgba(0x00, 0x00, 0x00, 0xff),
        "main-text" => rgba(0xee, 0xee, 0xee, 0xff),
        "main_bg" => rgba(0x22, 0x22, 0x22, 0xff),
        "selected_bg" => rgba(0x44, 0x44, 0x44, 0xff),
        "selected_bg_info" => rgba(0x33, 0x33, 0x33, 0xff),
        "selected_bg_dark" => rgba(0x28, 0x28, 0x28, 0xff),
        "selected_bg_light" => rgba(0x60, 0x60, 0x60, 0xff),
        "transparent" => rgba(0x0, 0x0, 0x0, 0x0),
        "tips" => rgba(0xaa, 0xaa, 0xaa, 0xff),
        "panel_bg" => rgba(0x26, 0x26, 0x26, 0xff),
        _ => rgba(0x00, 0x00, 0x00, 0xff),
    }
}

pub async fn sleep_micros(micros: u64) {
    tokio::time::sleep(Duration::from_micros(micros)).await;
}

pub async fn sleep_micros_for_ever(micros: u64) {
    loop {
        tokio::time::sleep(Duration::from_micros(micros)).await;
    }
}

pub fn storage_size_to_info(size: f64) -> (f64, &'static str) {
    if size > 1024.0 * 1024.0 * 1024.0 {
        (1024.0 * 1024.0 * 1024.0, "G")
    } else if size > 1024.0 * 1024.0 {
        (1024.0 * 1024.0, "M")
    } else if size > 1024.0 {
        (1024.0, "K")
    } else {
        (1.0, "B")
    }
}

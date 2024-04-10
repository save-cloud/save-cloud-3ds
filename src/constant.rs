// screen
pub const SCREEN_TOP_WIDTH: i64 = 400;
pub const SCREEN_BOTTOM_WIDTH: i64 = 320;
pub const SCREEN_HEIGHT: i64 = 240;

// color
pub const SELECTED_BG_COLOR: i64 = 4281874488;
pub const TIPS_COLOR_LIGHT: i64 = 4288256409;
pub const TIPS_COLOR: i64 = 4284900966;
pub const MAX_DEEP_3D: f32 = 5.0;

// buffer size
pub const DOWNLOAD_BUF_SIZE: usize = 1024 * 512; // 512kib;
pub const UPLOAD_SLICE_PER_SIZE: usize = 1024 * 1024 * 4; // 4 MiB

// invalid path chars
pub const INVALID_CHARS: [char; 10] = ['\\', '/', ':', '*', '?', '"', '\'', '<', '>', '|'];

// paths
pub const HOME_LOCAL_PATH_SAVE: &str = "/save-cloud/save";
pub const HOME_LOCAL_PATH_CACHE: &str = "/save-cloud/cache";
pub const CACHE_ICON_NAME: &str = "icons.bin";
pub const AUTH_BAIDU_CONFIG_PATH: &str = "/save-cloud/auth";
pub const GAME_SAVE_CLOUD_DIR_PREFIX: &str = "/apps/Backup/";
pub const GAME_SAVE_CLOUD_DIR_ROOT: &str = "/apps/Backup/3ds/save-cloud";
pub const GAME_SAVE_CLOUD_DIR: &str = "/apps/Backup/3ds/save-cloud/saves";
pub const UPLOAD_CACHE_DIR: &str = "/apps/Backup/upload_cache_can_delete";

// home page
pub const HOME_PAGE_URL: &str = "https://save-cloud.sketchraw.com?3ds=go";
pub const INVALID_EAT_PANCAKE: &str = "缺少 eat.pancake";
pub const ABOUT_TEXT: &str = "Save Cloud 云存档，扫码访问主页！";
pub const CURL_CERT_CURL: &str = "https://curl.se/ca/cacert.pem";
pub const CURL_CERT_PATH: &str = "/config/ssl/cacert.pem";
pub const FBI_SC_TITLE_ID: u64 = 0x400000F899900;

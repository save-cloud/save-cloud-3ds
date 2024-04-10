use std::{thread::sleep, time::Duration};

use dioxus::prelude::*;
use log::{error, info};

use crate::{
    api::{Api, AuthData},
    app::{loading::Loading, tips::TipsVisible, AuthState},
};

pub fn Auth() -> Element {
    let mut auth_state = use_context::<SyncSignal<AuthState>>();
    let mut tips_visible = use_context::<SyncSignal<TipsVisible>>();
    let mut qrcode_url = use_signal_sync::<Option<String>>(|| None);
    let mut toast = move |text: String| {
        if let Ok(mut visible) = tips_visible.try_write() {
            visible.show(Some(text));
        }
    };

    use_effect(move || {
        if Api::get_read().is_login() {
            return;
        }
        tokio::task::spawn_blocking(move || {
            sleep(Duration::from_millis(300));
            let api_type = Api::get_read().api_type;
            let auth_url = Api::get_read().get_auth_url();
            let device_code = match Api::start_auth(&auth_url, api_type) {
                Ok(auth_res) => {
                    let qr_url = Api::get_read()
                        .get_qr_code_url(&auth_res.user_code.expect("auth user code"));
                    if let Ok(mut qrcode_url) = qrcode_url.try_write() {
                        qrcode_url.replace(qr_url);
                    }
                    auth_res.device_code
                }
                Err(err) => {
                    error!("auth error: {:?}", err);
                    toast(format!("获取授权失败"));
                    None
                }
            };

            while let Some(device_code) = &device_code {
                let get_token_url = Api::get_read().get_token_url(device_code);
                match Api::start_fetch_token(&get_token_url, api_type) {
                    Ok(token_res) => {
                        // 更新登录状态
                        match Api::start_fetch_name_of_pancake(
                            token_res.access_token.as_ref().unwrap(),
                        ) {
                            Ok(name_of_pancake) => {
                                Api::update_auth_data(
                                    api_type,
                                    Some(AuthData::new(token_res, name_of_pancake)),
                                );
                                if let Ok(mut auth_state) = auth_state.try_write() {
                                    auth_state.0 = true;
                                }
                                toast("登录成功！".to_string());
                            }
                            Err(err) => {
                                error!("fetch profile failed: {:?}", err);
                                toast("登录失败，获取用户信息失败！".to_string());
                            }
                        }
                        break;
                    }
                    Err(err) => {
                        info!("fetch token failed: {:?}", err);
                    }
                }

                // wait 6s
                sleep(Duration::from_secs(6));

                if qrcode_url.try_read().is_err() {
                    break;
                }
            }
        });
    });

    rsx! {
        if let Some(url) = qrcode_url.read().as_ref() {
            div {
                display: "flex",
                flex_direction: "column",
                justify_content: "center",
                align_items: "center",

                img {
                    "scale": 1,
                    "media": "qrcode",
                    src: "{url}",
                    width: 128.0,
                    height: 128.0,
                    margin_bottom: 10.0,
                }

                "百度云 App 扫码登录"
            }
        } else {
            Loading {
                width: 36.0,
                height: 36.0,
            }
        }
    }
}

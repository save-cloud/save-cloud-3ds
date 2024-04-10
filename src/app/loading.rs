#![allow(non_snake_case)]

use std::time::{Duration, Instant};

use dioxus::prelude::*;

use crate::{
    c2d::rgba,
    constant::{SCREEN_BOTTOM_WIDTH, SCREEN_HEIGHT},
    utils::{ease_out_expo, sleep_micros},
};

#[derive(Clone)]
pub struct PageLoadingVisible {
    pub visible: bool,
    at: Instant,
    top: f64,
    from_top: f64,
    to_top: f64,
    title: Option<String>,
    desc: Option<String>,
    pub download_progress: SyncSignal<Option<(f64, f64, f64, String)>>,
}

impl PageLoadingVisible {
    pub fn visible(&self) -> bool {
        self.visible
    }

    fn set_top(&mut self, top: f64) {
        self.top = top;
    }

    pub fn show(&mut self) {
        self.at = Instant::now();
        self.visible = true;
    }

    pub fn show_info(&mut self, title: Option<String>, desc: Option<String>) {
        if title.is_some() {
            self.title = title;
        }
        if desc.is_some() {
            self.desc = desc;
        }
    }

    pub fn hide(&mut self) {
        self.at = Instant::now();
        self.visible = false;
        self.title = None;
        self.desc = None;
        self.download_progress.write().take();
    }

    pub fn is_show(&self) -> bool {
        self.visible || self.top < self.from_top
    }
}

pub fn use_page_loading(
    visible: bool,
    from_top: f64,
    to_top: f64,
) -> SyncSignal<PageLoadingVisible> {
    let download_progress = use_signal_sync(|| None);
    use_signal_sync(|| PageLoadingVisible {
        visible,
        at: Instant::now(),
        top: from_top,
        from_top,
        to_top,
        title: None,
        desc: None,
        download_progress,
    })
}

#[derive(Props, Clone, PartialEq)]
pub struct LoadingProps {
    left: Option<f64>,
    top: Option<f64>,
    width: f64,
    height: f64,
}

pub fn Loading(props: LoadingProps) -> Element {
    let LoadingProps {
        left,
        top,
        width,
        height,
    } = props;
    let mut active = use_signal(|| 0i8);
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            if *active.read() < 3 {
                active += 1;
            } else {
                *active.write() = 0;
            }
        }
    });
    let range = (0..4).collect::<Vec<i8>>();
    let list = range.iter().map(|&idx| {
        let idx = if idx == 2 {
            3
        } else if idx == 3 {
            2
        } else {
            idx
        };
        let active = *active.read();
        let (color, deep_3d) = if idx == active {
            (rgba(0x99, 0x99, 0x99, 0xbb), 0.0)
        } else {
            let pre = if active > 0 { active - 1 } else { 3 };
            let pre1 = if pre > 0 { pre - 1 } else { 3 };
            if idx == pre {
                (rgba(0x88, 0x88, 0x88, 0xaa), 1.0)
            } else if idx == pre1 {
                (rgba(0x77, 0x77, 0x77, 0x77), 1.0)
            } else {
                (rgba(0x66, 0x66, 0x66, 0x66), 1.0)
            }
        };

        rsx! {
            div {
                "deep_3d": deep_3d,
                key: "{idx}",
                width: width / 2.0,
                height: height / 2.0,
                background_color: color as i64,
            }
        }
    });

    rsx! {
        if let (Some(left), Some(top)) = (left, top) {
            div {
                position: "absolute",
                left,
                top,
                display: "flex",
                flex_wrap: "wrap",
                width: width,
                height: height,

                {list}
            }
        } else {
            div {
                display: "flex",
                flex_wrap: "wrap",
                width: width,
                height: height,

                {list}
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct PageLoadingProps {
    visible: SyncSignal<PageLoadingVisible>,
}

pub fn PageLoading(mut props: PageLoadingProps) -> Element {
    use_future(move || async move {
        loop {
            if let Some(PageLoadingVisible {
                visible,
                from_top,
                to_top,
                at,
                top,
                ..
            }) = props
                .visible
                .try_read()
                .ok()
                .map(|v| PageLoadingVisible { ..(*v).clone() })
            {
                let top_new = if visible {
                    ease_out_expo(
                        Instant::now().duration_since(at),
                        Duration::from_millis(300),
                        from_top,
                        to_top,
                    )
                } else {
                    ease_out_expo(
                        Instant::now().duration_since(at),
                        Duration::from_millis(300),
                        to_top,
                        from_top,
                    )
                };

                if top != top_new {
                    if let Ok(mut visible) = props.visible.try_write() {
                        visible.set_top(top_new);
                    }
                }
            }

            sleep_micros(16000).await;
        }
    });

    rsx! {
        if let Some(PageLoadingVisible {
            top,
            title,
            desc,
            download_progress,
            ..
        }) = props
            .visible
            .try_read()
            .ok()
            .map(|v| PageLoadingVisible { ..(*v).clone() })
        {
            Loading {
                left: Some(10.0),
                top: Some(top),
                width: 36.0,
                height: 36.0,
            }

            if let Some((progress, current, total, unit)) = download_progress
                .try_read().ok()
                .and_then(|d| d.as_ref().map(|d| (d.0, d.1, d.2, d.3.clone())))
            {
                div {
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    position: "absolute",
                    left: 0.0,
                    top: 0.0,
                    width: SCREEN_BOTTOM_WIDTH as f64,
                    height: SCREEN_HEIGHT as f64,

                    div {
                        display: "flex",
                        flex_direction: "column",
                        width: 260.0,
                        height: 80.0,
                        color: "white",
                        background_color: "selected_bg",
                        padding: 5.0,

                        if let Some(title) = title {
                            div {
                                "{title}"
                            }
                        }

                        div {
                            flex: 1,
                            display: "flex",
                            align_items: "center",
                            justify_content: "center",

                            "下载中：{progress:.2}% ({current:.2}/{total:.2} {unit})"
                        }
                    }
                }
            } else if desc.is_some() {
                div {
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    position: "absolute",
                    left: 0.0,
                    top: 0.0,
                    width: SCREEN_BOTTOM_WIDTH as f64,
                    height: SCREEN_HEIGHT as f64,

                    div {
                        display: "flex",
                        flex_direction: "column",
                        width: 260.0,
                        height: 80.0,
                        color: "white",
                        background_color: "selected_bg",
                        padding: 5.0,

                        if let Some(desc) = desc {
                            if let Some(title) = title {
                                div {
                                    "{title}"
                                }
                            }

                            div {
                                flex: 1,
                                display: "flex",
                                align_items: "center",
                                justify_content: "center",
                                "{desc}"
                            }
                        }
                    }
                }
            }
        }
    }
}

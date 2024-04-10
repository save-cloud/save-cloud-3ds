use std::time::{Duration, Instant};

use dioxus::prelude::*;

use crate::utils::{ease_out_expo, sleep_micros};

#[derive(Clone)]
pub struct TipsVisible {
    visible: bool,
    at: Instant,
    top: f64,
    from_top: f64,
    to_top: f64,
    text: Option<String>,
}

impl TipsVisible {
    pub fn visible(&self) -> bool {
        self.visible
    }

    fn set_top(&mut self, top: f64) {
        self.top = top;
    }

    pub fn show(&mut self, text: Option<String>) {
        self.at = Instant::now();
        self.visible = true;
        self.top = self.from_top;
        self.text = text;
    }

    pub fn hide(&mut self) {
        self.at = Instant::now();
        self.visible = false;
    }

    pub fn is_show(&self) -> bool {
        self.visible || self.top < self.from_top
    }
}

pub fn use_tips(visible: bool, from_top: f64, to_top: f64) -> SyncSignal<TipsVisible> {
    use_signal_sync(|| TipsVisible {
        visible,
        at: Instant::now(),
        top: from_top,
        from_top,
        to_top,
        text: None,
    })
}

#[derive(Props, Clone, PartialEq)]
pub struct TipsProps {
    visible: SyncSignal<TipsVisible>,
    #[props(default = String::from("bottom"))]
    screen: String,
    #[props(default = String::from("selected_bg_light"))]
    background_color: String,
}

pub fn Tips(mut props: TipsProps) -> Element {
    use_future(move || async move {
        loop {
            if let Some(TipsVisible {
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
                .map(|v| TipsVisible { ..(*v).clone() })
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
                } else if Instant::now().duration_since(at).as_millis() > 3000 {
                    if let Ok(mut visible) = props.visible.try_write() {
                        visible.hide();
                    }
                }
            }

            sleep_micros(16000).await;
        }
    });

    rsx! {
        if let Some(TipsVisible {
            top,
            text,
            ..
        }) = props
            .visible
            .try_read()
            .ok()
            .map(|v| TipsVisible { ..(*v).clone() })
        {
            div {
                "screen": props.screen.to_string(),
                display: "flex",
                position: "absolute",
                top,
                left: 10.0,
                align_items: "center",
                padding_top: 4.0,
                padding_bottom: 2.0,
                padding_left: 4.0,
                padding_right: 4.0,
                color: "white",
                background_color: props.background_color,

                if let Some(text) = text {
                    "{text}"
                }
            }
        }
    }
}

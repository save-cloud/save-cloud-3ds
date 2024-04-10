use std::time::{Duration, Instant};

use dioxus::prelude::*;

use crate::utils::{ease_out_expo, sleep_micros};

pub struct DialogVisible {
    pub visible: bool,
    at: Instant,
    top: f64,
    from_top: f64,
    to_top: f64,
    width: f64,
    height: f64,
    duration: Duration,
}

impl DialogVisible {
    pub fn visible(&self) -> bool {
        self.visible
    }

    fn at(&self) -> Instant {
        self.at
    }

    fn top(&self) -> f64 {
        self.top
    }

    fn set_top(&mut self, top: f64) {
        self.top = top;
    }

    pub fn show(&mut self) {
        self.at = Instant::now();
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.at = Instant::now();
        self.visible = false;
    }

    pub fn is_show(&self) -> bool {
        self.visible || self.top < self.from_top
    }
}

pub fn use_dialog(
    visible: bool,
    from_top: f64,
    to_top: f64,
    width: f64,
    height: f64,
    duration: Option<Duration>,
) -> Signal<DialogVisible> {
    use_signal(|| DialogVisible {
        visible,
        at: Instant::now(),
        top: from_top,
        from_top,
        to_top,
        width,
        height,
        duration: duration.unwrap_or(Duration::from_millis(300)),
    })
}

#[derive(Props, Clone, PartialEq)]
pub struct DialogProps {
    visible: Signal<DialogVisible>,
    #[props(default = String::from("bottom"))]
    screen: String,
    #[props(default = String::from("main_bg"))]
    background_color: String,
    children: Element,
}

pub fn Dialog(mut props: DialogProps) -> Element {
    use_future(move || async move {
        loop {
            let DialogVisible {
                visible,
                from_top,
                to_top,
                duration,
                ..
            } = *props.visible.read();
            let top_new = if visible {
                ease_out_expo(
                    Instant::now().duration_since(props.visible.read().at()),
                    duration,
                    from_top,
                    to_top,
                )
            } else {
                ease_out_expo(
                    Instant::now().duration_since(props.visible.read().at()),
                    duration,
                    to_top,
                    from_top,
                )
            };

            if props.visible.read().top() != top_new {
                props.visible.write().set_top(top_new);
            }

            sleep_micros(16000).await;
        }
    });

    rsx! {
        div {
            "screen": props.screen.to_string(),
            display: "flex",
            position: "absolute",
            top: props.visible.read().top(),
            left: 0,
            width: props.visible.read().width,
            height: props.visible.read().height,
            background_color: props.background_color,
            onmousedown: |_| {
                // prevent click through dialog
            },
            onmouseup: |_| {
                // prevent click through dialog
            },
            onclick: |_| {
                // prevent click through dialog
            },

            {props.children}
        }
    }
}

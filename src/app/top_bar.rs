#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::{
    app::{line::Line, Panel},
    constant::SELECTED_BG_COLOR,
};

#[component]
pub fn NavItem(is_pending: bool) -> Element {
    let mut selected_panel = use_context::<Signal<Panel>>();

    let left = match *selected_panel.read() {
        Panel::Device => 9,
        _ => 46,
    };

    rsx! {
        div {
          position: "relative",

          div {
              position: "absolute",
              left: left,
              top: 1,
              width: 32,
              height: 32,
              background_color: SELECTED_BG_COLOR,
              onkeypress: move |e| {
                  if is_pending{
                      return;
                  }
                  let current_panel = *selected_panel.read();
                  match e.data.code() {
                      Code::KeyL if current_panel != Panel::Device => {
                          *selected_panel.write() = Panel::Device;
                      }
                      Code::KeyR if current_panel != Panel::Cloud => {
                          *selected_panel.write() = Panel::Cloud;
                      }
                      _ => {}
                  }
              },
          }

          div {
            "scale": 0.28,
            position: "absolute",
            display: "flex",
            padding_top: 26,
            width: 86,
            height: 34,

            div {
              display: "flex",
              flex: 1,
              color: "tips",
              margin_left: 1.0,
              "L ",

              div {
                  margin_top: 2,
                  "←"
              }
            }

            div {
              display: "flex",
              color: "tips",

              div {
                  margin_top: 2,
                  "→"
              }

              " R",
            }
          }

          div {
              display: "flex",
              justify_content: "space-around",
              align_items: "center",
              width: 86,
              height: 34,
              padding_left: 6,
              padding_right: 6,

              img {
                  "scale": 1,
                  src: 1,
                  width: 24,
                  height: 24,
                  onclick: move |_| {
                      if is_pending || *selected_panel.read() == Panel::Device {
                          return;
                      }
                      *selected_panel.write() = Panel::Device;
                  }
              }

              img {
                  "scale": 1,
                  src: 2,
                  width: 24,
                  height: 24,
                  onclick: move |_| {
                      if is_pending || *selected_panel.read() == Panel::Cloud {
                          return;
                      }
                      *selected_panel.write() = Panel::Cloud;
                  }
              }
          }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct NavProps {
    #[props(optional)]
    children: Element,
    is_pending: bool,
}

pub fn NavBar(props: NavProps) -> Element {
    rsx! {
        div {
            display: "flex",
            height: 34,
            align_items: "center",

            // bg image
            img {
              "scale": 1,
              src: 3,
              position: "absolute",
              left: 0,
              top: 0,
              width: 320,
              height: 34
            }

            // icon image
            img {
                "scale": 1,
                src: 0,
                width: 24,
                height: 24,
                margin_left: 6,
            }

            // device and cloud image
            NavItem {
                is_pending: props.is_pending
            }


            // right area
            {props.children}
        }

        Line {}
    }
}

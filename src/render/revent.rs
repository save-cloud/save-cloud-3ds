use std::{
    any::Any,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Instant,
};

use ctru::{os::current_3d_slider_state, services::hid::KeyPad};
use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_elements::{
    geometry::{euclid::Point2D, ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
};
use dioxus_native_core::{prelude::*, tree::TreeRef};
use taffy::TaffyTree;

use crate::{
    app::AppExit, render::rdom::taffy_layout::TaffyLayout, resource::Resource, utils::sleep_micros,
};

use super::ImageDataSet;

#[derive(Clone)]
pub struct KeyEventData(pub Key, pub Code);

#[derive(Clone)]
pub struct MouseEventData(u16, u16);

fn downcast_key_event(event: &PlatformEventData) -> KeyEventData {
    event
        .downcast::<KeyEventData>()
        .expect("event should be of type key EventData")
        .clone()
}

fn downcast_mouse_event(event: &PlatformEventData) -> MouseEventData {
    event
        .downcast::<MouseEventData>()
        .expect("event should be of type mouse EventData")
        .clone()
}

pub(crate) struct SerializedHtmlEventConverter;

impl HtmlEventConverter for SerializedHtmlEventConverter {
    fn convert_animation_data(&self, _: &PlatformEventData) -> AnimationData {
        panic!("animation events not supported")
    }

    fn convert_clipboard_data(&self, _: &PlatformEventData) -> ClipboardData {
        panic!("clipboard events not supported")
    }

    fn convert_composition_data(&self, _: &PlatformEventData) -> CompositionData {
        panic!("composition events not supported")
    }

    fn convert_drag_data(&self, _: &PlatformEventData) -> DragData {
        panic!("drag events not supported")
    }

    fn convert_focus_data(&self, _: &PlatformEventData) -> FocusData {
        panic!("event should be of type Focus")
    }

    fn convert_form_data(&self, _: &PlatformEventData) -> FormData {
        panic!("event should be of type Form")
    }

    fn convert_image_data(&self, _: &PlatformEventData) -> ImageData {
        panic!("image events not supported")
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData {
        KeyboardData::new(downcast_key_event(event))
    }

    fn convert_media_data(&self, _: &PlatformEventData) -> MediaData {
        panic!("media events not supported")
    }

    fn convert_mounted_data(&self, _: &PlatformEventData) -> MountedData {
        panic!("mounted events not supported")
    }

    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        MouseData::new(downcast_mouse_event(event))
    }

    fn convert_pointer_data(&self, _: &PlatformEventData) -> PointerData {
        panic!("pointer events not supported")
    }

    fn convert_scroll_data(&self, _: &PlatformEventData) -> ScrollData {
        panic!("scroll events not supported")
    }

    fn convert_selection_data(&self, _: &PlatformEventData) -> SelectionData {
        panic!("selection events not supported")
    }

    fn convert_toggle_data(&self, _: &PlatformEventData) -> ToggleData {
        panic!("toggle events not supported")
    }

    fn convert_touch_data(&self, _: &PlatformEventData) -> TouchData {
        panic!("touch events not supported")
    }

    fn convert_transition_data(&self, _: &PlatformEventData) -> TransitionData {
        panic!("transition events not supported")
    }

    fn convert_wheel_data(&self, _: &PlatformEventData) -> WheelData {
        panic!("wheel events not supported")
    }
}

impl ModifiersInteraction for MouseEventData {
    fn modifiers(&self) -> Modifiers {
        Modifiers::default()
    }
}

impl InteractionElementOffset for MouseEventData {
    fn coordinates(&self) -> Coordinates {
        Coordinates::new(
            Point2D::new(self.0 as f64, self.1 as f64),
            Point2D::new(self.0 as f64, self.1 as f64),
            Point2D::new(self.0 as f64, self.1 as f64),
            Point2D::new(self.0 as f64, self.1 as f64),
        )
    }

    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.0 as f64, self.1 as f64)
    }
}

impl InteractionLocation for MouseEventData {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.0 as f64, self.1 as f64)
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.0 as f64, self.1 as f64)
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.0 as f64, self.1 as f64)
    }
}

impl PointerInteraction for MouseEventData {
    fn trigger_button(&self) -> Option<MouseButton> {
        None
    }

    fn held_buttons(&self) -> MouseButtonSet {
        MouseButtonSet::default()
    }
}

impl HasMouseData for MouseEventData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ModifiersInteraction for KeyEventData {
    fn modifiers(&self) -> Modifiers {
        Modifiers::default()
    }
}

impl HasKeyboardData for KeyEventData {
    fn key(&self) -> Key {
        self.0.clone()
    }

    fn code(&self) -> Code {
        self.1
    }

    fn location(&self) -> Location {
        Location::Standard
    }

    fn is_auto_repeating(&self) -> bool {
        false
    }

    fn is_composing(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn create_keyboard_event_data(keypad: KeyPad) -> Option<KeyEventData> {
    match keypad {
        KeyPad::A => Some((Key::Unidentified, Code::KeyA)),
        KeyPad::B => Some((Key::Unidentified, Code::KeyB)),
        KeyPad::Y => Some((Key::Unidentified, Code::KeyY)),
        KeyPad::X => Some((Key::Unidentified, Code::KeyX)),
        KeyPad::L => Some((Key::Unidentified, Code::KeyL)),
        KeyPad::R => Some((Key::Unidentified, Code::KeyR)),
        KeyPad::ZL => Some((Key::Unidentified, Code::ControlLeft)),
        KeyPad::ZR => Some((Key::Unidentified, Code::ControlRight)),
        KeyPad::DPAD_LEFT => Some((Key::Unidentified, Code::ArrowLeft)),
        KeyPad::DPAD_RIGHT => Some((Key::Unidentified, Code::ArrowRight)),
        KeyPad::DPAD_UP => Some((Key::Unidentified, Code::ArrowUp)),
        KeyPad::DPAD_DOWN => Some((Key::Unidentified, Code::ArrowDown)),
        KeyPad::START => Some((Key::Unidentified, Code::Enter)),
        KeyPad::SELECT => Some((Key::Unidentified, Code::ShiftLeft)),
        _ => None,
    }
    .map(|(key, code)| KeyEventData(key, code))
}

pub struct EventTrigger {
    pub(crate) mousedown_node_id: Option<ElementId>,
    pub(crate) first_touch: (u16, u16),
    pub(crate) touch: (u16, u16),
    pub(crate) touch_at: Instant,
    pub(crate) keypad: KeyPad,
    pub(crate) last_keypad_at: Instant,
    pub(crate) last_repeat_at: Instant,
}

fn depth_first(rdom: &RealDom, mut f: impl FnMut(NodeRef<'_>) -> bool) {
    let mut list = vec![(rdom.root_id(), false)];
    while let Some((node_id, is_scan_children)) = list.pop() {
        if is_scan_children {
            if f(rdom.get(node_id).unwrap()) {
                break;
            }
        } else {
            list.push((node_id, true));
            let node = rdom.get(node_id).unwrap();
            let rdom = node.real_dom();
            for child_id in rdom.tree_ref().children_ids_advanced(node_id, true) {
                list.push((child_id, false));
            }
        }
    }
}

fn get_parent_location(
    x: u16,
    y: u16,
    node: &NodeRef<'_>,
    rdom: &RealDom,
    taffy: Arc<Mutex<TaffyTree<()>>>,
) -> (u16, u16) {
    let mut x = x;
    let mut y = y;
    let res = {
        node.parent().is_some_and(|parent| {
            let r = parent.get::<TaffyLayout>().unwrap();
            let lock = taffy.lock().expect("get taffy lock in mouse event");
            let layout = lock.layout(r.node.unwrap()).unwrap();
            x += layout.location.x as u16;
            y += layout.location.y as u16;
            true
        })
    };
    if res {
        get_parent_location(x, y, &node.parent().unwrap(), rdom, taffy)
    } else {
        (x, y)
    }
}

impl EventTrigger {
    pub fn new() -> Self {
        Self {
            mousedown_node_id: None,
            first_touch: (0, 0),
            touch: (0, 0),
            touch_at: Instant::now(),
            keypad: KeyPad::empty(),
            last_keypad_at: Instant::now(),
            last_repeat_at: Instant::now(),
        }
    }

    fn handle_keyboard_event(&mut self, keypad: KeyPad, rdom: &RealDom, vdom: &mut VirtualDom) {
        // event
        if let Some(data) = create_keyboard_event_data(keypad) {
            rdom.get_listening_sorted("keypress")
                .into_iter()
                .map(|node| node.mounted_id())
                .filter(|id| id.is_some())
                .for_each(|id| {
                    vdom.handle_event(
                        "keypress",
                        Rc::new(PlatformEventData::new(Box::new(data.clone()))),
                        id.unwrap(),
                        false,
                    )
                });
        }
    }

    fn handle_mouse_event(
        &mut self,
        event_type: &str,
        click: (u16, u16),
        rdom: &RealDom,
        vdom: &mut VirtualDom,
        taffy: Arc<Mutex<TaffyTree<()>>>,
    ) {
        // event
        let data = MouseEventData(click.0, click.1);
        if event_type == "mouseup" {
            if let Some(node) = self.mousedown_node_id {
                self.mousedown_node_id = None;
                vdom.handle_event(
                    "mouseup",
                    Rc::new(PlatformEventData::new(Box::new(data))),
                    node,
                    false,
                );
            }
            return;
        }

        // find all nodes that are listening for the event and in the click area
        let list = rdom
            .get_listening_sorted(event_type)
            .into_iter()
            .filter(|node| node.mounted_id().is_some())
            .map(|node| {
                let (px, py) = get_parent_location(0, 0, &node, rdom, taffy.clone());
                let r = node.get::<TaffyLayout>().unwrap();
                let lock = taffy.lock().expect("get taffy lock in mouse event");
                let layout = lock.layout(r.node.unwrap()).unwrap();
                let x = layout.location.x as u16 + px;
                let y = layout.location.y as u16 + py;
                let tx = x + layout.size.width as u16;
                let ty = y + layout.size.height as u16;
                (node.mounted_id().unwrap(), x, y, tx, ty)
            })
            .filter(|&(_, x, y, tx, ty)| {
                !(click.0 < x || click.0 > tx || click.1 < y || click.1 > ty)
            })
            .collect::<Vec<_>>();

        depth_first(rdom, |node| {
            list.iter()
                .find(|&&(id, ..)| {
                    node.mounted_id()
                        .map(|node_id| {
                            if node_id == id {
                                if event_type == "mousedown" {
                                    self.mousedown_node_id = Some(node_id);
                                }
                                vdom.handle_event(
                                    event_type,
                                    Rc::new(PlatformEventData::new(Box::new(data.clone()))),
                                    node_id,
                                    false,
                                );
                                return true;
                            }
                            false
                        })
                        .unwrap_or(false)
                })
                .is_some()
        });
    }

    pub async fn scan_controller_input(
        &mut self,
        resource: &Rc<Resource>,
        previous_3d: f32,
    ) -> (
        KeyPad,
        f32,
        Option<(u16, u16)>,
        Option<(u16, u16)>,
        Option<(u16, u16)>,
    ) {
        resource.hid.borrow_mut().scan_input();

        // touch
        let mut click = None;
        let mut mousedown = None;
        let mut mouseup = None;
        let current_touch = resource.hid.borrow().touch_position();
        if current_touch.0 == 0 && current_touch.1 == 0 {
            if self.touch.0 != 0 || self.touch.1 != 0 {
                mouseup = Some(self.touch);
                if self.touch_at.elapsed().as_millis() < 1000
                    && (self.first_touch.0 == 0 && self.first_touch.1 == 0
                        || ((self.touch.0 as i32 - self.first_touch.0 as i32).pow(2)
                            + (self.touch.1 as i32 - self.first_touch.1 as i32).pow(2))
                            < 400)
                {
                    click = Some(self.touch);
                }
                self.touch = (0, 0);
                self.first_touch = (0, 0);
            }
        } else {
            if self.touch.0 == 0 && self.touch.1 == 0 {
                self.touch_at = Instant::now();
                self.first_touch = current_touch;
                mousedown = Some(current_touch);
            }
            self.touch = current_touch;
        }

        // keypad
        let mut keypad = resource.hid.borrow().keys_held();
        if keypad != self.keypad {
            self.keypad = keypad;
            self.last_keypad_at = Instant::now();
        } else if self.last_keypad_at.elapsed().as_millis() > 300
            && self.last_repeat_at.elapsed().as_millis() > 60
        {
            self.last_repeat_at = Instant::now();
        } else {
            keypad = KeyPad::empty();
        }
        // 3d slider
        let current_3d = current_3d_slider_state();

        if keypad.is_empty() || current_3d == previous_3d {
            sleep_micros(0).await
        }
        (keypad, current_3d, click, mousedown, mouseup)
    }

    pub async fn poll_event_and_wait_for_work(
        &mut self,
        resource: &Rc<Resource>,
        rdom: &RealDom,
        vdom: &mut VirtualDom,
        current_3d: &f32,
        current_new_3d: &mut f32,
        images: &mut ImageDataSet,
        app_exit: &Rc<AppExit>,
        taffy: Arc<Mutex<TaffyTree<()>>>,
    ) {
        while resource.main_loop() {
            tokio::select! {
                // wait for input
                (keypad, new_3d, click, mousedown, mouseup) = self.scan_controller_input(resource, *current_3d) => {
                    // keyboard
                    self.handle_keyboard_event(keypad, rdom, vdom);

                    // click
                    click.map(|point| {
                        self.handle_mouse_event("click", point, rdom, vdom, taffy.clone());
                    });

                    // mouse down
                    mousedown.map(|point| {
                        self.handle_mouse_event("mousedown", point, rdom, vdom, taffy.clone());
                    });

                    mouseup.map(|point| {
                        self.handle_mouse_event("mouseup", point, rdom, vdom, taffy.clone());
                    });

                    if new_3d != *current_3d {
                        *current_new_3d = new_3d;
                        break;
                    }

                    if images.is_update() {
                        break;
                    }

                    if app_exit.is_exit() {
                        break;
                    }
                }
                // wait for work
                _ = vdom.wait_for_work() => {
                    break;
                }
            }
        }
    }
}

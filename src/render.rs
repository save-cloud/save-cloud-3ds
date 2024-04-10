use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use ctru::os::current_3d_slider_state;
use dioxus::prelude::*;
use dioxus_native_core::{
    node::{OwnedAttributeDiscription, OwnedAttributeValue},
    prelude::*,
    tree::TreeRef,
};
use taffy::{prelude::*, Point};

use crate::{
    app::AppExit,
    c2d::{
        c2d_draw_image, c2d_draw_rect, c2d_draw_text, c2d_draw_text_wrap, C2dImageTrait, C2dText,
    },
    constant::{SCREEN_HEIGHT, SCREEN_TOP_WIDTH},
    platform::pl_open_the_title,
    resource::Resource,
    utils::{sleep_micros, sleep_micros_for_ever},
};

use self::{
    image_data_set::ImageDataSet,
    rdom::{rdom_style::RdomStyle, taffy_layout::TaffyLayout},
    revent::{EventTrigger, SerializedHtmlEventConverter},
};

pub mod image_data_set;
mod rdom;
pub mod revent;

fn render(
    node: NodeRef,
    taffy: Arc<Mutex<TaffyTree<()>>>,
    parent_location: Point<f32>,
    resource: &Rc<Resource>,
    current_3d: f32,
    images: &mut ImageDataSet,
) {
    let (origin_x, y, width, height) = {
        let lock = taffy.lock().expect("get taffy lock in render");
        let node = node.get::<TaffyLayout>().unwrap();
        let layout = lock.layout(node.node.unwrap()).unwrap();
        (
            layout.location.x + parent_location.x,
            layout.location.y + parent_location.y,
            layout.size.width,
            layout.size.height,
        )
    };

    // get style
    let rdom_style = *node.get::<RdomStyle>().unwrap();
    let is_top_screen = rdom_style.is_top();
    let RdomStyle {
        color,
        background_color,
        reset_color,
        scale,
        deep_3d,
        z_index,
        max_width,
        ..
    } = rdom_style;
    let render_3d = is_top_screen && current_3d != 0.0;
    let deep_3d = deep_3d * current_3d;
    let x = if !render_3d {
        origin_x
    } else {
        origin_x - deep_3d
    };

    // clear the screen
    if let Some(color) = reset_color {
        if is_top_screen {
            resource.c2d.clear_top_scene(color);
            resource.c2d.start_top_scene();
            if render_3d {
                resource.c2d.clear_top_scene_right(color);
                resource.c2d.start_top_scene_right();
            }
        } else {
            resource.c2d.clear_bottom_scene(color);
            resource.c2d.start_bottom_scene();
        }
    }

    // draw element background_color
    if let Some(color) = background_color {
        if is_top_screen {
            resource.c2d.start_top_scene();
        } else {
            resource.c2d.start_bottom_scene();
        }
        c2d_draw_rect(x, y, z_index, width, height, color);

        if render_3d {
            resource.c2d.start_top_scene_right();
            c2d_draw_rect(origin_x + deep_3d, y, z_index, width, height, color);
        }
    }

    match &*node.node_type() {
        NodeType::Text(text) => {
            if is_top_screen {
                resource.c2d.start_top_scene();
            } else {
                resource.c2d.start_bottom_scene();
            }
            let c2d_text = C2dText::new(&text.text);
            if let Some(max_width) = max_width {
                c2d_draw_text_wrap(&c2d_text, x, y, z_index, scale, color, max_width);
            } else {
                c2d_draw_text(&c2d_text, x, y, z_index, scale, color);
            }

            if render_3d {
                resource.c2d.start_top_scene_right();
                if let Some(max_width) = max_width {
                    c2d_draw_text_wrap(&c2d_text, x, y, z_index, scale, color, max_width);
                } else {
                    c2d_draw_text(&c2d_text, origin_x + deep_3d, y, z_index, scale, color);
                }
            }
        }
        NodeType::Element(ElementNode {
            tag, attributes, ..
        }) => {
            match tag.as_str() {
                "div" => {
                    // border ?
                }
                "img" => {
                    let image: Option<Box<Rc<dyn C2dImageTrait>>> = match match attributes
                        .get(&OwnedAttributeDiscription::from("src".to_string()))
                    {
                        Some(OwnedAttributeValue::Int(idx)) => resource
                            .c2d
                            .get_image_from_sheet(*idx as usize)
                            .map(|image| Box::new(Rc::new(image) as Rc<dyn C2dImageTrait>)),
                        Some(OwnedAttributeValue::Text(id)) => {
                            match attributes
                                .get(&OwnedAttributeDiscription::from("media".to_string()))
                            {
                                Some(OwnedAttributeValue::Text(media)) => {
                                    if media == "qrcode" {
                                        images.get_qrcode(id)
                                    } else {
                                        images.get_image(id, media)
                                    }
                                }
                                _ => None,
                            }
                        }
                        _ => None,
                    } {
                        Some(image) => Some(image),
                        None => resource
                            .c2d
                            .get_image_from_sheet(4)
                            .map(|image| Box::new(Rc::new(image) as Rc<dyn C2dImageTrait>)),
                    };

                    if let Some(image) = image {
                        if is_top_screen {
                            resource.c2d.start_top_scene();
                        } else {
                            resource.c2d.start_bottom_scene();
                        }
                        c2d_draw_image(image.get_image(), x, y, z_index, scale, scale);

                        if render_3d {
                            resource.c2d.start_top_scene_right();
                            c2d_draw_image(
                                image.get_image(),
                                origin_x + deep_3d,
                                y,
                                z_index,
                                scale,
                                scale,
                            );
                        }
                    }
                }
                _ => {}
            }
            let rdom = node.real_dom();
            for child_id in rdom.tree_ref().children_ids_advanced(node.id(), true) {
                let child = rdom.get(child_id).unwrap();
                render(
                    child,
                    Arc::clone(&taffy),
                    Point { x: origin_x, y },
                    resource,
                    current_3d,
                    images,
                );
            }
        }
        _ => {}
    };
}

fn compute_layout(taffy: &Arc<Mutex<TaffyTree<()>>>, rdom: &RealDom) {
    let root_node = rdom
        .get(rdom.root_id())
        .unwrap()
        .get::<TaffyLayout>()
        .unwrap()
        .node
        .unwrap();

    // the root node fills the entire area
    let mut taffy = taffy.lock().expect("taffy lock");
    let mut style = taffy.style(root_node).unwrap().clone();

    let width = SCREEN_TOP_WIDTH as f32;
    let height = SCREEN_HEIGHT as f32;
    let new_size = Size {
        width: length(width),
        height: length(height),
    };
    if style.size != new_size {
        style.size = new_size;
        taffy.set_style(root_node, style).unwrap();
    }

    taffy
        .compute_layout(
            root_node,
            Size {
                width: AvailableSpace::Definite(width),
                height: AvailableSpace::Definite(height),
            },
        )
        .unwrap();
}

pub fn launch(
    app_enter: fn() -> Element,
    resource: Rc<Resource>,
) -> Result<(), Box<dyn std::error::Error>> {
    // we need to run the vdom in a async runtime
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_time()
        .build()?
        .block_on(async move {
            // set event converter
            dioxus_html::set_event_converter(Box::new(SerializedHtmlEventConverter));
            // get the current 3d slider state
            let mut current_3d = 2.0;
            // app data
            let app_exit = Rc::new(AppExit::new());
            // taffy tree
            let taffy: Arc<Mutex<TaffyTree<()>>> = Arc::new(Mutex::new(TaffyTree::new()));
            // create the vdom, the real_dom, and the binding layer between them
            // provide the app data to the vdom
            let mut vdom = VirtualDom::new(app_enter)
                .with_root_context(app_exit.clone())
                .with_root_context(Rc::clone(&resource));
            // rdom
            let mut rdom =
                RealDom::new([TaffyLayout::to_type_erased(), RdomStyle::to_type_erased()]);
            // create the dioxus state
            let mut dioxus_state = DioxusState::create(&mut rdom);
            // rebuild
            vdom.rebuild(&mut dioxus_state.create_mutation_writer(&mut rdom));
            // create the context
            let mut ctx = SendAnyMap::new();
            // insert taffy
            ctx.insert(Arc::clone(&taffy));
            // update the State for nodes in the real_dom tree
            let to_rerender = rdom.update_state(ctx);
            let mut is_layout_dirty = !to_rerender.0.is_empty() || !to_rerender.1.is_empty();
            let mut is_need_rerender = is_layout_dirty;
            let mut event_trigger = EventTrigger::new();
            let mut image_data_set = ImageDataSet::new();
            let mut current_new_3d = current_3d_slider_state();
            while !app_exit.is_exit() && resource.main_loop() {
                // update the taffy layout
                // let now = std::time::Instant::now();
                if is_layout_dirty {
                    is_layout_dirty = false;
                    compute_layout(&taffy, &rdom);
                }

                if is_need_rerender {
                    is_need_rerender = false;
                    // start render...
                    resource.c2d.start_drawing();
                    // if the 3d slider state changed, we need to update the 3d state
                    if current_3d != current_new_3d {
                        current_3d = current_new_3d;
                        if current_3d != 0.0 {
                            resource.c2d.enable_3d();
                        } else {
                            resource.c2d.disable_3d();
                        }
                    }
                    // render the real_dom tree
                    render(
                        rdom.get(rdom.root_id()).unwrap(),
                        Arc::clone(&taffy),
                        Point { x: 0.0, y: 0.0 },
                        &resource,
                        current_3d,
                        &mut image_data_set,
                    );
                    // end render...
                    resource.c2d.end_drawing();
                    // Loading missing images
                    image_data_set.loading_missing_image();
                    // release qrcode
                    image_data_set.release_qrcode();
                }

                // println!("render time: {:?}", now.elapsed());

                while resource.main_loop() {
                    tokio::select! {
                        _ = async {
                            if is_need_rerender {
                                // combine multiple rerenders into one
                                sleep_micros(100).await
                            } else {
                                sleep_micros_for_ever(1000000).await
                            }
                        } => {
                            break;
                        }
                        _ = event_trigger.poll_event_and_wait_for_work(
                            &resource,
                            &rdom,
                            &mut vdom,
                            &current_3d,
                            &mut current_new_3d,
                            &mut image_data_set,
                            &app_exit,
                            taffy.clone(),
                        ) => {
                            is_need_rerender = true;
                        }
                    }

                    // get the mutations from the vdom and apply them to the real_dom
                    vdom.render_immediate(&mut dioxus_state.create_mutation_writer(&mut rdom));
                    let mut ctx = SendAnyMap::new();
                    ctx.insert(Arc::clone(&taffy));
                    let to_rerender = rdom.update_state(ctx);
                    if !to_rerender.0.is_empty() || !to_rerender.1.is_empty() {
                        is_layout_dirty = true;
                    }
                }
            }
            if let (title_id, Some((media, params))) = app_exit.inner_value() {
                let res = pl_open_the_title(title_id, media as u8, &params);
                if res {
                    loop {
                        sleep(Duration::from_millis(10000));
                    }
                }
            }
        });

    Ok(())
}

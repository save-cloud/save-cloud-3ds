use dioxus_native_core::{exports::shipyard::Component, node::OwnedAttributeValue, prelude::*};
use dioxus_native_core_macro::partial_derive_state;

use crate::{c2d::rgba, constant::MAX_DEEP_3D, utils::color_name_rgba};

#[derive(Clone, Copy, PartialEq, Component)]
pub struct RdomStyle {
    pub color: u32,
    pub background_color: Option<u32>,
    pub reset_color: Option<u32>,
    pub scale: f32,
    pub deep_3d: f32,
    pub z_index: f32,
    pub max_width: Option<f32>,
    screen: u8,
}

impl RdomStyle {
    pub fn is_top(&self) -> bool {
        self.screen == 0
    }
}

impl Default for RdomStyle {
    fn default() -> Self {
        RdomStyle {
            color: rgba(0x00, 0x00, 0x00, 0xff),
            background_color: None,
            reset_color: None,
            scale: 1.0,
            screen: 0,
            deep_3d: 0.0,
            max_width: None,
            z_index: 0.0,
        }
    }
}

#[partial_derive_state]
impl State for RdomStyle {
    // TextColor depends on the TextColor part of the parent
    type ParentDependencies = (Self,);

    type ChildDependencies = ();

    type NodeDependencies = ();

    // TextColor only cares about the color attribute of the current node
    const NODE_MASK: NodeMaskBuilder<'static> =
        // Get access to the color attribute
        NodeMaskBuilder::new().with_attrs(AttributeMaskBuilder::Some(&[
                "color",
                "background-color",
                "bg_reset",
                "screen",
                "scale",
                "deep_3d",
                "z-index",
                "max-width",
            ]));

    fn update<'a>(
        &mut self,
        node_view: NodeView<()>,
        _node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _context: &SendAnyMap,
    ) -> bool {
        // TextColor only depends on the color tag, so getting the first tag is equivalent to looking through all tags
        let mut new = RdomStyle::default();
        match &parent {
            Some((parent,)) => {
                new.color = parent.color;
                new.screen = parent.screen;
                new.scale = parent.scale;
                new.deep_3d = parent.deep_3d;
                new.z_index = parent.z_index;
                new.max_width = parent.max_width;
            }
            None => {}
        }
        if let Some(attributes) = node_view.attributes() {
            for attr in attributes {
                match attr.attribute.name.as_str() {
                    "color" => {
                        new.color = match attr.value {
                            OwnedAttributeValue::Text(color) => color_name_rgba(color),
                            OwnedAttributeValue::Int(color) if *color > 0 => *color as u32,
                            _ => match &parent {
                                Some((parent,)) => parent.color,
                                None => Self::default().color,
                            },
                        }
                    }
                    "screen" => {
                        new.screen = match attr.value.as_text() {
                            Some("top") => 0,
                            Some("bottom") => 1,
                            _ => match &parent {
                                Some((parent,)) => parent.screen,
                                None => 0,
                            },
                        }
                    }
                    "scale" => {
                        new.scale = match attr.value {
                            OwnedAttributeValue::Float(scale) => *scale as f32,
                            OwnedAttributeValue::Int(scale) => *scale as f32,
                            _ => match &parent {
                                Some((parent,)) => parent.scale,
                                None => 1.0,
                            },
                        }
                    }
                    "deep_3d" => {
                        new.deep_3d = match attr.value {
                            OwnedAttributeValue::Float(deep_3d) => *deep_3d as f32,
                            OwnedAttributeValue::Int(deep_3d) => *deep_3d as f32,
                            _ => match &parent {
                                Some((parent,)) => parent.deep_3d,
                                None => 0.0,
                            },
                        };

                        if new.deep_3d > MAX_DEEP_3D {
                            new.deep_3d = MAX_DEEP_3D;
                        } else if new.deep_3d < -MAX_DEEP_3D {
                            new.deep_3d = -MAX_DEEP_3D;
                        }
                    }
                    "z-index" => {
                        new.z_index = match attr.value {
                            OwnedAttributeValue::Float(z) => *z as f32,
                            OwnedAttributeValue::Int(z) => *z as f32,
                            _ => match &parent {
                                Some((parent,)) => parent.z_index,
                                None => 0.0,
                            },
                        }
                    }
                    "max-width" => {
                        new.max_width = match attr.value {
                            OwnedAttributeValue::Float(max) => Some(*max as f32),
                            OwnedAttributeValue::Int(max) => Some(*max as f32),
                            _ => match &parent {
                                Some((parent,)) => parent.max_width,
                                None => Self::default().max_width,
                            },
                        }
                    }
                    "background-color" => {
                        new.background_color = match attr.value {
                            OwnedAttributeValue::Text(color) => Some(color_name_rgba(color)),
                            OwnedAttributeValue::Int(color) if *color > 0 => Some(*color as u32),
                            _ => None,
                        }
                    }
                    "bg_reset" => {
                        new.reset_color = match attr.value {
                            OwnedAttributeValue::Text(color) => Some(color_name_rgba(color)),
                            OwnedAttributeValue::Int(color) if *color > 0 => Some(*color as u32),
                            _ => None,
                        }
                    }
                    _ => {}
                }
            }
        }
        // check if the member has changed
        let changed = new != *self;
        *self = new;
        changed
    }
}

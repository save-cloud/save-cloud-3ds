use std::sync::{Arc, Mutex};

use dioxus_native_core::{exports::shipyard::Component, prelude::*};
use dioxus_native_core_macro::partial_derive_state;
use taffy::{
    prelude::{NodeId as TaffyNodeId, *},
    Overflow, Point,
};

use crate::c2d::C2dText;

use super::rdom_style::RdomStyle;

// these are the attributes in layout_attiributes in native-core
const SORTED_LAYOUT_ATTRS: &[&str] = &[
    "align-content",
    "align-items",
    "align-self",
    "border-bottom-width",
    "border-left-width",
    "border-right-width",
    "border-top-width",
    "bottom",
    "display",
    "flex",
    "flex-basis",
    "flex-direction",
    "flex-grow",
    "flex-shrink",
    "flex-wrap",
    "height",
    "justify-content",
    "left",
    "margin",
    "margin-bottom",
    "margin-left",
    "margin-right",
    "margin-top",
    "padding",
    "padding-bottom",
    "padding-left",
    "padding-right",
    "padding-top",
    "position",
    "right",
    "top",
    "width",
    "max-width",
    "overflow",
    "gap",
];

#[derive(Clone, PartialEq, Default, Component)]
pub struct TaffyLayout {
    pub style: Style,
    pub node: Option<TaffyNodeId>,
}

#[partial_derive_state]
impl State for TaffyLayout {
    type ParentDependencies = (RdomStyle,);

    type ChildDependencies = (Self,);

    type NodeDependencies = ();

    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new()
        .with_attrs(AttributeMaskBuilder::Some(SORTED_LAYOUT_ATTRS))
        .with_text();

    fn update<'a>(
        &mut self,
        node_view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        ctx: &SendAnyMap,
    ) -> bool {
        let mut changed = false;
        let mut style = Style {
            overflow: Point {
                x: Overflow::Hidden,
                y: Overflow::Hidden,
            },
            display: Display::Block,
            ..Style::default()
        };

        // gather up all the styles from the attribute list
        if let Some(attributes) = node_view.attributes() {
            for OwnedAttributeView {
                attribute, value, ..
            } in attributes
            {
                if value.as_custom().is_none() {
                    let value_text = value.as_text().unwrap_or("");
                    let value_f = value.as_float().unwrap_or(0.0) as f32;
                    match attribute.name.as_str() {
                        "align-content" => match value_text {
                            "center" => style.align_content = Some(AlignContent::Center),
                            "flex-end" => style.align_content = Some(AlignContent::FlexEnd),
                            "flex-start" => style.align_content = Some(AlignContent::FlexStart),
                            "space-around" => style.align_content = Some(AlignContent::SpaceAround),
                            "space-between" => {
                                style.align_content = Some(AlignContent::SpaceBetween)
                            }
                            "stretch" => style.align_content = Some(AlignContent::Stretch),
                            _ => {}
                        },
                        "align-items" => match value_text {
                            "baseline" => style.align_items = Some(AlignItems::Baseline),
                            "center" => style.align_items = Some(AlignItems::Center),
                            "flex-end" => style.align_items = Some(AlignItems::FlexEnd),
                            "flex-start" => style.align_items = Some(AlignItems::FlexStart),
                            "stretch" => style.align_items = Some(AlignItems::Stretch),
                            _ => {}
                        },
                        "align-self" => match value_text {
                            "baseline" => style.align_self = Some(AlignSelf::Baseline),
                            "center" => style.align_self = Some(AlignSelf::Center),
                            "flex-end" => style.align_self = Some(AlignSelf::FlexEnd),
                            "flex-start" => style.align_self = Some(AlignSelf::FlexStart),
                            "stretch" => style.align_self = Some(AlignSelf::Stretch),
                            _ => {}
                        },
                        "border-bottom-width" => {
                            style.border.bottom = length(value_f);
                        }
                        "border-left-width" => {
                            style.border.left = length(value_f);
                        }
                        "border-right-width" => {
                            style.border.right = length(value_f);
                        }
                        "border-top-width" => {
                            style.border.top = length(value_f);
                        }
                        "bottom" => {
                            style.inset.bottom = length(value_f);
                        }
                        "display" => {
                            style.display = match value_text {
                                "block" => Display::Block,
                                "flex" => Display::Flex,
                                _ => Display::Block,
                            };
                        }
                        "flex" => {
                            style.flex_grow = value_f;
                            style.flex_shrink = 1.0;
                            style.flex_basis = length(value_f);
                        }
                        "flex-basis" => {
                            style.flex_basis = length(value_f);
                        }
                        "flex-direction" => {
                            style.flex_direction = match value_text {
                                "column" => FlexDirection::Column,
                                "column-reverse" => FlexDirection::ColumnReverse,
                                "row" => FlexDirection::Row,
                                "row-reverse" => FlexDirection::RowReverse,
                                _ => FlexDirection::Row,
                            };
                        }
                        "flex-grow" => {
                            style.flex_grow = value_f;
                        }
                        "flex-shrink" => {
                            style.flex_shrink = value.as_float().unwrap_or(1.0) as f32;
                        }
                        "flex-wrap" => {
                            style.flex_wrap = match value_text {
                                "nowrap" => FlexWrap::NoWrap,
                                "wrap" => FlexWrap::Wrap,
                                "wrap-reverse" => FlexWrap::WrapReverse,
                                _ => FlexWrap::NoWrap,
                            };
                        }
                        "gap" => {
                            style.gap = length(value_f);
                        }
                        "height" => {
                            style.size.height = length(value_f);
                        }
                        "justify-content" => {
                            style.justify_content = match value_text {
                                "center" => Some(JustifyContent::Center),
                                "flex-end" => Some(JustifyContent::FlexEnd),
                                "flex-start" => Some(JustifyContent::FlexStart),
                                "space-around" => Some(JustifyContent::SpaceAround),
                                "space-between" => Some(JustifyContent::SpaceBetween),
                                _ => None,
                            };
                        }
                        "left" => {
                            style.inset.left = length(value_f);
                        }
                        "margin" => {
                            style.margin = Rect {
                                left: length(value_f),
                                right: length(value_f),
                                top: length(value_f),
                                bottom: length(value_f),
                            };
                        }
                        "margin-bottom" => {
                            style.margin.bottom = length(value_f);
                        }
                        "margin-left" => {
                            style.margin.left = length(value_f);
                        }
                        "margin-right" => {
                            style.margin.right = length(value_f);
                        }
                        "margin-top" => {
                            style.margin.top = length(value_f);
                        }
                        "padding" => {
                            style.padding = Rect {
                                left: length(value_f),
                                right: length(value_f),
                                top: length(value_f),
                                bottom: length(value_f),
                            };
                        }
                        "padding-bottom" => {
                            style.padding.bottom = length(value_f);
                        }
                        "padding-left" => {
                            style.padding.left = length(value_f);
                        }
                        "padding-right" => {
                            style.padding.right = length(value_f);
                        }
                        "padding-top" => {
                            style.padding.top = length(value_f);
                        }
                        "position" => {
                            style.position = match value_text {
                                "absolute" => Position::Absolute,
                                "relative" => Position::Relative,
                                _ => Position::Relative,
                            };
                        }
                        "right" => {
                            style.inset.right = length(value_f);
                        }
                        "top" => {
                            style.inset.top = length(value_f);
                        }
                        "width" => {
                            style.size.width = length(value_f);
                        }
                        _ => {}
                    }
                }
            }
        }

        {
            let taffy: &Arc<Mutex<TaffyTree<()>>> = ctx.get().unwrap();
            let mut taffy = taffy.lock().expect("get taffy lock in rdom style");
            if let Some(text) = node_view.text() {
                let (scale, max_width) = match parent {
                    Some((parent,)) => (parent.scale, parent.max_width),
                    _ => (1.0, None),
                };
                let (mut width, height) = C2dText::new(text).dimension(scale, scale);
                if let Some(max_width) = max_width {
                    width = width.min(max_width);
                }
                style = Style {
                    size: Size {
                        width: Dimension::Length(width),
                        height: Dimension::Length(height),
                    },
                    ..Default::default()
                };
                if let Some(n) = self.node {
                    if self.style != style {
                        taffy.set_style(n, style.clone()).unwrap();
                    }
                } else {
                    self.node = Some(taffy.new_leaf(style.clone()).unwrap());
                    changed = true;
                }
            } else {
                // Set all direct nodes as our children
                let mut child_layout = vec![];
                for (l,) in children {
                    if let Some(node) = l.node {
                        child_layout.push(node);
                    }
                }

                if let Some(n) = self.node {
                    if self.style != style {
                        taffy.set_style(n, style.clone()).unwrap();
                    }
                    if taffy.children(n).unwrap() != child_layout {
                        taffy.set_children(n, &child_layout).unwrap();
                    }
                } else {
                    self.node = Some(
                        taffy
                            .new_with_children(style.clone(), &child_layout)
                            .unwrap(),
                    );
                    changed = true;
                }
            }
        }

        if self.style != style {
            self.style = style;
            changed = true;
        }
        changed
    }
}

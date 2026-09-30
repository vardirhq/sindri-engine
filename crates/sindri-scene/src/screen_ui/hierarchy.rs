//! Where a UI element ends up once its parents have had their say.
//!
//! The overlay's rule used to be one line: an element's anchor picks a point on
//! the viewport and its transform is an offset from that point. That is exactly
//! right for a HUD reading, and it quietly means a hierarchy is not one. A label
//! parented to a card was placed against the *screen*, so six cards' worth of
//! labels landed on top of each other however far apart the cards were; and a
//! layout's spacing reached the code that decides what was clicked but never the
//! code that decides what is drawn, so an element could be clickable somewhere
//! it was not.
//!
//! This resolves both, once, for everything: the frame, the pointer, and the
//! editor's handles all read the same answer. An element's placement is its own
//! offset composed with every ancestor's, plus whatever a parent's layout has to
//! say about where it sits among its siblings.
//!
//! ## What is inherited, and what is not
//!
//! Position and rotation compose; **size does not**. That is a consequence of
//! what a UI transform means here: `scale` is the element's size in overlay
//! units, not a multiplier on a coordinate space. A card two units wide holding
//! a label is not asking for the label to be two times anything — it is asking
//! for a label on a card. Inheriting size would make every child of a wide panel
//! wide, which is not what anyone drawing a panel means, and it is why this is
//! deliberately not a full `RectTransform`: that answers a different question
//! (how a child *stretches* with its parent) and answering it needs anchors with
//! two corners rather than one point.
//!
//! The anchor is taken from the outermost ancestor that declares one, because a
//! child re-anchoring against the screen is how a label leaves the card it is
//! written on when the window changes shape.

use std::collections::BTreeMap;

use glam::{Quat, Vec2};
use sindri_core::{ComponentRegistryError, ComponentSchemaRegistry, EntityId, Transform3D, World};

use super::layout_pass::{Laid, lay_out};
use super::{UiButtonComponent, UiTextSizes};
use crate::{UiAnchor, UiImageComponent, UiShapeComponent, UiTextComponent};

/// How deep a parent chain is followed.
///
/// A malformed world can hold a cycle — an entity that is its own ancestor —
/// and a walk up such a chain never ends. Bounded rather than detected, because
/// the bound is also the honest answer to a chain nobody could have authored on
/// purpose: no real UI is sixty-four elements deep.
const MAX_DEPTH: usize = 64;

/// Where one UI element sits, with its ancestors folded in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiPlaced {
    /// The offset from the anchor's point on the viewport, in overlay units.
    pub offset: Vec2,
    /// The element's own rotation with its ancestors' turned in.
    pub rotation: Quat,
    /// The anchor `offset` is measured from: the outermost one in the chain.
    pub anchor: UiAnchor,
    /// The size a parent's layout gave the element, where one grew, shrank,
    /// stretched or fitted it; `None` keeps the element's own size.
    pub size: Option<Vec2>,
}

impl UiPlaced {
    /// An element placed by nothing but its own anchor, at the anchor's point.
    #[must_use]
    pub fn at_anchor(anchor: UiAnchor) -> Self {
        Self {
            offset: Vec2::ZERO,
            rotation: Quat::IDENTITY,
            anchor,
            size: None,
        }
    }

    /// The element's size: what its layout made it, or `own` if nothing did.
    #[must_use]
    pub fn size_or(self, own: [f32; 2]) -> [f32; 2] {
        self.size.map_or(own, |size| size.to_array())
    }

    /// This placement with a child's own offset and turn applied inside it.
    ///
    /// The child's offset is turned by the parent's rotation before it is added,
    /// which is what makes a rotated panel carry its contents round with it
    /// rather than sliding them sideways.
    #[must_use]
    fn with_child(self, offset: Vec2, rotation: Quat) -> Self {
        let turned = self.rotation * offset.extend(0.0);
        Self {
            offset: self.offset + turned.truncate(),
            rotation: self.rotation * rotation,
            anchor: self.anchor,
            size: None,
        }
    }
}

/// Every UI element's placement, resolved once.
#[derive(Clone, Debug, Default)]
pub struct UiHierarchy {
    placed: BTreeMap<EntityId, UiPlaced>,
    /// Each scroll region's content height, as its children are laid out.
    content: BTreeMap<EntityId, f32>,
}

impl UiHierarchy {
    /// Resolves every UI element in the world.
    ///
    /// Every element, including inactive ones: what is drawn and what is
    /// clickable are decided elsewhere, and an editor showing a hidden screen's
    /// layout needs the same answer as the frame that would draw it.
    pub fn of(
        world: &World,
        components: &ComponentSchemaRegistry,
    ) -> Result<Self, ComponentRegistryError> {
        Self::measured(world, components, &UiTextSizes::new())
    }

    /// Resolves every UI element, with the words of text elements that fit
    /// their content measured by a host: see [`super::measure_ui_text`].
    pub fn measured(
        world: &World,
        components: &ComponentSchemaRegistry,
        text: &UiTextSizes,
    ) -> Result<Self, ComponentRegistryError> {
        let anchors = declared_anchors(world, components)?;
        let laid = lay_out(world, components, text)?;
        let mut placed = BTreeMap::new();
        for entity in anchors.keys().copied() {
            let mut resolved = resolve(world, &anchors, &laid, entity);
            resolved.size = laid.sizes.get(&entity).copied().map(Vec2::from_array);
            placed.insert(entity, resolved);
        }
        Ok(Self {
            placed,
            content: laid.content,
        })
    }

    /// How tall a scroll region's content is as laid out, measured down from
    /// its top edge; `None` for anything that is not a scroll region.
    #[must_use]
    pub fn scroll_content(&self, entity: EntityId) -> Option<f32> {
        self.content.get(&entity).copied()
    }

    /// Where this element sits, or `None` for an entity that draws no UI.
    #[must_use]
    pub fn placement(&self, entity: EntityId) -> Option<UiPlaced> {
        self.placed.get(&entity).copied()
    }

    /// Where this element sits, falling back to its own anchor alone.
    ///
    /// For a caller that has an anchor in hand and only wants the hierarchy's
    /// answer where there is one — a tool inspecting an entity mid-edit, before
    /// the hierarchy it belongs to has been resolved again.
    #[must_use]
    pub fn placement_or(&self, entity: EntityId, anchor: UiAnchor) -> UiPlaced {
        self.placement(entity)
            .unwrap_or_else(|| UiPlaced::at_anchor(anchor))
    }
}

/// One element's placement, by walking its chain to the outermost ancestor and
/// composing back down.
fn resolve(
    world: &World,
    anchors: &BTreeMap<EntityId, UiAnchor>,
    laid: &Laid,
    entity: EntityId,
) -> UiPlaced {
    // Up first, collecting the chain, because the anchor belongs to its far end
    // and the composition runs the other way.
    let mut chain = vec![entity];
    let mut walker = entity;
    while chain.len() < MAX_DEPTH {
        let Some(parent) = world.get(walker).and_then(|data| data.parent) else {
            break;
        };
        chain.push(parent);
        walker = parent;
    }
    // The outermost declared anchor wins; an element in a chain that declares
    // none anywhere falls back to its own, which is what a lone HUD reading has.
    let anchor = chain
        .iter()
        .rev()
        .find_map(|entity| anchors.get(entity).copied())
        .or_else(|| anchors.get(&entity).copied())
        .unwrap_or_default();

    let mut placed = UiPlaced::at_anchor(anchor);
    for link in chain.iter().rev().copied() {
        let transform = world
            .get(link)
            .and_then(|data| data.transform_3d)
            .unwrap_or_default();
        let layout = laid.offsets.get(&link).copied().unwrap_or(Vec2::ZERO);
        placed = placed.with_child(
            Vec2::from_array(transform.position_2d()) + layout,
            rotation_of(transform),
        );
    }
    let mut parent = world.get(entity).and_then(|d| d.parent);
    for _ in 0..MAX_DEPTH {
        let Some(e) = parent else {
            break;
        };
        let Some(data) = world.get(e) else {
            break;
        };
        // Everything inside a scroll region moves with it: up by as far as
        // the list has been scrolled, clamped to what it can scroll.
        if let Some(payload) = data
            .components
            .get(<super::UiScrollComponent as sindri_core::SceneComponent>::TYPE_NAME)
            && let Ok(scroll) = serde_json::from_value::<super::UiScrollComponent>(payload.clone())
        {
            let height = laid.sizes.get(&e).map_or_else(
                || data.transform_3d.unwrap_or_default().scale_2d()[1],
                |size| size[1],
            );
            let measured = laid.content.get(&e).copied().unwrap_or(0.0);
            placed.offset.y += scroll.coerce(scroll.offset, height, measured);
        }
        parent = data.parent;
    }
    placed
}

/// The transform's rotation, or none at all if it is not a rotation.
///
/// A quaternion of zeros deserializes happily and normalizes to a NaN, which
/// would put an element nowhere at all rather than somewhere wrong.
fn rotation_of(transform: Transform3D) -> Quat {
    let raw = Quat::from_array(transform.rotation);
    if raw.length_squared() > f32::EPSILON {
        raw.normalize()
    } else {
        Quat::IDENTITY
    }
}

/// The anchor every UI element declares.
///
/// This is also the set of entities that *are* UI elements, which is why the
/// hierarchy is keyed on it: an entity with a transform and no UI component is
/// a group, and a group is placed but never drawn.
///
/// A button counts even with no art of its own, because a hit area with no art
/// is a legitimate thing to author — and because leaving it out is not
/// "it gets no anchor", it is "it gets no *placement*", so a row of bare
/// buttons stops being laid out at all. The same set `ScreenUi::elements`
/// collects, for the same reason.
fn declared_anchors(
    world: &World,
    components: &ComponentSchemaRegistry,
) -> Result<BTreeMap<EntityId, UiAnchor>, ComponentRegistryError> {
    let mut anchors = BTreeMap::new();
    for (entity, image) in components.query::<UiImageComponent>(world)? {
        anchors.insert(entity, image.anchor);
    }
    for (entity, shape) in components.query::<UiShapeComponent>(world)? {
        anchors.entry(entity).or_insert(shape.anchor);
    }
    for (entity, text) in components.query::<UiTextComponent>(world)? {
        anchors.entry(entity).or_insert(text.anchor);
    }
    for (entity, _) in components.query::<UiButtonComponent>(world)? {
        // A button's own anchor comes from whatever it draws with; a bare one
        // is centred, like anything else that says nothing.
        anchors.entry(entity).or_insert(UiAnchor::Center);
    }
    for (entity, data) in world.entities() {
        if [
            "sindri.ui.slider",
            "sindri.ui.toggle",
            "sindri.ui.text_input",
            "sindri.ui.scroll",
        ]
        .iter()
        .any(|name| data.components.contains_key(*name))
        {
            anchors.entry(entity).or_insert(UiAnchor::Center);
        }
    }
    Ok(anchors)
}

#[cfg(test)]
mod tests;

//! The seam between a simulated world and a drawn frame.
//!
//! `sindri-render` and `sindri-grid` deliberately know nothing about worlds,
//! components, or scenes, and `sindri-core` knows nothing about drawing or
//! grid gameplay. This crate owns the built-in `sindri.*` component schemas and
//! adapts a world into the derived forms those neutral crates consume: ordered
//! render frames and validated navigation snapshots.

mod animation;
mod audio;
mod camera_control;
mod camera_math;
mod collision_outline;
mod components;
pub(crate) mod effects;
mod extract;
mod generation;
mod input_actions;
mod navigation;
mod occlusion;
mod physics;
mod physics_joints;
mod physics_material;
mod physics_sync;
mod placement;
pub(crate) mod screen_ui;
mod sequence;
mod textures;
mod tile_chunk;
mod tile_surface;
mod tilemap_collision;
mod tilesets;
mod voxel_render;

pub use animation::{AnimationClip, AnimationError, SpriteAnimationComponent, SpriteAnimations};
pub use audio::AudioSourceComponent;
pub use camera_control::{
    clear_camera_bounds, clear_camera_follow, raise_camera_trauma, set_camera_bounds,
    set_camera_dead_zone, set_camera_follow_offset, set_camera_follow_target, set_camera_max_speed,
    set_camera_orthographic_size, set_camera_shake, set_camera_smoothing,
};
pub use camera_math::camera_rotation_from_look_at;
pub use collision_outline::{CollisionShapes, collision_shapes};
pub use components::{
    BiomeDocument, CameraBehaviorComponent, CameraBounds, CameraComponent, CameraFit, CameraFollow,
    CameraShake, EnvironmentAmbientOcclusion, EnvironmentBloom, EnvironmentComponent,
    EnvironmentError, EnvironmentFog, EnvironmentPostProcess, EnvironmentShadows,
    EnvironmentToneMapping, GridNavigationComponent, GridOccupantComponent, GridPlacementComponent,
    GridWallDocument, LightComponent, LightError, LightKind, MeshComponent, MeshPrimitive,
    NaturalTerrainDocument, ShapeComponent, ShapeGeometry, SpriteColorTransform, SpriteComponent,
    Sun, TileCellDocument, TileDraw, TileGridComponent, TileGridError, TileProjection, TileSpace,
    TileVolumeComponent, TileVolumeError, TileVolumeIndex, TilemapComponent, TilemapError,
    UiAnchor, UiFill, UiFillEdge, UiImageComponent, UiShapeBlend, UiShapeComponent, UiShapeKind,
    UiShapeShadow, UiTextAutoSize, UiTextCase, UiTextComponent, UiTextLineAlign, UiTextOutline,
    UiTextShadow, UiTextWrap, VoxelBlock, VoxelEdit, VoxelGeneratorDocument, VoxelMapFlood,
    VoxelMaterialDocument, VoxelView, VoxelWorldComponent, add_camera_trauma, cell_to_local_in,
    default_sun_transform, environment_of, environments_in, light_direction, lights_in, sun_in,
    ui_text_template, update_camera_behaviors,
};
pub use effects::{EffectBurstComponent, Effects2d, Fleck};
pub use extract::{
    CameraView, ExtractProblem, OverlayPlacement, OverlayView, SceneExtractError, SceneExtractor,
    SceneRuntime, UiCanvas, ViewCamera, VoxelGround, VoxelWorldHit, WorldProjection,
    overlay_for_viewport, overlay_in_scene, pan_for_drag, voxel_space, world_camera_of,
};
pub use input_actions::InputActionsComponent;
pub use navigation::{GridNavigationError, GridPlacement, WorldGridNavigation};
pub use occlusion::{
    OcclusionError, OcclusionFinding, OcclusionProbe, OcclusionReport, sweep_occlusion,
};
pub use physics::{
    Collider2dComponent, LAYER_LIMIT, OneWay2dComponent, PhysicsWorld2dComponent,
    RigidBody2dComponent, RigidBodyKind, collision_layers, layer_bit,
};
pub use physics_joints::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SliderJoint2dComponent, SpringJoint2dComponent,
};
pub use physics_material::{
    PhysicsMaterial2dComponent, PhysicsMaterialError, PhysicsMaterialSources,
    physics_material_profile, referenced_physics_materials,
};
pub use physics_sync::{PhysicsSyncError, ScenePhysics2d};
pub use placement::{
    GridPlacementError, GridSurfaces, blocking_step_ahead, nearest_cell, resolve_grid_placements,
    standing_depth,
};
pub use screen_ui::{
    SafeArea, ScreenExtent, ScreenRect, ScreenUi, UiAlignSelf, UiBoxComponent, UiButtonComponent,
    UiCaret, UiDirection, UiDropdownComponent, UiGridComponent, UiHierarchy, UiInput, UiLayoutBox,
    UiLayoutChild, UiLayoutComponent, UiOptionComponent, UiPlaced, UiScrollComponent, UiSides,
    UiSliderComponent, UiSliderOrientation, UiTextInputComponent, UiTextSizes, UiToggleComponent,
    UiTrack, dropdown_options, measure_ui_text,
};
pub use sequence::{
    Axis, Cue, CueFired, CueSound, EASINGS, Key, Property, Sequence, SequenceComponent,
    SequenceError, SequenceStep, Sequences, Step, TRANSFORM_PROPERTIES, Track, easing, pose,
    referenced_sounds, resolve, top_level,
};
/// The shapes a collider is made of, which the editor draws and resizes.
pub use sindri_physics::{Collider2d, ColliderShape2d};
pub use textures::{
    FONT_NAMING_COMPONENTS, PROCEDURAL_TEXTURES, ProceduralTexture, SheetBindError,
    TEXTURE_NAMING_COMPONENTS, TextureBindings, referenced_fonts, referenced_sheets,
    referenced_textures, unresolved_sprites, unresolved_textures,
};
pub use tile_chunk::{TILE_CHUNK_SIZE, TileChunkCoord, TileChunkStore};
pub use tile_surface::{TileSurfaceError, TileSurfaces};
pub use tilemap_collision::{TilemapCollider2dComponent, TilemapCollisionError};
pub mod voxel;
pub use voxel::{VoxelError, VoxelFace, VoxelHit, cube_faces, face_quad, pick};
pub use voxel_render::{
    CompiledVoxelBatch, CompiledVoxelSection, VoxelRenderBridge, VoxelRenderError,
    VoxelRenderStats, VoxelTexture, VoxelTextureSource, compile_block_mesh,
};

pub use tilesets::{TileSetBindings, referenced_tile_sets, tile_set_sheets, tile_set_textures};

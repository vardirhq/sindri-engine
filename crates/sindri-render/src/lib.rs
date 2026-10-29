//! Target-independent Sindri rendering building blocks.

mod batch_stats;
mod bloom;
mod camera;
mod color;
mod cube;
mod depth;
mod encode;
mod fog;
mod frame;
mod glyph;
mod light2d;
mod lighting;
mod material;
mod mesh;
mod mesh_stats;
mod model;
mod offscreen;
mod retained_mesh;
mod shadow;
mod shape;
mod sprite;
mod sprite_batch;
mod sprite_shader;
mod target;
mod text;
mod texture;
mod textured_cube;
mod textured_mesh_cache;
mod transparency;
mod triangle;
mod uv_rect;

pub use batch_stats::BatchDrawStats;
pub use bloom::{Bloom, BloomSettings, PostProcessSettings, ToneMapping};
pub use camera::{
    OrthographicCamera, PerspectiveCamera, look_at, orthographic_projection, perspective_projection,
};
pub use color::{COLOR_TARGET_FORMAT, ColorSpaceError, require_srgb_target};
pub use cube::CubeRenderer;
pub use depth::DepthTarget;
pub use encode::{
    FrameEncodeError, FrameRenderers, FrameTarget, Lighting, encode_lit_frame,
    encode_prepared_frame,
};
pub use fog::FogSettings;
pub use frame::{
    ClearOperations, ExtractedFrame, FrameCamera, FrameCommand, FramePass, FramePlanError,
    PreparedFrame, RenderLayer, RenderStage, SpriteEnvironment, Viewport,
};
pub use glyph::{
    EDGE, FIELD_PER_RASTER_PIXEL, GlyphAtlas, GlyphDrawError, GlyphInstance, GlyphRenderer,
    GlyphSlot, RASTER_EM,
};
pub use light2d::{
    Light2d, Light2dShadow, LightCone2d, Lighting2d, MAX_LIGHTS_2D, SHADOW_INTERVAL_FLOATS_2D,
    SHADOW_INTERVALS_2D, SHADOW_MAP_LEN_2D, SHADOW_SAMPLES_2D, Sight2d, sees, shadow_map_2d,
    shadow_maps_2d,
};
pub use lighting::{
    LocalLight, MAX_LOCAL_LIGHTS, PointLight, PointLightError, SpotCone, WorldLighting,
};
pub use material::{MaterialError, MaterialFactors, MaterialSidedness};
pub use mesh::{ColoredVertex, MeshBuffers, TexturedVertex};
pub use mesh_stats::MeshDrawStats;
pub use model::{
    ModelCacheStats, ModelData, ModelGeometry, ModelHierarchyNode, ModelImage, ModelIndexData,
    ModelNodeInstance, ModelRenderError, ModelSurface, ModelVertex, RenderModel,
};
pub use offscreen::{OffscreenError, OffscreenReadback, OffscreenTarget};
pub use retained_mesh::{MeshGeometryError, RenderMesh};
pub use shadow::ShadowSettings;
pub use shape::{
    MAX_POLYGON_POINTS, Shape, ShapeBlend, ShapeDrawError, ShapeInstance, ShapeRenderer,
};
pub use sprite::{SpriteBlendMode, SpriteRenderer};
pub use sprite_batch::{
    SpriteBatchDraw, SpriteBatchError, SpriteBatchRenderer, SpriteBatchStats, SpriteBatchTarget,
    SpriteDepth, SpriteInstance, SpriteMask,
};
pub use sprite_shader::{SpriteShader, SpriteShaderError};
pub use target::{ViewportTarget, encode_clear, sampled_format};
pub use text::{
    GlyphQuads, LineAlign, TextAlign, TextCase, TextError, TextFit, TextInstance, TextRenderer,
    TextShadow, TextStroke, TextStyle, TextWrap, aligned_origin,
};
pub use texture::{
    RegistryIdentity, Texture2D, TextureError, TextureFilter, TextureId, TextureRegistry,
};
pub use textured_cube::{DrawContext, TexturedCubeRenderer, WorldSettings};
pub use textured_mesh_cache::{
    CachedMeshId, CachedTexturedMeshUpload, MeshSurface, TexturedMeshCacheStats,
};
pub use transparency::{TransparentOrder, TransparentOrderError};
pub use triangle::TriangleRenderer;
pub use uv_rect::{UvRect, UvRectError};

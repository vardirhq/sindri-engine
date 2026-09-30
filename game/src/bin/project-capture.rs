//! A photograph of any project, offscreen, played the way the browser plays
//! it: the same session, the same Weave styling for the viewport, the same
//! draw, with the person's clicks, keys and wheel delivered as a host would.
//!
//! ```bash
//! cargo run -p sindri-causeway --bin project-capture -- \
//!     examples/ui target/ui.png 390 844 click:callsign type:Nova key:Enter wait:0.5
//! ```
//!
//! Steps after the size run in order: `click:<entity>` presses and releases
//! over an element, `wheel:<entity>:<pixels>` scrolls over one (positive is
//! down the list), `key:<Key>` taps a key, `type:<text>` commits characters,
//! and `wait:<seconds>` plays on. Each prints what it hit, so a picture of the
//! wrong thing says why.

#[cfg(not(target_arch = "wasm32"))]
mod capture {
    use std::{collections::BTreeMap, error::Error, fs, io::BufWriter, path::Path, time::Duration};

    use sindri_assets::{AssetBytes, AssetDecoder, FontAssetDecoder, TextureAssetDecoder};
    use sindri_causeway::{Session, extractor};
    use sindri_core::{
        AssetId, LoadedScenes, PREFAB_SUFFIX, PROFILE_SUFFIX, PrefabDocument, ProfileDocument,
        SceneDocument, SceneEntityId, SpriteSheetDocument, World, sheet_id_for,
    };
    use sindri_decay::{PrefabSources, ProfileSources, ScriptSources};
    use sindri_gpu::{GpuContext, GpuRequestOptions};
    use sindri_platform::{InputEvent, InputState, Key, MouseButton};
    use sindri_render::{
        DepthTarget, FrameRenderers, FrameTarget, GlyphRenderer, OffscreenTarget, ShapeRenderer,
        SpriteBatchRenderer, TextRenderer, Texture2D, TextureRegistry, TexturedCubeRenderer,
        Viewport, encode_prepared_frame,
    };
    use sindri_scene::{
        CameraView, SceneExtractor, SceneRuntime, TextureBindings, measure_ui_text,
    };

    const STEP: f32 = 1.0 / 60.0;

    /// Every file under `root` with `extension`, by its asset ID.
    fn files(root: &Path, extension: &str) -> BTreeMap<String, Vec<u8>> {
        let mut found = BTreeMap::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(dir) = pending.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.to_string_lossy().ends_with(extension)
                    && let (Ok(relative), Ok(bytes)) = (path.strip_prefix(root), fs::read(&path))
                {
                    found.insert(relative.to_string_lossy().replace('\\', "/"), bytes);
                }
            }
        }
        found
    }

    fn text(bytes: Vec<u8>) -> Result<String, Box<dyn Error>> {
        Ok(String::from_utf8(bytes)?)
    }

    /// The project's main scene and its entry stylesheets, from `sindri.toml`.
    fn manifest(project: &Path) -> Result<(String, Vec<String>), Box<dyn Error>> {
        let toml = fs::read_to_string(project.join("sindri.toml"))?;
        let quoted = |line: &str| -> Vec<String> {
            line.split('"')
                .skip(1)
                .step_by(2)
                .map(str::to_owned)
                .collect()
        };
        let scene = toml
            .lines()
            .find(|line| line.trim_start().starts_with("main_scene"))
            .and_then(|line| quoted(line).into_iter().next())
            .ok_or("sindri.toml names no main_scene")?;
        let scene = scene.strip_prefix("assets/").unwrap_or(&scene).to_owned();
        let sheets = toml
            .lines()
            .filter(|line| line.trim_start().starts_with("include"))
            .flat_map(quoted)
            .filter(|id| id.ends_with(".weave"))
            .collect();
        Ok((scene, sheets))
    }

    /// A pixel over the middle of `name`, as the last step laid it out.
    fn pixel_of(session: &Session, world: &World, name: &str, size: [f32; 2]) -> Option<[f32; 2]> {
        let entity = world.entity_for_source_id(&SceneEntityId::new(name).ok()?)?;
        let rect = session.screen_ui().rect(entity)?;
        let half = size[1] / 2.0;
        Some([
            size[0] / 2.0 + rect.center[0] * half,
            half - rect.center[1] * half,
        ])
    }

    struct Player {
        session: Session,
        world: World,
        input: InputState,
        scene: SceneExtractor,
        text: TextRenderer,
        size: [f32; 2],
    }

    impl Player {
        fn viewport(&self) -> weave::Viewport {
            weave::Viewport {
                width: self.size[0],
                height: self.size[1],
            }
        }

        /// One fixed step and one draw's worth of styling, as a host runs.
        fn step(&mut self) -> Result<(), Box<dyn Error>> {
            let viewport = (self.size[0], self.size[1]);
            self.session
                .step(&mut self.world, &self.input, viewport, STEP)?;
            self.input.begin_frame(Duration::from_secs_f32(STEP));
            let view = self.viewport();
            let undo = self.session.style(&mut self.world, view)?;
            let sizes = measure_ui_text(&self.world, self.scene.components(), &mut self.text)?;
            self.session.record_drawn(&self.world, view, sizes)?;
            if let Some(undo) = undo {
                undo.undo(&mut self.world);
            }
            Ok(())
        }

        fn play(&mut self, seconds: f32) -> Result<(), Box<dyn Error>> {
            for _ in 0..(seconds / STEP).ceil().max(1.0) as usize {
                self.step()?;
            }
            Ok(())
        }

        fn point_at(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
            let [x, y] = pixel_of(&self.session, &self.world, name, self.size)
                .ok_or_else(|| format!("{name} is not laid out on screen"))?;
            println!("{name} at ({x:.0}, {y:.0})");
            self.input.apply(InputEvent::PointerMoved { x, y });
            Ok(())
        }

        fn perform(&mut self, action: &str) -> Result<(), Box<dyn Error>> {
            let (verb, rest) = action.split_once(':').unwrap_or((action, ""));
            match verb {
                "click" => {
                    self.point_at(rest)?;
                    self.input
                        .apply(InputEvent::ButtonPressed(MouseButton::Left));
                    self.step()?;
                    self.input
                        .apply(InputEvent::ButtonReleased(MouseButton::Left));
                    self.step()?;
                }
                "wheel" => {
                    let (name, pixels) = rest.rsplit_once(':').ok_or("wheel:<entity>:<px>")?;
                    self.point_at(name)?;
                    let pixels: f32 = pixels.parse()?;
                    self.input
                        .apply(InputEvent::Scrolled { x: 0.0, y: -pixels });
                    self.step()?;
                }
                "key" => {
                    let key = Key::from_name(rest).ok_or_else(|| format!("no key {rest}"))?;
                    self.input.apply(InputEvent::KeyPressed(key));
                    self.step()?;
                    self.input.apply(InputEvent::KeyReleased(key));
                    self.step()?;
                }
                "type" => {
                    for c in rest.chars() {
                        self.input.apply(InputEvent::TextInput(c));
                    }
                    self.step()?;
                }
                "wait" => self.play(rest.parse()?)?,
                _ => return Err(format!("unknown step {action}").into()),
            }
            Ok(())
        }
    }

    /// Everything opened: the player, and the images and sheets to bind.
    type Opened = (Player, BTreeMap<String, Vec<u8>>, BTreeMap<String, Vec<u8>>);

    fn open(project: &Path, size: [f32; 2]) -> Result<Opened, Box<dyn Error>> {
        let assets = project.join("assets");
        let (scene_id, sheet_ids) = manifest(project)?;
        let document = SceneDocument::from_json(&fs::read_to_string(assets.join(&scene_id))?)?;
        let mut world = World::default();
        let mut loaded = LoadedScenes::new();
        loaded.enter_keeping_identities(&mut world, &scene_id, &document)?;

        let mut sources = ScriptSources::new();
        for (id, bytes) in files(&assets, ".decay") {
            sources.insert(id, text(bytes)?);
        }
        let mut prefabs = PrefabSources::new();
        for (id, bytes) in files(&assets, PREFAB_SUFFIX) {
            prefabs.insert(id, PrefabDocument::from_json(&text(bytes)?)?);
        }
        let mut profiles = ProfileSources::new();
        for (id, bytes) in files(&assets, PROFILE_SUFFIX) {
            profiles.insert(id, ProfileDocument::from_json(&text(bytes)?)?);
        }
        let weave_sources: BTreeMap<String, String> = files(&assets, ".weave")
            .into_iter()
            .map(|(id, bytes)| Ok((id, text(bytes)?)))
            .collect::<Result<_, Box<dyn Error>>>()?;
        let mut sheets = Vec::new();
        for id in sheet_ids {
            sheets.push(weave::compose(&id, &weave_sources)?);
        }

        let scene = extractor()?;
        let mut session = Session::with_sources(scene.components().clone(), sources)
            .with_prefabs(prefabs)
            .with_profiles(profiles)
            .with_scenes(vec![(scene_id, document)], loaded)
            .with_styles(sheets);
        let view = weave::Viewport {
            width: size[0],
            height: size[1],
        };
        session.settle_styles(&mut world, view)?;
        let mut text = TextRenderer::new();
        for (id, bytes) in files(&assets, ".ttf") {
            let asset = FontAssetDecoder.decode(AssetBytes::new(id.parse::<AssetId>()?, bytes))?;
            text.bind_font(&id, asset.family(), asset.bytes().to_vec());
        }
        let player = Player {
            session,
            world,
            input: InputState::default(),
            scene,
            text,
            size,
        };
        Ok((player, files(&assets, ".png"), files(&assets, ".sheet")))
    }

    pub(super) async fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
        env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            .try_init()
            .ok();
        let [project, out, width, height, steps @ ..] = args.as_slice() else {
            return Err("project-capture <project> <out.png> <width> <height> [steps...]".into());
        };
        let (width, height): (u32, u32) = (width.parse()?, height.parse()?);
        #[allow(clippy::cast_precision_loss)]
        let size = [width as f32, height as f32];
        let (mut player, pngs, sheets) = open(Path::new(project), size)?;
        player.play(0.5)?;
        for step in steps {
            player.perform(step)?;
        }
        player.play(0.3)?;

        let instance = wgpu::Instance::default();
        let gpu = GpuContext::request(&instance, None, &GpuRequestOptions::default()).await?;
        let mut textures = TextureRegistry::new(&gpu.device, &gpu.queue);
        let mut bindings = TextureBindings::new();
        for (id, bytes) in pngs {
            let Ok(asset) =
                TextureAssetDecoder.decode(AssetBytes::new(id.parse::<AssetId>()?, bytes))
            else {
                continue;
            };
            let texture = Texture2D::from_rgba8(
                &gpu.device,
                &gpu.queue,
                &id,
                asset.width(),
                asset.height(),
                asset.rgba8(),
            )?;
            bindings.bind(&id, textures.insert(texture));
            let sheet = id
                .parse::<AssetId>()
                .ok()
                .and_then(|id| sheet_id_for(&id))
                .and_then(|sheet| sheets.get(sheet.as_str()));
            if let Some(json) = sheet {
                bindings.bind_sheet(&id, &SpriteSheetDocument::from_json(&text(json.clone())?)?)?;
            }
        }
        let target = OffscreenTarget::new(&gpu.device, width, height)?;
        let depth = DepthTarget::new(&gpu.device, width, height);
        let mut cubes = TexturedCubeRenderer::new(&gpu.device, OffscreenTarget::FORMAT);
        let mut sprites = SpriteBatchRenderer::new(&gpu.device, OffscreenTarget::FORMAT);
        let mut glyphs = GlyphRenderer::new(&gpu.device, OffscreenTarget::FORMAT);
        let mut shapes = ShapeRenderer::new(&gpu.device, OffscreenTarget::FORMAT);

        let view = player.viewport();
        let undo = player.session.style(&mut player.world, view)?;
        let sizes = measure_ui_text(&player.world, player.scene.components(), &mut player.text)?;
        let prepared = player.scene.extract_animated(
            &player.world,
            Viewport::new(width, height),
            CameraView::default(),
            &bindings,
            SceneRuntime::default()
                .with_animations(player.session.animations())
                .with_effects(player.session.effects())
                .with_text_sizes(&sizes),
        )?;
        if let Some(undo) = undo {
            undo.undo(&mut player.world);
        }
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("project capture"),
            });
        encode_prepared_frame(
            FrameRenderers {
                cube: &mut cubes,
                sprites: &mut sprites,
                text: &mut player.text,
                glyphs: &mut glyphs,
                shapes: &mut shapes,
                textures: &textures,
            },
            &gpu.device,
            &gpu.queue,
            &mut encoder,
            FrameTarget {
                color: target.view(),
                depth: &depth,
            },
            &prepared,
        )?;
        let readback = target.copy_to_buffer(&gpu.device, &mut encoder)?;
        gpu.queue.submit([encoder.finish()]);
        let pixels = readback.read_rgba8(&gpu.device)?;
        let path = Path::new(out);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut png = png::Encoder::new(BufWriter::new(fs::File::create(path)?), width, height);
        png.set_color(png::ColorType::Rgba);
        png.set_depth(png::BitDepth::Eight);
        png.write_header()?.write_image_data(&pixels)?;
        println!("wrote {}", path.display());
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(capture::run(std::env::args().skip(1).collect()))
}

#[cfg(target_arch = "wasm32")]
fn main() {}

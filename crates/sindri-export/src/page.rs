//! The page that loads the host.
//!
//! Generated rather than copied, because the two things that differ between one
//! deployment and the next — what the game is called and where it is served
//! from — are the two things a hand-copied page gets wrong.

/// The page, with `{{name}}`, `{{base}}` and `{{build}}` still in it.
pub const PAGE_TEMPLATE: &str = include_str!("page.html");

/// A project's own brand, shown on the loading screen after the Sindri mark.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Splash {
    /// The image's extension and bytes, written beside the page.
    pub image: Option<(String, Vec<u8>)>,
    pub title: Option<String>,
    pub caption: Option<String>,
    /// `#rrggbb`, already checked.
    pub background: Option<String>,
    /// How long it shows at least.
    pub seconds: f64,
}

impl Splash {
    /// The file the image is written as, named by what is in it so a new
    /// image is never answered from a cache holding the old one.
    #[must_use]
    pub fn image_file(&self) -> Option<String> {
        let (extension, bytes) = self.image.as_ref()?;
        let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        });
        Some(format!("splash-{hash:016x}.{extension}"))
    }
}

/// The `wasm-pack` module the site's one host is built as.
///
/// Named here rather than spelled at each call, because the page and the writer
/// have to agree on it and a second spelling is a page that imports a module
/// nobody built.
pub const HOST_MODULE: &str = "sindri_player";

/// The page for a project, served from `base_path`.
///
/// `base_path` is normalised to start and end with `/`, because
/// `<base href>` means something different without the trailing one: `/repo`
/// resolves `pkg/x.js` against the site root and 404s, while `/repo/` resolves
/// it inside the project. That is the entire GitHub Pages subpath problem, and
/// it is fixed here rather than in a deployment note nobody reads.
#[must_use]
pub fn page_for(name: &str, base_path: &str) -> String {
    page_for_host(name, base_path, HOST_MODULE, "")
}

/// The same page, naming the JavaScript `wasm-pack` produced.
///
/// The module is named after the host crate, which the export cannot know: it
/// reads a project, and a project does not say which binary will run it. So it
/// is a parameter with a default rather than a guess.
/// `build` identifies what was deployed, and is what makes the host cacheable.
///
/// Empty for a local export, which is served fresh anyway and where a changing
/// id in the output would make two exports of one project differ. In CI it is
/// the commit, which is the only thing that answers "is the thing I am looking
/// at the thing I just merged" -- the question that cost three rounds of
/// debugging a fault that was already fixed, because the assets were
/// content-addressed and fresh while the host beside them came from a cache.
#[must_use]
pub fn page_for_host(name: &str, base_path: &str, host_module: &str, build: &str) -> String {
    page_with_splash(name, base_path, host_module, build, None)
}

/// The same page, with a project's brand following the Sindri mark on its
/// loading screen.
#[must_use]
pub fn page_with_splash(
    name: &str,
    base_path: &str,
    host_module: &str,
    build: &str,
    splash: Option<&Splash>,
) -> String {
    let mut base = base_path.trim().to_owned();
    if base.is_empty() {
        base.push('/');
    }
    if !base.starts_with('/') {
        base.insert(0, '/');
    }
    if !base.ends_with('/') {
        base.push('/');
    }
    let build = escape(build.trim());
    // No id means no query at all, rather than a bare `?v=`: a URL that carries
    // an empty parameter is a second spelling of the same file, and a cache
    // that has one has not got the other.
    let cachebust = if build.is_empty() {
        String::new()
    } else {
        format!("?v={build}")
    };
    let (brand, brand_attributes) = splash.map_or_else(
        || (String::new(), String::new()),
        |splash| brand_stage(name, splash),
    );
    PAGE_TEMPLATE
        .replace("{{brand}}", &brand)
        .replace("{{brand_attributes}}", &brand_attributes)
        .replace("{{name}}", &escape(name))
        .replace("{{base}}", &escape(&base))
        .replace("{{host}}", &escape(host_module))
        .replace("{{cachebust}}", &cachebust)
        .replace("{{build}}", &build)
}

/// The brand's stage on the loading screen, and the attributes that tell the
/// page about it.
fn brand_stage(name: &str, splash: &Splash) -> (String, String) {
    let mut parts = Vec::new();
    if let Some(file) = splash.image_file() {
        let alt = escape(splash.title.as_deref().unwrap_or(name));
        parts.push(format!(r#"      <img src="{file}" alt="{alt}">"#));
    }
    if let Some(title) = &splash.title {
        parts.push(format!(
            r#"      <div class="brand-title">{}</div>"#,
            escape(title)
        ));
    }
    if let Some(caption) = &splash.caption {
        parts.push(format!(
            r#"      <div class="brand-caption">{}</div>"#,
            escape(caption)
        ));
    }
    let stage = format!(
        "    <div class=\"stage brand\">\n{}\n    </div>",
        parts.join("\n")
    );
    let mut attributes = format!(r#" data-brand-seconds="{}""#, splash.seconds);
    if let Some(background) = &splash.background {
        attributes.push_str(&format!(
            r#" style="--brand-background: {}""#,
            escape(background)
        ));
    }
    (stage, attributes)
}

/// Text that cannot close a tag or an attribute.
///
/// A project's name comes from a file someone edited, and a name with a quote
/// in it should be a name rather than a broken page.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::page_for;

    use super::page_for_host;

    #[test]
    fn the_host_module_is_named_rather_than_guessed() {
        let page = page_for_host("Game", "/", "my_game", "");
        assert!(page.contains(r#"./pkg/my_game.js"#), "{page}");
        assert!(!page.contains("{{host}}"));
    }

    #[test]
    fn the_name_reaches_the_page() {
        let page = page_for("Orbital Last Stand", "/");
        assert!(page.contains("<title>Orbital Last Stand</title>"), "{page}");
        assert!(!page.contains("{{name}}"));
    }

    /// `<base href="/repo">` resolves `pkg/x.js` against the site root and
    /// 404s. The trailing slash is the whole GitHub Pages subpath problem.
    #[test]
    fn a_subpath_always_ends_in_a_slash() {
        for given in ["/game", "game", "/game/", "game/"] {
            let page = page_for("Game", given);
            assert!(
                page.contains(r#"<base href="/game/">"#),
                "{given} became something else"
            );
        }
    }

    #[test]
    fn no_base_path_is_the_site_root() {
        assert!(page_for("Game", "").contains(r#"<base href="/">"#));
    }

    /// A name with a quote in it should be a name, not a broken page.
    #[test]
    fn a_name_cannot_break_out_of_the_page() {
        let page = page_for(r#"a" onload="steal()"#, "/");
        assert!(!page.contains(r#"onload="steal()"#), "{page}");
        assert!(page.contains("&quot;"));
    }

    /// A player on a browser without WebGPU deserves a sentence, not a blank
    /// canvas.
    #[test]
    fn the_page_says_what_is_missing() {
        let page = page_for("Game", "/");
        assert!(
            page.contains("!navigator.gpu"),
            "no missing-WebGPU capability guard"
        );
        assert!(page.contains("WebGPU"), "no WebGPU message");
        assert!(page.contains("sindri:failed"), "no failure channel");
    }
}

#[cfg(test)]
mod build_stamp_tests {
    use super::{page_for, page_for_host};

    #[test]
    fn a_build_id_versions_the_host_and_the_wasm_it_loads() {
        // Both, because the query on the module does not reach the file the
        // module fetches: versioning only the JavaScript leaves the actual code
        // cacheable, which is the whole fault this exists to close.
        let page = page_for_host("Orbital", "/sindri-engine/", "sindri_player", "a1b2c3d");
        assert!(page.contains("./pkg/sindri_player.js?v=a1b2c3d"), "{page}");
        assert!(
            page.contains("./pkg/sindri_player_bg.wasm?v=a1b2c3d"),
            "{page}"
        );
    }

    #[test]
    fn the_build_is_on_the_page_where_somebody_can_read_it_off_a_phone() {
        let page = page_for_host("Orbital", "/", "sindri_player", "a1b2c3d");
        assert!(page.contains(r#"id="sindri-build">a1b2c3d<"#), "{page}");
    }

    #[test]
    fn no_build_id_leaves_no_query_at_all() {
        // Not a bare `?v=`: that is a second URL for the same file, so a cache
        // holding one has not got the other and the export would be its own
        // cache-miss on every deployment that forgot the flag.
        let page = page_for("Orbital", "/");
        assert!(page.contains("./pkg/sindri_player.js\""), "{page}");
        assert!(!page.contains("?v="), "{page}");
    }

    #[test]
    fn the_trace_is_off_unless_the_url_asks_for_it() {
        // A diagnostic that showed itself to players would be a worse fault
        // than the one it was added to find.
        let page = page_for("Orbital", "/");
        assert!(page.contains(r#"has("input-debug")"#), "{page}");
        assert!(
            page.contains("#sindri-input {\n      display: none;"),
            "{page}"
        );
    }

    #[test]
    fn the_sindri_loading_screen_is_on_every_page_until_the_game_is_ready() {
        let page = page_for("Orbital", "/");
        assert!(page.contains(r#"id="sindri-loading""#), "{page}");
        assert!(page.contains(r#"aria-label="Sindri""#), "{page}");
        // It gives way when the host says so, and to a failure.
        assert!(
            page.contains(r#"addEventListener("sindri:ready""#),
            "{page}"
        );
        assert!(page.contains(r##"querySelector("#sindri-loading")?.remove()"##));
        // No brand, and no placeholder left behind for one.
        assert!(!page.contains("stage brand"), "{page}");
        assert!(!page.contains("{{"), "{page}");
    }

    #[test]
    fn a_brand_follows_the_sindri_mark_as_the_project_describes_it() {
        let splash = super::Splash {
            image: Some(("png".to_owned(), vec![1, 2, 3])),
            title: Some("Vardir <Games>".to_owned()),
            caption: Some("presents".to_owned()),
            background: Some("#101820".to_owned()),
            seconds: 2.5,
        };
        let file = splash.image_file().expect("an image names a file");
        assert!(
            file.starts_with("splash-") && file.ends_with(".png"),
            "{file}"
        );
        let page = super::page_with_splash("Orbital", "/", "sindri_player", "", Some(&splash));
        assert!(page.contains(r#"<div class="stage brand">"#), "{page}");
        assert!(page.contains(&format!(r#"<img src="{file}" alt="Vardir &lt;Games&gt;">"#)));
        assert!(page.contains("Vardir &lt;Games&gt;</div>"), "{page}");
        assert!(page.contains(r#"data-brand-seconds="2.5""#), "{page}");
        assert!(page.contains("--brand-background: #101820"), "{page}");
        assert!(!page.contains("{{"), "{page}");
    }

    #[test]
    fn a_different_image_is_a_different_file() {
        let image = |bytes: Vec<u8>| super::Splash {
            image: Some(("png".to_owned(), bytes)),
            ..super::Splash::default()
        };
        assert_ne!(image(vec![1]).image_file(), image(vec![2]).image_file());
    }
}

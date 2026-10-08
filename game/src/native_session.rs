//! What the native game plays: the runtime session with the project it embeds,
//! and the sound device it plays through.

use sindri_core::ComponentSchemaRegistry;
use sindri_platform::{MaybeAudio, NativeAudioBackend};
use sindri_runtime::Session;

use crate::assets::{prefabs, sources};
use crate::error::CausewayError;

pub type CausewayAudio = MaybeAudio<NativeAudioBackend>;

/// The machine's sound device, or silence on one that has none.
#[allow(clippy::unnecessary_wraps)]
pub fn causeway_audio_backend() -> Result<CausewayAudio, CausewayError> {
    Ok(MaybeAudio::open_or_silent(NativeAudioBackend::new))
}

/// A session backed by the scripts and prefabs the native game embeds.
///
/// # Panics
/// When an embedded prefab does not parse, which the game's own tests refuse.
#[must_use]
pub fn session(components: ComponentSchemaRegistry) -> Session {
    Session::with_sources(components, sources())
        .with_prefabs(prefabs().expect("the embedded prefabs parse"))
}

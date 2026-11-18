//! What the command's tests share.

/// Whether a run stopped for want of a GPU adapter, which skips a picture
/// test unless `SINDRI_REQUIRE_GPU` says this machine must have one.
pub fn no_gpu(report: &serde_json::Value) -> bool {
    let wanting = report["stopped"]
        .as_str()
        .is_some_and(|why| why.contains("needs a GPU adapter"));
    if wanting {
        assert!(
            std::env::var_os("SINDRI_REQUIRE_GPU").is_none(),
            "SINDRI_REQUIRE_GPU is set but a shot found no adapter: {report}"
        );
        eprintln!("skipping the picture: no GPU adapter");
    }
    wanting
}

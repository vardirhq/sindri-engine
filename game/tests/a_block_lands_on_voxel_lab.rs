//! The fixture CI exports to prove voxel collision in a browser, played here
//! natively first: Voxel Lab given a physics world and collision, and an
//! observer that drops a block onto it and says when it has come to rest.
//!
//! If this fails, the browser check would too, and for a reason that is
//! easier to read here.

use std::path::PathBuf;
use std::process::Command;

use sindri_runtime::ProjectRun;

#[test]
fn a_block_dropped_on_voxel_lab_comes_to_rest_on_its_blocks() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let project = std::env::temp_dir().join(format!("voxel-landing-{}", std::process::id()));
    let status = Command::new("python3")
        .arg(repository.join("scripts/browser/prepare-voxel-landing.py"))
        .arg(&project)
        .status()
        .expect("python3 runs");
    assert!(status.success(), "the fixture is prepared");
    let mut run = ProjectRun::open(&project, [960.0, 540.0]).expect("the fixture opens");
    let mut printed = Vec::new();
    for _ in 0..60 * 20 {
        let report = run.step(1.0 / 60.0).expect("the step runs");
        assert!(report.notes().is_empty(), "{:?}", report.notes());
        printed.extend(report.scripts.printed.iter().map(|m| m.message.clone()));
        if printed.iter().any(|line| line.contains("landing verified")) {
            return;
        }
    }
    panic!("the block never came to rest on the voxels: {printed:?}");
}

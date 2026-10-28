//! The crate's landing flash is driven by real solved contact impulses.

use platformer::Run;

#[test]
fn a_falling_crate_flashes_on_impact_and_the_flash_expires() {
    let mut run = Run::open().unwrap();
    let entity = run.entity("crate-impact-flash").unwrap();
    let mut flashed = false;
    for _ in 0..120 {
        let notes = run.step(1.0 / 60.0);
        assert!(notes.is_empty(), "{notes:?}");
        let flash = &run.world.get(entity).unwrap().components["sindri.shape"];
        flashed |= flash["fill"][3].as_f64().is_some_and(|alpha| alpha > 0.4);
    }
    assert!(
        run.board("crate_impacts") > 0.0,
        "falling crate registered a solid impact"
    );
    assert!(
        flashed,
        "the Decay landing response actually lit the crate overlay"
    );
    let flash = &run.world.get(entity).unwrap().components["sindri.shape"];
    assert!(
        flash["fill"][3].as_f64().unwrap().abs() < f64::EPSILON,
        "flash expired at rest"
    );
}

//! What setup must say, in every state a machine can be in.

use super::*;

fn pinned() -> Machine {
    Machine {
        runtime: managed::runtime_for("linux", "x86_64"),
        model: managed::model_file(managed::MODEL),
        memory: Some(12.0),
        ..Machine::default()
    }
}

#[test]
fn a_computer_nothing_is_published_for_is_told_so() {
    let machine = Machine {
        runtime: None,
        ..pinned()
    };
    assert_eq!(setup(&machine), Setup::Unavailable);
}

/// The whole cost is stated before anything starts.
#[test]
fn a_fresh_machine_is_offered_the_whole_download_up_front() {
    let machine = pinned();
    let Setup::Offer(offer) = setup(&machine) else {
        panic!("{:?}", setup(&machine));
    };
    let runtime = machine.runtime.as_ref().expect("pinned").size;
    let model = machine.model.as_ref().expect("pinned").size;
    assert_eq!(offer.download, runtime + model);
    assert_eq!(offer.model_name, "Qwen2.5 Coder 7B");
    assert_eq!(offer.fit, Some(Tier::Recommended));
    assert!(!offer.partly_done);
}

#[test]
fn an_interrupted_setup_offers_to_continue_with_only_what_is_missing() {
    let machine = Machine {
        has_runtime: true,
        ..pinned()
    };
    let Setup::Offer(offer) = setup(&machine) else {
        panic!("{:?}", setup(&machine));
    };
    assert_eq!(offer.download, machine.model.as_ref().expect("pinned").size);
    assert!(offer.partly_done);
}

/// A small machine is told the truth rather than refused.
#[test]
fn a_machine_that_cannot_hold_the_model_hears_it_before_downloading() {
    let small = Machine {
        memory: Some(3.0),
        ..pinned()
    };
    let Setup::Offer(offer) = setup(&small) else {
        panic!("{:?}", setup(&small));
    };
    assert_eq!(offer.fit, Some(Tier::Unsupported));
    let unknown = Machine {
        memory: None,
        ..pinned()
    };
    let Setup::Offer(offer) = setup(&unknown) else {
        panic!("{:?}", setup(&unknown));
    };
    assert_eq!(offer.fit, None, "unknown memory is not no memory");
}

#[test]
fn everything_on_disk_but_unchecked_is_not_ready() {
    let machine = Machine {
        has_runtime: true,
        has_model: true,
        ..pinned()
    };
    assert_eq!(setup(&machine), Setup::Unchecked);
}

#[test]
fn a_model_that_repaired_both_cases_is_ready() {
    let machine = Machine {
        has_runtime: true,
        has_model: true,
        verified: Some(vec![Feature::DecayRepair]),
        ..pinned()
    };
    assert_eq!(
        setup(&machine),
        Setup::Ready {
            verified: vec![Feature::DecayRepair]
        }
    );
}

#[test]
fn a_model_that_could_not_repair_is_unusable() {
    let machine = Machine {
        has_runtime: true,
        has_model: true,
        verified: Some(Vec::new()),
        ..pinned()
    };
    assert_eq!(setup(&machine), Setup::Unusable);
}

/// Required means something in the editor uses it, and is proved by a case.
/// A requirement nothing tests could never be met; one nothing uses would
/// withhold the assistant for a feature that does not exist.
#[test]
fn the_required_features_are_the_ones_the_editor_offers() {
    assert!(Feature::DecayRepair.required());
    assert!(!Feature::StructuredOutput.required());
    assert!(!Feature::Vision.required());
    for feature in Feature::ALL {
        assert!(!feature.required() || feature.tested(), "{feature:?}");
    }
}

#[test]
fn every_feature_has_a_stable_saved_name() {
    for feature in Feature::ALL {
        assert_eq!(Feature::from_id(feature.id()), Some(feature));
    }
    assert_eq!(Feature::from_id("not a feature"), None);
}

#[test]
fn every_step_says_what_it_is_doing() {
    for step in Step::ALL {
        assert!(!step.title().is_empty());
        assert!(step.doing().ends_with('.'), "{step:?}");
    }
}

#[test]
fn sizes_read_the_way_a_person_says_them() {
    assert_eq!(size(31_198_960), "31 MB");
    assert_eq!(size(4_683_073_536), "4.7 GB");
    assert_eq!(size(940_000_000), "940 MB");
    assert_eq!(size(10), "1 MB");
}

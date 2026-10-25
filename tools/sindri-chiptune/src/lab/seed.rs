//! Numeric seeds and portable, versioned UTF-8 phrase hashing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Seed {
    pub(super) value: u64,
    pub(super) text: Option<String>,
}

/// Numeric inputs keep their old values. Phrases use FNV-1a/64 version 1.
pub(super) fn parse(input: &str) -> Result<Seed, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Enter a number or a seed phrase.".into());
    }
    if input.chars().count() > 128 || input.chars().any(char::is_control) {
        return Err("A seed phrase must be a single line of at most 128 characters.".into());
    }
    if let Ok(value) = input.parse::<u64>() {
        return Ok(Seed { value, text: None });
    }
    if input.bytes().all(|b| b.is_ascii_digit()) {
        return Err(
            "That number is larger than a 64-bit seed. Use a shorter number or words.".into(),
        );
    }
    let mut value = 0xcbf2_9ce4_8422_2325_u64;
    for byte in input.bytes() {
        value = (value ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    Ok(Seed {
        value,
        text: Some(input.to_owned()),
    })
}

pub(super) fn random_different(current: u64) -> Result<u64, String> {
    for _ in 0..8 {
        let mut bytes = [0_u8; 8];
        entropy(&mut bytes)?;
        let seed = u64::from_le_bytes(bytes);
        if seed != current {
            return Ok(seed);
        }
    }
    Err("The random source repeated its value; try again.".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn entropy(bytes: &mut [u8; 8]) -> Result<(), String> {
    getrandom::fill(bytes).map_err(|error| format!("Random seed unavailable: {error}"))
}

#[cfg(target_arch = "wasm32")]
fn entropy(bytes: &mut [u8; 8]) -> Result<(), String> {
    web_sys::window()
        .ok_or("Browser window unavailable")?
        .crypto()
        .map_err(|_| "Browser randomness unavailable")?
        .get_random_values_with_u8_array(bytes)
        .map(|_| ())
        .map_err(|_| "Could not obtain a random seed from the browser.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_numbers_and_portable_phrase_vectors_are_stable() {
        assert_eq!(parse("42").unwrap().value, 42);
        assert_eq!(parse(" 0042 ").unwrap().value, 42);
        assert_eq!(parse("18446744073709551615").unwrap().value, u64::MAX);
        assert_eq!(parse("hello").unwrap().value, 0xa430_d846_80aa_bd0b);
        assert_eq!(parse("hello").unwrap().text.as_deref(), Some("hello"));
        assert_eq!(parse("  hello ").unwrap(), parse("hello").unwrap());
        assert_ne!(parse("Hello").unwrap().value, parse("hello").unwrap().value);
        assert_eq!(
            parse("norsk blåtime").unwrap(),
            parse("norsk blåtime").unwrap()
        );
    }

    #[test]
    fn empty_overlong_and_overflowing_numbers_have_clear_errors() {
        for value in [
            "",
            "   ",
            "18446744073709551616",
            "hello\nworld",
            &"a".repeat(129),
        ] {
            assert!(parse(value).is_err(), "accepted invalid seed {value:?}");
        }
    }
    #[test]
    fn saved_phrases_are_validated_against_the_numeric_seed() {
        let settings = crate::lab::Settings::default();
        let mut song =
            crate::lab::song::SongFile::new(&settings, &crate::lab::compose_with(&settings));
        song.seed_text = Some("hello".into());
        assert!(crate::lab::song::SongFile::from_json(&song.to_json()).is_err());
        song.seed = parse("hello").unwrap().value;
        assert!(crate::lab::song::SongFile::from_json(&song.to_json()).is_ok());
    }
}

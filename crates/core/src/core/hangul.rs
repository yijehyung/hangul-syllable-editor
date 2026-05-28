pub const NO_JONG: char = '\0';

pub const HANGUL_SYLLABLE_START: u32 = 0xAC00;
pub const HANGUL_SYLLABLE_END: u32 = 0xD7A3;

pub fn all_hangul_syllables() -> impl Iterator<Item = char> {
    (HANGUL_SYLLABLE_START..=HANGUL_SYLLABLE_END).filter_map(char::from_u32)
}

pub fn decompose_hangul(c: char) -> Option<(usize, usize, usize)> {
    let code = c as u32;
    if (HANGUL_SYLLABLE_START..=HANGUL_SYLLABLE_END).contains(&code) {
        let offset = code - 0xAC00;
        let jong = (offset % 28) as usize;
        let jung = ((offset / 28) % 21) as usize;
        let cho = ((offset / 28) / 21) as usize;
        Some((cho, jung, jong))
    } else {
        None
    }
}

const CHO_JAMO: &[char] = &[
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];
const JUNG_JAMO: &[char] = &[
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ', 'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];
const JONG_JAMO: &[char] = &[
    NO_JONG, 'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ', 'ㅁ', 'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ',
    'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

pub fn get_jamo_char(component: crate::core::types::HangulComponent, idx: usize) -> char {
    use crate::core::types::HangulComponent;
    match component {
        HangulComponent::Cho => CHO_JAMO.get(idx).copied().unwrap_or('?'),
        HangulComponent::Jung => JUNG_JAMO.get(idx).copied().unwrap_or('?'),
        HangulComponent::Jong => match JONG_JAMO.get(idx).copied() {
            Some(NO_JONG) | None => ' ',
            Some(c) => c,
        },
    }
}

pub fn cho_allowed() -> &'static [char] {
    CHO_JAMO
}

pub fn jung_allowed() -> &'static [char] {
    JUNG_JAMO
}

pub fn jong_allowed() -> &'static [char] {
    &JONG_JAMO[1..]
}

pub fn jong_allowed_with_none() -> &'static [char] {
    JONG_JAMO
}

// 옛 초성: Jamo U+1113..=U+115E + Extended-A U+A960..=U+A97C
pub fn cho_allowed_ext() -> Vec<char> {
    let mut v: Vec<char> = CHO_JAMO.to_vec();
    v.extend((0x1113u32..=0x115E).filter_map(char::from_u32));
    v.extend((0xA960u32..=0xA97C).filter_map(char::from_u32));
    v
}

// 옛 중성: Jamo U+1176..=U+11A7 + Extended-B U+D7B0..=U+D7C6
pub fn jung_allowed_ext() -> Vec<char> {
    let mut v: Vec<char> = JUNG_JAMO.to_vec();
    v.extend((0x1176u32..=0x11A7).filter_map(char::from_u32));
    v.extend((0xD7B0u32..=0xD7C6).filter_map(char::from_u32));
    v
}

// 옛 종성 (null 없음): Jamo U+11C3..=U+11FF + Extended-B U+D7CB..=U+D7FB
pub fn jong_allowed_ext() -> Vec<char> {
    let mut v: Vec<char> = JONG_JAMO[1..].to_vec();
    v.extend((0x11C3u32..=0x11FF).filter_map(char::from_u32));
    v.extend((0xD7CBu32..=0xD7FB).filter_map(char::from_u32));
    v
}

// 옛 종성 (null 포함)
pub fn jong_allowed_with_none_ext() -> Vec<char> {
    let mut v = vec!['\0'];
    v.extend(jong_allowed_ext());
    v
}

pub fn allowed_chars_for_target(t: crate::core::types::HangulComponent) -> &'static [char] {
    use crate::core::types::HangulComponent;
    match t {
        HangulComponent::Cho => cho_allowed(),
        HangulComponent::Jung => jung_allowed(),
        HangulComponent::Jong => jong_allowed(),
    }
}

pub fn allowed_chars_extended(t: crate::core::types::HangulComponent) -> Vec<char> {
    use crate::core::types::HangulComponent;
    match t {
        HangulComponent::Cho => cho_allowed_ext(),
        HangulComponent::Jung => jung_allowed_ext(),
        HangulComponent::Jong => jong_allowed_ext(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::HangulComponent;

    fn syllable(cho: u32, jung: u32, jong: u32) -> char {
        char::from_u32(0xAC00 + cho * 21 * 28 + jung * 28 + jong).unwrap()
    }

    #[test]
    fn decompose_ga() {
        assert_eq!(decompose_hangul('가'), Some((0, 0, 0)));
    }

    #[test]
    fn decompose_hih() {
        assert_eq!(decompose_hangul('힣'), Some((18, 20, 27)));
    }

    #[test]
    fn decompose_out_of_range_above() {
        let c = char::from_u32(0xD7A4).unwrap();
        assert_eq!(decompose_hangul(c), None);
    }

    #[test]
    fn decompose_ascii() {
        assert_eq!(decompose_hangul('A'), None);
    }

    #[test]
    fn decompose_standalone_jamo() {
        assert_eq!(decompose_hangul('ㄱ'), None);
    }

    #[test]
    fn decompose_below_range() {
        let c = char::from_u32(0xABFF).unwrap();
        assert_eq!(decompose_hangul(c), None);
    }

    #[test]
    fn decompose_all_syllables_roundtrip() {
        let mut count = 0usize;
        for c in all_hangul_syllables() {
            let (cho, jung, jong) = decompose_hangul(c).expect("should decompose");
            assert!(cho < 19, "cho={cho} out of range");
            assert!(jung < 21, "jung={jung} out of range");
            assert!(jong < 28, "jong={jong} out of range");
            assert_eq!(c, syllable(cho as u32, jung as u32, jong as u32));
            count += 1;
        }
        assert_eq!(count, 11172);
    }

    #[test]
    fn get_cho_first() {
        assert_eq!(get_jamo_char(HangulComponent::Cho, 0), 'ㄱ');
    }

    #[test]
    fn get_cho_last() {
        assert_eq!(get_jamo_char(HangulComponent::Cho, 18), 'ㅎ');
    }

    #[test]
    fn get_cho_oob() {
        assert_eq!(get_jamo_char(HangulComponent::Cho, 19), '?');
    }

    #[test]
    fn get_jung_first() {
        assert_eq!(get_jamo_char(HangulComponent::Jung, 0), 'ㅏ');
    }

    #[test]
    fn get_jung_last() {
        assert_eq!(get_jamo_char(HangulComponent::Jung, 20), 'ㅣ');
    }

    #[test]
    fn get_jung_oob() {
        assert_eq!(get_jamo_char(HangulComponent::Jung, 21), '?');
    }

    #[test]
    fn get_jong_zero_is_space() {
        assert_eq!(get_jamo_char(HangulComponent::Jong, 0), ' ');
    }

    #[test]
    fn get_jong_last() {
        assert_eq!(get_jamo_char(HangulComponent::Jong, 27), 'ㅎ');
    }

    #[test]
    fn get_jong_oob_is_space() {
        assert_eq!(get_jamo_char(HangulComponent::Jong, 28), ' ');
    }

    #[test]
    fn cho_allowed_count() {
        assert_eq!(cho_allowed().len(), 19);
    }

    #[test]
    fn jung_allowed_count() {
        assert_eq!(jung_allowed().len(), 21);
    }

    #[test]
    fn jong_allowed_count() {
        assert_eq!(jong_allowed().len(), 27);
    }

    #[test]
    fn jong_allowed_no_space() {
        assert!(!jong_allowed().contains(&' '));
    }

    #[test]
    fn jong_allowed_with_none_count() {
        assert_eq!(jong_allowed_with_none().len(), 28);
    }

    #[test]
    fn jong_allowed_with_none_starts_with_null() {
        assert_eq!(jong_allowed_with_none()[0], '\0');
    }
}

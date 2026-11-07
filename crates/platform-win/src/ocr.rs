//! Reading the text in an image with Windows' own OCR (`Windows.Media.Ocr`).
//!
//! A Windows OCR engine reads one language, and reads another script as
//! nonsense ("Привет" as "npnaer"), so every language of the user's Windows
//! profile that has OCR installed reads the image, and each word is taken
//! from the engine whose reading of it looks least like nonsense.

use image::{RgbaImage, imageops};
use windows::{
    Foundation::Rect,
    Globalization::Language,
    Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::{OcrEngine, OcrResult},
    Storage::Streams::DataWriter,
    System::UserProfile::GlobalizationPreferences,
    Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx},
    core::HSTRING,
};

/// Below this many pixels the image is enlarged first: Windows OCR misses
/// small screen text (under ~20 px tall) far more often.
const ENLARGE_BELOW: u64 = 1_500_000;
/// At most this many languages read each image.
const MAX_ENGINES: usize = 3;
/// How much two words' boxes must overlap to be the same word.
const SAME_WORD: f32 = 0.5;

/// The text in `image`, a line per line of text. Empty when there is none.
/// Blocking, and it starts COM in the calling thread: give it a thread of
/// its own.
pub fn read(image: &RgbaImage) -> Result<String, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let engines = engines();
    if engines.is_empty() {
        return Err("Windows has no text recognition for your languages \
             (Settings → Time & language → Language: add the language's OCR feature)"
            .into());
    }
    let bitmap = bitmap(image)?;
    let mut readings = Vec::new();
    for (tag, engine) in &engines {
        let result = engine
            .RecognizeAsync(&bitmap)
            .and_then(|operation| operation.get())
            .map_err(message)?;
        readings.push(Reading::of(tag, &result)?);
    }
    Ok(merge(readings))
}

fn message(err: windows::core::Error) -> String {
    format!("Couldn't read the text: {}", err.message())
}

/// An engine per language of the user's profile that has OCR, in the
/// profile's order (the language Windows' own pick would use first).
fn engines() -> Vec<(String, OcrEngine)> {
    let mut engines: Vec<(String, OcrEngine)> = Vec::new();
    let tags: Vec<HSTRING> = GlobalizationPreferences::Languages()
        .map(|tags| tags.into_iter().collect())
        .unwrap_or_default();
    for tag in tags {
        if engines.len() == MAX_ENGINES {
            break;
        }
        let Ok(language) = Language::CreateLanguage(&tag) else {
            continue;
        };
        if !OcrEngine::IsLanguageSupported(&language).unwrap_or(false) {
            continue;
        }
        let Ok(engine) = OcrEngine::TryCreateFromLanguage(&language) else {
            continue;
        };
        // en-US and en-GB read alike: one engine per OCR language.
        let tag = engine
            .RecognizerLanguage()
            .and_then(|language| language.LanguageTag())
            .map(|tag| tag.to_string_lossy())
            .unwrap_or_else(|_| tag.to_string_lossy());
        if engines.iter().all(|(seen, _)| *seen != tag) {
            engines.push((tag, engine));
        }
    }
    if engines.is_empty()
        && let Ok(engine) = OcrEngine::TryCreateFromUserProfileLanguages()
    {
        let tag = engine
            .RecognizerLanguage()
            .and_then(|language| language.LanguageTag())
            .map(|tag| tag.to_string_lossy())
            .unwrap_or_default();
        engines.push((tag, engine));
    }
    engines
}

/// `image` as OCR takes it: enlarged when small, shrunk when too large.
fn bitmap(image: &RgbaImage) -> Result<SoftwareBitmap, String> {
    let max = OcrEngine::MaxImageDimension().unwrap_or(10_000);
    let (w, h) = image.dimensions();
    let resized;
    let image = if (w as u64 * h as u64) < ENLARGE_BELOW && w * 2 <= max && h * 2 <= max {
        resized = imageops::resize(image, w * 2, h * 2, imageops::FilterType::CatmullRom);
        &resized
    } else if w > max || h > max {
        let scale = max as f32 / w.max(h) as f32;
        let (nw, nh) = ((w as f32 * scale) as u32, (h as f32 * scale) as u32);
        resized = imageops::resize(image, nw.max(1), nh.max(1), imageops::FilterType::Triangle);
        &resized
    } else {
        image
    };
    let mut bgra = image.as_raw().clone();
    for px in bgra.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
        px[3] = 255;
    }
    let writer = DataWriter::new().map_err(message)?;
    writer.WriteBytes(&bgra).map_err(message)?;
    let buffer = writer.DetachBuffer().map_err(message)?;
    SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        image.width() as i32,
        image.height() as i32,
        BitmapAlphaMode::Premultiplied,
    )
    .map_err(message)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Bounds {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl From<Rect> for Bounds {
    fn from(rect: Rect) -> Self {
        Self {
            x: rect.X,
            y: rect.Y,
            w: rect.Width,
            h: rect.Height,
        }
    }
}

impl Bounds {
    /// How much of the smaller box the other covers, 0 to 1.
    fn overlap(&self, other: &Bounds) -> f32 {
        let w = (self.x + self.w).min(other.x + other.w) - self.x.max(other.x);
        let h = (self.y + self.h).min(other.y + other.h) - self.y.max(other.y);
        if w <= 0. || h <= 0. {
            return 0.;
        }
        let smaller = (self.w * self.h).min(other.w * other.h);
        if smaller <= 0. { 0. } else { w * h / smaller }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Word {
    text: String,
    bounds: Bounds,
}

/// What one engine read: lines of words.
#[derive(Debug)]
struct Reading {
    /// Words run together, without spaces (Chinese, Japanese).
    joined: bool,
    lines: Vec<Vec<Word>>,
}

impl Reading {
    fn of(tag: &str, result: &OcrResult) -> Result<Self, String> {
        let mut lines = Vec::new();
        for line in result.Lines().map_err(message)? {
            let mut words = Vec::new();
            for word in line.Words().map_err(message)? {
                words.push(Word {
                    text: word.Text().map_err(message)?.to_string_lossy(),
                    bounds: word.BoundingRect().map_err(message)?.into(),
                });
            }
            if !words.is_empty() {
                lines.push(words);
            }
        }
        let joined = ["zh", "ja"].iter().any(|lang| tag.starts_with(lang));
        Ok(Self { joined, lines })
    }

    fn nonsense(&self) -> u32 {
        self.lines
            .iter()
            .flatten()
            .map(|word| nonsense(&word.text))
            .sum()
    }
}

/// The readings made one: the least nonsensical reading as a whole (the
/// first on a tie) gives the lines, and each word is the least nonsensical
/// reading of its spot by any engine. Between equally plausible readings
/// ("Hello" and "Непо"), the one in the script of the rest of its line.
fn merge(mut readings: Vec<Reading>) -> String {
    let Some(best) = (0..readings.len()).min_by_key(|&i| readings[i].nonsense()) else {
        return String::new();
    };
    let base = readings.swap_remove(best);
    let separator = if base.joined { "" } else { " " };
    let mut text = Vec::new();
    for line in &base.lines {
        // Each word's least nonsensical readings, the base's first.
        let choices: Vec<Vec<&str>> = line
            .iter()
            .map(|word| {
                let mut all = vec![word.text.as_str()];
                all.extend(
                    readings
                        .iter()
                        .flat_map(|reading| reading.lines.iter().flatten())
                        .filter(|other| other.bounds.overlap(&word.bounds) >= SAME_WORD)
                        .map(|other| other.text.as_str()),
                );
                let least = all.iter().map(|text| nonsense(text)).min().unwrap_or(0);
                all.retain(|text| nonsense(text) == least);
                all
            })
            .collect();
        let settled = |choice: &Vec<&str>| {
            let first = main_script(choice[0]);
            choice.iter().all(|text| main_script(text) == first)
        };
        let (mut latin, mut cyrillic) = (0, 0);
        for choice in choices.iter().filter(|choice| settled(choice)) {
            match main_script(choice[0]) {
                Some(Script::Latin) => latin += 1,
                Some(Script::Cyrillic) => cyrillic += 1,
                _ => {}
            }
        }
        let line_script = match latin.cmp(&cyrillic) {
            std::cmp::Ordering::Greater => Some(Script::Latin),
            std::cmp::Ordering::Less => Some(Script::Cyrillic),
            std::cmp::Ordering::Equal => None,
        };
        let words: Vec<&str> = choices
            .iter()
            .map(|choice| {
                choice
                    .iter()
                    .find(|text| line_script.is_some() && main_script(text) == line_script)
                    .unwrap_or(&choice[0])
            })
            .copied()
            .collect();
        text.push(words.join(separator));
    }
    text.join(
        "
",
    )
}

/// The script of a word's letters, when it has any of Latin or Cyrillic.
fn main_script(word: &str) -> Option<Script> {
    word.chars().map(script).find(|s| *s != Script::Other)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Script {
    Latin,
    Cyrillic,
    Other,
}

fn script(c: char) -> Script {
    match c {
        'a'..='z' | 'A'..='Z' | '\u{00C0}'..='\u{024F}' => Script::Latin,
        '\u{0400}'..='\u{04FF}' => Script::Cyrillic,
        _ => Script::Other,
    }
}

/// How much a word read by OCR looks like a misreading: letters of two
/// scripts mixed, digits among letters, a capital after a small letter, odd
/// marks inside it. 0 for a plausible word.
fn nonsense(word: &str) -> u32 {
    // Punctuation around a word is normal: "(see", "world,".
    let inner = word.trim_matches(|c: char| !c.is_alphanumeric());
    let mut score = 0;
    let letters: Vec<Script> = inner
        .chars()
        .filter(|c| c.is_alphabetic())
        .map(script)
        .filter(|s| *s != Script::Other)
        .collect();
    if letters.iter().any(|s| *s != letters[0]) {
        score += 2;
    }
    let has_letter = inner.chars().any(char::is_alphabetic);
    let digits = inner.chars().filter(char::is_ascii_digit).count();
    if has_letter && digits > 0 {
        score += 1;
    }
    let chars: Vec<char> = inner.chars().collect();
    if chars
        .windows(2)
        .any(|pair| pair[0].is_lowercase() && pair[1].is_uppercase())
    {
        score += 1;
    }
    let odd = inner
        .chars()
        .filter(|c| {
            !c.is_alphanumeric() && !matches!(c, '-' | '\'' | '’' | '.' | ',' | ':' | '_' | '@')
        })
        .count();
    // "I.I.I", "l.0": digits read as the letters that look like them.
    let lookalikes = has_letter
        && inner.contains(['.', ',', ':'])
        && inner
            .chars()
            .all(|c| matches!(c, 'I' | 'l' | 'O' | 'o' | '.' | ',' | ':') || c.is_ascii_digit());
    score + odd as u32 + lookalikes as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plausible_words_score_zero() {
        for word in [
            "Hello",
            "world,",
            "Привет,",
            "мир!",
            "1.1.1",
            "e-mail",
            "(see",
            "don't",
            "GPUI",
        ] {
            assert_eq!(nonsense(word), 0, "{word}");
        }
    }

    #[test]
    fn misreadings_score_higher() {
        for word in [
            "Pacn03HaaaH\"1e",
            "M/P!",
            "SpotIight",
            "Pаcпознавание",
            "I.I.I",
        ] {
            assert!(nonsense(word) > 0, "{word}");
        }
    }

    fn word(text: &str, x: f32) -> Word {
        Word {
            text: text.into(),
            bounds: Bounds {
                x,
                y: 0.,
                w: 40.,
                h: 12.,
            },
        }
    }

    #[test]
    fn each_word_from_the_better_reading() {
        let english = Reading {
            joined: false,
            lines: vec![vec![
                word("Hello", 0.),
                word("npnaer", 50.),
                word("M/P!", 100.),
            ]],
        };
        let russian = Reading {
            joined: false,
            lines: vec![vec![
                word("Непо", 0.),
                word("Привет,", 50.),
                word("мир!", 100.),
            ]],
        };
        // In a Russian line, the Russian reading of the first word too.
        assert_eq!(merge(vec![english, russian]), "Непо Привет, мир!");
    }

    #[test]
    fn ties_follow_the_line() {
        let english = Reading {
            joined: false,
            lines: vec![vec![
                word("Hello", 0.),
                word("world,", 50.),
                word("Spotlight", 100.),
            ]],
        };
        let russian = Reading {
            joined: false,
            lines: vec![vec![
                word("Непо", 0.),
                word("world,", 50.),
                word("SpotIight", 100.),
            ]],
        };
        assert_eq!(merge(vec![russian, english]), "Hello world, Spotlight");
    }
}

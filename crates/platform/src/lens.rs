//! Searching an image with Google Lens: a page of our own in the temp
//! folder posts it to Lens's upload form in the default browser, so the
//! results open there with the user's Google sign-in and settings. Nothing
//! is uploaded by us.

use std::{fs, io::Cursor, path::PathBuf, thread, time::Duration, time::SystemTime};

use image::{ImageFormat, RgbaImage};

use crate::os;

/// Where the image goes (the form Lens's own "upload an image" uses).
const UPLOAD: &str = "https://lens.google.com/v3/upload";
/// How long the page (holding the picture) is kept for the browser to load it.
const KEEP: Duration = Duration::from_secs(60);

/// Opens Google Lens on `image` in the default browser. Blocking (and on
/// Windows it starts COM in the calling thread): give it a thread of its own.
pub fn search(image: &RgbaImage) -> Result<(), String> {
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|err| format!("Couldn't encode the image: {err}"))?;
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path: PathBuf = std::env::temp_dir().join(format!("loupe-lens-{stamp}.html"));
    fs::write(&path, page(&png, stamp)).map_err(|err| format!("Couldn't write the page: {err}"))?;
    let opened = os::commands::open_blocking(&path.to_string_lossy());
    thread::spawn(move || {
        thread::sleep(KEEP);
        let _ = fs::remove_file(path);
    });
    opened
}

fn page(png: &[u8], stamp: u128) -> String {
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Google Lens</title>
<meta name="color-scheme" content="dark light"></head>
<body style="font:15px 'Segoe UI',sans-serif;display:grid;place-items:center;height:90vh">
<p>Sending the picture to Google Lens…</p>
<form id="f" method="POST" enctype="multipart/form-data" action="{UPLOAD}?ep=ccm&amp;s=&amp;st={stamp}">
<input type="file" name="encoded_image" id="i" hidden>
<noscript><p>Turn on JavaScript to send the picture.</p></noscript>
</form>
<script>
const b = atob("{data}");
const a = new Uint8Array(b.length);
for (let i = 0; i < b.length; i++) a[i] = b.charCodeAt(i);
const t = new DataTransfer();
t.items.add(new File([a], "screen.png", {{ type: "image/png" }}));
document.getElementById("i").files = t.files;
document.getElementById("f").submit();
</script>
</body></html>
"#,
        data = base64(png)
    )
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_pads() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn page_posts_the_image() {
        let page = page(b"foo", 42);
        assert!(
            page.contains(r#"action="https://lens.google.com/v3/upload?ep=ccm&amp;s=&amp;st=42""#)
        );
        assert!(page.contains(r#"atob("Zm9v")"#));
        assert!(page.contains(r#"name="encoded_image""#));
    }
}

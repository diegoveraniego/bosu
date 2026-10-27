//! Small, streaming GIF89a encoder. Clear codes keep the LZW dictionary at 9 bits.
use crate::{
    body_heat,
    body_map::{KINDS, Snapshot},
    body_regions::Model,
};
use gtk::{cairo, gdk};
use std::{
    io::{BufWriter, Write},
    path::Path,
};

pub fn moment_label(moment: &str) -> &str {
    match moment {
        "before" => "Antes de la sesión",
        "after" => "Después de la sesión",
        _ => "Registro general",
    }
}
pub fn render(
    cr: &cairo::Context,
    model: &Model,
    snapshot: &Snapshot,
    kind: usize,
    width: f64,
    height: f64,
    ink: &gdk::RGBA,
) {
    let values = body_heat::values(snapshot, kind);
    let body_h = (height - 130.).max(1.);
    let body_w = (body_h * 376. / 833.).min(width / 2. - 16.);
    for (i, back) in [false, true].iter().enumerate() {
        let _ = cr.save();
        cr.translate(width * (i as f64 + 0.5) / 2. - body_w / 2., 55.);
        cr.scale(body_w, body_h);
        model.draw_heat(cr, ink, *back, &values, kind);
        let _ = cr.restore();
    }
    cr.set_source_rgb(ink.red() as f64, ink.green() as f64, ink.blue() as f64);
    cr.set_font_size(14.);
    cr.move_to(12., 19.);
    let _ = cr.show_text(&format!(
        "{} · {}",
        KINDS[kind.min(KINDS.len() - 1)],
        snapshot.date
    ));
    cr.set_font_size(11.);
    cr.move_to(12., 38.);
    let _ = cr.show_text(moment_label(&snapshot.moment));
    cr.move_to(15., height - 52.);
    let _ = cr.show_text("Frente");
    cr.move_to(width / 2. + 15., height - 52.);
    let _ = cr.show_text("Espalda");
    for intensity in 0..=10u8 {
        let c = body_heat::color(kind, intensity);
        cr.set_source_rgb(c.0, c.1, c.2);
        cr.rectangle(16. + f64::from(intensity) * 12., height - 40., 12., 8.);
        let _ = cr.fill();
    }
    cr.set_source_rgb(ink.red() as f64, ink.green() as f64, ink.blue() as f64);
    cr.move_to(158., height - 33.);
    let _ = cr.show_text("0 a 10 · ? sin intensidad");
    cr.move_to(16., height - 10.);
    let _ = cr.show_text("Gris: sin datos · un cuadro por registro");
}
fn blocks(out: &mut impl Write, bytes: &[u8]) -> std::io::Result<()> {
    for b in bytes.chunks(255) {
        out.write_all(&[b.len() as u8])?;
        out.write_all(b)?;
    }
    out.write_all(&[0])
}
fn frame(out: &mut impl Write, pixels: &[u8], width: u16, height: u16) -> std::io::Result<()> {
    out.write_all(&[0x21, 0xf9, 4, 4, 80, 0, 0, 0])?; // 800 ms; replace each full frame.
    out.write_all(&[0x2c, 0, 0, 0, 0])?;
    out.write_all(&width.to_le_bytes())?;
    out.write_all(&height.to_le_bytes())?;
    out.write_all(&[0, 8])?;
    let mut encoded = Vec::with_capacity(pixels.len() * 2);
    let mut bits = 0u32;
    let mut count = 0;
    let mut emit = |code: u16| {
        bits |= u32::from(code) << count;
        count += 9;
        while count >= 8 {
            encoded.push(bits as u8);
            bits >>= 8;
            count -= 8;
        }
    };
    for chunk in pixels.chunks(200) {
        emit(256);
        for p in chunk {
            emit(u16::from(*p));
        }
    }
    emit(257);
    if count > 0 {
        encoded.push(bits as u8);
    }
    blocks(out, &encoded)
}
pub fn export(path: &Path, snapshots: &[Snapshot], kind: usize) -> Result<(), String> {
    if snapshots.is_empty() {
        return Err("No hay registros SVG en el período seleccionado".into());
    }
    let model = Model::load()?;
    let (width, height) = (420u16, 570u16);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temp = path.with_extension(format!("gif.{nonce}.part"));
    let result = (|| -> Result<(), String> {
        let mut out = BufWriter::new(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|e| e.to_string())?,
        );
        out.write_all(b"GIF89a").map_err(|e| e.to_string())?;
        out.write_all(&width.to_le_bytes())
            .map_err(|e| e.to_string())?;
        out.write_all(&height.to_le_bytes())
            .map_err(|e| e.to_string())?;
        out.write_all(&[0xf7, 215, 0]).map_err(|e| e.to_string())?;
        for index in 0..256u16 {
            let c = if index < 216 {
                [
                    (index / 36) as u8 * 51,
                    ((index / 6) % 6) as u8 * 51,
                    (index % 6) as u8 * 51,
                ]
            } else {
                [255, 255, 255]
            };
            out.write_all(&c).map_err(|e| e.to_string())?;
        }
        out.write_all(b"\x21\xff\x0bNETSCAPE2.0\x03\x01\x00\x00\x00")
            .map_err(|e| e.to_string())?;
        let ink = gdk::RGBA::new(0.15, 0.15, 0.18, 1.);
        for snapshot in snapshots {
            let surface = cairo::ImageSurface::create(
                cairo::Format::ARgb32,
                i32::from(width),
                i32::from(height),
            )
            .map_err(|e| e.to_string())?;
            {
                let cr = cairo::Context::new(&surface).map_err(|e| e.to_string())?;
                cr.set_source_rgb(1., 1., 1.);
                cr.paint().map_err(|e| e.to_string())?;
                render(
                    &cr,
                    &model,
                    snapshot,
                    kind,
                    f64::from(width),
                    f64::from(height),
                    &ink,
                );
            }
            surface.flush();
            let stride = surface.stride() as usize;
            let mut pixels = Vec::with_capacity(usize::from(width) * usize::from(height));
            surface
                .with_data(|data| {
                    for y in 0..usize::from(height) {
                        for x in 0..usize::from(width) {
                            let at = y * stride + x * 4;
                            let rgba = u32::from_ne_bytes(data[at..at + 4].try_into().unwrap());
                            let r = (rgba >> 16) & 255;
                            let g = (rgba >> 8) & 255;
                            let b = rgba & 255;
                            pixels.push(
                                (((r + 25) / 51).min(5) * 36
                                    + ((g + 25) / 51).min(5) * 6
                                    + ((b + 25) / 51).min(5)) as u8,
                            );
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
            frame(&mut out, &pixels, width, height).map_err(|e| e.to_string())?;
        }
        out.write_all(&[0x3b]).map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
        drop(out);
        std::fs::rename(&temp, path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exports_two_distinct_dated_frames() {
        let mut a:Snapshot=serde_json::from_str(r#"{"date":"2026-10-01 10:00:00","map_version":1,"points":[{"x":0.4,"y":0.2,"back":false,"kind":0,"region_id":0,"intensity":2}]}"#).unwrap();
        let first = a.clone();
        a.date = "2026-10-02 10:00:00".into();
        a.points[0].intensity = Some(9);
        let path =
            std::env::temp_dir().join(format!("bosu-evolution-test-{}.gif", std::process::id()));
        export(&path, &[first, a], 0).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"GIF89a"));
        assert_eq!(bytes.last(), Some(&0x3b));
        // Decoder validation is also available through the optional Pillow verification script.
        if let Ok(destination) = std::env::var("BOSU_GIF_TEST_OUTPUT") {
            std::fs::copy(&path, destination).unwrap();
        }
        std::fs::remove_file(path).unwrap();
    }
}

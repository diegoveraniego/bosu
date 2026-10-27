//! SVG geometry from hugobonifay/body-map; see assets/body-map/NOTICE.md.
use gtk::cairo;
use serde::Deserialize;

pub const WIDTH: f64 = 376.;
pub const HEIGHT: f64 = 833.;
#[derive(Deserialize)]
pub struct Region {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub back: bool,
    bounds: [i32; 4],
    center: [f64; 2],
    offset: usize,
}
pub struct Shape {
    pub region: Region,
    mask: cairo::ImageSurface,
    alpha: Vec<u8>,
    bounds: (i32, i32, i32, i32),
    pub center: (f64, f64),
}
pub struct Model {
    pub shapes: Vec<Shape>,
}
impl Model {
    pub fn load() -> Result<Self, String> {
        let regions: Vec<Region> =
            serde_json::from_str(include_str!("assets/body-map/regions.json"))
                .map_err(|e| e.to_string())?;
        let mut shapes = Vec::new();
        for region in regions {
            let [left, top, width, height] = region.bounds;
            if width <= 0 || height <= 0 {
                return Err(format!("Zona vacía: {}", region.name));
            }
            let bytes = include_bytes!("assets/body-map/masks.bin");
            let size = (width * height) as usize;
            let alpha = bytes
                .get(region.offset..region.offset + size)
                .ok_or("Máscara SVG incompleta")?
                .to_vec();
            let rowstride = cairo::Format::A8
                .stride_for_width(width as u32)
                .map_err(|e| e.to_string())?;
            let mut data = vec![0; rowstride as usize * height as usize];
            for y in 0..height as usize {
                data[y * rowstride as usize..y * rowstride as usize + width as usize]
                    .copy_from_slice(&alpha[y * width as usize..(y + 1) * width as usize]);
            }
            let center = (region.center[0], region.center[1]);
            let mask = cairo::ImageSurface::create_for_data(
                data,
                cairo::Format::A8,
                width,
                height,
                rowstride,
            )
            .map_err(|e| e.to_string())?;
            shapes.push(Shape {
                region,
                mask,
                alpha,
                bounds: (left, top, width, height),
                center,
            });
        }
        Ok(Self { shapes })
    }
    pub fn locate(&self, x: f64, y: f64, back: bool) -> Option<usize> {
        if !(0.0..1.0).contains(&x) || !(0.0..1.0).contains(&y) {
            return None;
        }
        let (x, y) = ((x * WIDTH) as i32, (y * HEIGHT) as i32);
        self.shapes.iter().position(|s| {
            let (left, top, w, h) = s.bounds;
            s.region.back == back
                && x >= left
                && x < left + w
                && y >= top
                && y < top + h
                && s.alpha[((y - top) * w + x - left) as usize] > 32
        })
    }
    pub fn draw_heat(
        &self,
        cr: &cairo::Context,
        ink: &gtk::gdk::RGBA,
        back: bool,
        values: &std::collections::HashMap<u32, Option<u8>>,
        kind: usize,
    ) {
        self.draw(cr, ink, back, None);
        let _ = cr.save();
        cr.scale(WIDTH.recip(), HEIGHT.recip());
        for shape in self.shapes.iter().filter(|s| s.region.back == back) {
            if let Some(value) = values.get(&shape.region.id) {
                if let Some(intensity) = value {
                    let c = crate::body_heat::color(kind, *intensity);
                    cr.set_source_rgb(c.0, c.1, c.2);
                } else {
                    cr.set_source_rgb(0.55, 0.55, 0.6);
                }
                let _ = cr.mask_surface(&shape.mask, shape.bounds.0 as f64, shape.bounds.1 as f64);
                if value.is_none() {
                    cr.set_source_rgb(0.1, 0.1, 0.15);
                    cr.set_font_size(20.);
                    cr.move_to(shape.center.0 * WIDTH - 5., shape.center.1 * HEIGHT + 7.);
                    let _ = cr.show_text("?");
                }
            }
        }
        let _ = cr.restore();
    }

    pub fn draw(
        &self,
        cr: &cairo::Context,
        ink: &gtk::gdk::RGBA,
        back: bool,
        highlight: Option<usize>,
    ) {
        let _ = cr.save();
        cr.scale(WIDTH.recip(), HEIGHT.recip());
        for (index, shape) in self
            .shapes
            .iter()
            .enumerate()
            .filter(|(_, s)| s.region.back == back)
        {
            if highlight == Some(index) {
                cr.set_source_rgba(0.20, 0.55, 0.90, 0.65);
            } else {
                cr.set_source_rgba(
                    ink.red() as f64,
                    ink.green() as f64,
                    ink.blue() as f64,
                    0.24,
                );
            }
            let _ = cr.mask_surface(&shape.mask, shape.bounds.0 as f64, shape.bounds.1 as f64);
        }
        let _ = cr.restore();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_svg_regions_have_matching_interior_hit_targets() {
        let model = Model::load().unwrap();
        assert_eq!(model.shapes.len(), 73);
        assert_eq!(model.shapes.iter().filter(|s| !s.region.back).count(), 39);
        assert_eq!(model.shapes.iter().filter(|s| s.region.back).count(), 34);
        let mut ids = std::collections::HashSet::new();
        for (index, s) in model.shapes.iter().enumerate() {
            assert!(ids.insert(s.region.id));
            assert_eq!(
                model.locate(s.center.0, s.center.1, s.region.back),
                Some(index),
                "{}",
                s.region.name
            );
            assert!(!s.region.description.is_empty());
        }
        assert_eq!(model.locate(0., 0., false), None);
        assert_eq!(model.locate(1., 1., true), None);
    }
}

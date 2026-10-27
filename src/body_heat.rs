//! User-reported intensity, kept separate for each sensation.
use crate::body_map::Snapshot;
use std::collections::HashMap;

pub fn values(snapshot: &Snapshot, kind: usize) -> HashMap<u32, Option<u8>> {
    let mut result = HashMap::new();
    if snapshot.map_version != 1 {
        return result;
    }
    for p in snapshot.points.iter().filter(|p| p.kind == kind) {
        if let Some(id) = p.region_id {
            let next = p.intensity.map(|v| v.min(10));
            result
                .entry(id)
                .and_modify(|current: &mut Option<u8>| {
                    *current = match (*current, next) {
                        (Some(a), Some(b)) => Some(a.max(b)),
                        (None, v) => v,
                        (v, None) => v,
                    };
                })
                .or_insert(next);
        }
    }
    result
}
/// A fixed scale across all dates. Strong/worked use their own positive hues.
pub fn color(kind: usize, intensity: u8) -> (f64, f64, f64) {
    let t = f64::from(intensity.min(10)) / 10.;
    let (low, high) = match kind {
        4 => ((0.82, 0.93, 1.), (0.05, 0.35, 0.85)),
        5 => ((0.85, 0.96, 0.85), (0.05, 0.58, 0.25)),
        _ => ((1., 0.94, 0.65), (0.85, 0.08, 0.12)),
    };
    (
        low.0 + (high.0 - low.0) * t,
        low.1 + (high.1 - low.1) * t,
        low.2 + (high.2 - low.2) * t,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregation_uses_maximum_and_does_not_invent_intensity_or_mix_sensations() {
        let s:Snapshot=serde_json::from_str(r#"{"date":"2026-10-27","map_version":1,"points":[{"x":0.5,"y":0.2,"back":false,"kind":0,"region_id":0,"intensity":2},{"x":0.5,"y":0.2,"back":false,"kind":0,"region_id":0,"intensity":7},{"x":0.5,"y":0.2,"back":false,"kind":1,"region_id":0,"intensity":10},{"x":0.5,"y":0.2,"back":false,"kind":0,"region_id":1},{"x":0.5,"y":0.2,"back":false,"kind":0,"region_id":2,"intensity":0}]}"#).unwrap();
        let v = values(&s, 0);
        assert_eq!(v.get(&0), Some(&Some(7)));
        assert_eq!(v.get(&1), Some(&None));
        assert_eq!(v.get(&2), Some(&Some(0)));
        assert_eq!(v.get(&3), None);
        assert_eq!(values(&s, 1).get(&0), Some(&Some(10)));
        let mut old = s;
        old.map_version = 0;
        assert!(values(&old, 0).is_empty());
    }
}

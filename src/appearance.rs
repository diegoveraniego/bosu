//! Native Libadwaita appearance, shared by every Bosu window.
pub fn index(value: &str) -> u32 {
    match value {
        "light" => 1,
        "dark" => 2,
        _ => 0,
    }
}
pub fn value(index: u32) -> &'static str {
    match index {
        1 => "light",
        2 => "dark",
        _ => "",
    }
}
pub fn apply(value: &str) {
    adw::StyleManager::default().set_color_scheme(match value {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::PreferLight,
    });
}

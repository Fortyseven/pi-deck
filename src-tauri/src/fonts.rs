//! System font enumeration — feeds the font pickers in settings.

use std::collections::BTreeSet;

use font_kit::source::SystemSource;

/// Returns the names of all installed font families (deduped, sorted).
#[tauri::command]
pub fn list_system_fonts() -> Vec<String> {
    let mut fams = BTreeSet::new();
    if let Ok(list) = SystemSource::new().all_families() {
        fams.extend(list);
    }
    fams.into_iter().collect()
}

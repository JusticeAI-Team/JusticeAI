use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoValidationInput {
    pub latitude: f64,
    pub longitude: f64,
    pub address_text: String,
    pub area_code: String,
    pub area_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeoValidationResult {
    pub confidence: f64,
    pub conflict_flags: Vec<String>,
    pub suggested_area_code: String,
    pub suggested_area_name: String,
}

pub fn validate_beijing_location(input: &GeoValidationInput) -> GeoValidationResult {
    let mut confidence: f64 = 0.72;
    let mut flags = Vec::new();

    let in_beijing_bbox =
        (115.4..=117.6).contains(&input.longitude) && (39.4..=41.1).contains(&input.latitude);
    if in_beijing_bbox {
        confidence += 0.12;
    } else {
        flags.push("coordinates_outside_beijing_bbox".to_string());
        confidence -= 0.25;
    }

    if input.area_code.starts_with("110") {
        confidence += 0.08;
    } else {
        flags.push("area_code_not_beijing".to_string());
        confidence -= 0.2;
    }

    if input.address_text.contains("北京") || input.area_name.contains("北京") {
        confidence += 0.05;
    } else {
        flags.push("address_without_beijing_hint".to_string());
    }

    if !input.area_code.is_empty()
        && !input.area_name.is_empty()
        && !known_region_name(&input.area_code, &input.area_name)
    {
        flags.push("area_code_name_mismatch".to_string());
        confidence -= 0.18;
    }

    let suggested = suggest_region(input);
    GeoValidationResult {
        confidence: confidence.clamp(0.0, 0.99),
        conflict_flags: flags,
        suggested_area_code: suggested.0,
        suggested_area_name: suggested.1,
    }
}

pub fn conflict_flags_text(flags: &[String]) -> String {
    flags.join(",")
}

fn known_region_name(area_code: &str, area_name: &str) -> bool {
    match area_code {
        "110101" => area_name.contains("东城") || area_name.contains("XX"),
        "110102" => area_name.contains("西城") || area_name.contains("XX"),
        "110105" => area_name.contains("朝阳") || area_name.contains("XX"),
        "110112" => area_name.contains("通州") || area_name.contains("XX"),
        "110115" => area_name.contains("大兴") || area_name.contains("XX"),
        _ => area_code.starts_with("110"),
    }
}

fn suggest_region(input: &GeoValidationInput) -> (String, String) {
    if input.area_code.starts_with("110") && !input.area_name.trim().is_empty() {
        return (input.area_code.clone(), input.area_name.clone());
    }
    if input.longitude > 116.55 && input.latitude > 39.75 && input.latitude < 40.05 {
        return ("110112".to_string(), "北京市通州区".to_string());
    }
    ("110000".to_string(), "北京市".to_string())
}

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AppealScoreInput {
    pub oral_description: String,
    pub worker_name: String,
    pub worker_phone: String,
    pub project_name: String,
    pub employer_name: String,
    pub contractor_name: String,
    pub wage_amount_text: String,
    pub area_name: Option<String>,
    pub material_categories: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AppealScore {
    pub score: i32,
    pub missing_materials: Vec<String>,
}

pub fn calculate_material_score(input: &AppealScoreInput) -> AppealScore {
    let mut score = 0;
    let categories: HashSet<&str> = input.material_categories.iter().map(String::as_str).collect();

    if !input.oral_description.trim().is_empty() {
        score += 20;
    }
    if !input.worker_name.trim().is_empty() && !input.worker_phone.trim().is_empty() {
        score += 15;
    }
    if !input.project_name.trim().is_empty()
        || !input.employer_name.trim().is_empty()
        || !input.contractor_name.trim().is_empty()
        || input
            .area_name
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    {
        score += 15;
    }
    if !input.wage_amount_text.trim().is_empty() {
        score += 15;
    }

    let has_strong = ["labor_contract", "wage_record", "attendance", "bank_statement"]
        .iter()
        .any(|category| categories.contains(category));
    let has_support = [
        "identity",
        "chat_record",
        "work_badge",
        "project_photo",
        "location_screenshot",
        "coworker_statement",
        "other",
    ]
    .iter()
    .any(|category| categories.contains(category));

    if has_strong {
        score += 25;
    }
    if has_support {
        score += 10;
    }

    let mut missing = Vec::new();
    if input.oral_description.trim().is_empty() {
        missing.push("口语化事实描述".to_string());
    }
    if input.worker_name.trim().is_empty() || input.worker_phone.trim().is_empty() {
        missing.push("姓名和联系方式".to_string());
    }
    if input.project_name.trim().is_empty()
        && input.employer_name.trim().is_empty()
        && input.contractor_name.trim().is_empty()
        && input
            .area_name
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
    {
        missing.push("项目或地点信息".to_string());
    }
    if input.wage_amount_text.trim().is_empty() {
        missing.push("欠薪金额说明".to_string());
    }
    if !has_strong {
        missing.push("劳动合同、工资记录、考勤或银行流水之一".to_string());
    }
    if !has_support {
        missing.push("身份、聊天记录、工牌、项目照片、定位截图或工友证明之一".to_string());
    }

    AppealScore {
        score: score.min(100),
        missing_materials: missing,
    }
}

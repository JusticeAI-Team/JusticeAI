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

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> AppealScoreInput {
        AppealScoreInput {
            oral_description: "老板拖欠三个月工资".to_string(),
            worker_name: "张三".to_string(),
            worker_phone: "13800001234".to_string(),
            project_name: "北京市XX区XX地点项目".to_string(),
            employer_name: String::new(),
            contractor_name: "李某".to_string(),
            wage_amount_text: "大概三万二".to_string(),
            area_name: Some("北京市XX区".to_string()),
            material_categories: Vec::new(),
        }
    }

    #[test]
    fn complete_material_chain_scores_full_100() {
        let mut input = base_input();
        input.material_categories = vec!["wage_record".to_string(), "chat_record".to_string()];
        let score = calculate_material_score(&input);
        assert_eq!(score.score, 100);
        assert!(score.missing_materials.is_empty());
    }

    #[test]
    fn incomplete_material_chain_stays_below_submit_threshold() {
        let input = AppealScoreInput {
            oral_description: "老板欠工资".to_string(),
            worker_name: "张三".to_string(),
            worker_phone: "13800001234".to_string(),
            project_name: String::new(),
            employer_name: String::new(),
            contractor_name: String::new(),
            wage_amount_text: String::new(),
            area_name: None,
            material_categories: Vec::new(),
        };
        let score = calculate_material_score(&input);
        assert_eq!(score.score, 35);
        assert!(score.score < 60);
        assert!(score.missing_materials.contains(&"项目或地点信息".to_string()));
        assert!(score
            .missing_materials
            .contains(&"劳动合同、工资记录、考勤或银行流水之一".to_string()));
    }

    #[test]
    fn area_name_counts_as_project_or_location_information() {
        let mut input = base_input();
        input.project_name.clear();
        input.employer_name.clear();
        input.contractor_name.clear();
        input.material_categories = vec!["attendance".to_string()];
        let score = calculate_material_score(&input);
        assert_eq!(score.score, 90);
        assert!(!score.missing_materials.contains(&"项目或地点信息".to_string()));
    }
}

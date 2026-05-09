use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelContract {
    pub provider_style: String,
    pub base_url: String,
    pub chat_endpoint: String,
    pub model_name: String,
    pub json_mode_supported: bool,
    pub api_key_configured: bool,
    pub is_placeholder: bool,
}

#[derive(Debug, Clone)]
pub struct ExtractionInput {
    pub title: String,
    pub area_name: String,
    pub source_type: String,
    pub risk_level: String,
}

#[derive(Debug, Clone)]
pub struct ExtractedEntity {
    pub entity_type: String,
    pub entity_name: String,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct ExtractedRelation {
    pub relation_type: String,
    pub source_index: usize,
    pub target_index: usize,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct ExtractionOutput {
    pub entities: Vec<ExtractedEntity>,
    pub relations: Vec<ExtractedRelation>,
    pub summary: String,
    pub is_placeholder: bool,
    pub model_contract: ModelContract,
}

#[derive(Debug, Clone)]
pub struct RecommendationInput {
    pub title: String,
    pub area_name: String,
    pub risk_level: String,
    pub source_type: String,
    pub entity_count: usize,
    pub alert_count: usize,
    pub dispatch_count: usize,
    pub reference_cases: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RecommendationOutput {
    pub reason_summary: String,
    pub disposal_advice: Vec<String>,
    pub is_placeholder: bool,
    pub model_contract: ModelContract,
}

#[derive(Debug, Clone)]
pub struct ReportInput {
    pub title: String,
    pub report_type: String,
    pub period: String,
    pub case_count: i64,
    pub high_risk_count: i64,
    pub alert_count: i64,
    pub dispatch_count: i64,
}

#[derive(Debug, Clone)]
pub struct ReportOutput {
    pub summary: String,
    pub content: String,
    pub is_placeholder: bool,
    pub model_contract: ModelContract,
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleAiService {
    client: Client,
    base_url: String,
    preferred_model_name: String,
    api_key: Option<String>,
}

impl OpenAiCompatibleAiService {
    pub fn new(
        client: Client,
        base_url: impl Into<String>,
        preferred_model_name: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            preferred_model_name: preferred_model_name.into(),
            api_key: api_key
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        }
    }

    pub fn configured_contract(&self) -> ModelContract {
        ModelContract {
            provider_style: "openai_chat_completion_compatible".to_string(),
            base_url: self.base_url.clone(),
            chat_endpoint: "/chat/completions".to_string(),
            model_name: self.preferred_model_name.clone(),
            json_mode_supported: false,
            api_key_configured: self.api_key.is_some(),
            is_placeholder: false,
        }
    }

    pub async fn extract_case_graph(&self, input: &ExtractionInput) -> ExtractionOutput {
        let fallback = fallback_extraction(input, self.configured_contract());
        let prompt = format!(
            "Case title: {}\nArea: {}\nSource type: {}\nRisk level: {}\n\
             Return a JSON object with keys summary, entities, relations.\n\
             Allowed entity_type values: person, organization, department, event, area, risk_factor.\n\
             entities is an array of objects {{\"entity_type\":string,\"entity_name\":string,\"confidence\":number}}.\n\
             relations is an array of objects {{\"relation_type\":string,\"source_entity_name\":string,\"target_entity_name\":string,\"confidence\":number}}.\n\
             Every source_entity_name and target_entity_name in relations must exactly match an entity_name that appears in entities.\n\
             Only return JSON.",
            input.title, input.area_name, input.source_type, input.risk_level
        );

        let system = "You are JusticeAI extraction engine. Return strict JSON only without explanation.";
        let response = match self.chat_json::<ExtractionModelResponse>(system, &prompt, 0.0, 900).await {
            Ok(value) => value,
            Err(error) => {
                warn!(error = %error, "AI extraction request failed, using deterministic fallback");
                return fallback;
            }
        };

        let entities = response
            .entities
            .into_iter()
            .filter(|entity| !entity.entity_name.trim().is_empty())
            .map(|entity| ExtractedEntity {
                entity_type: normalize_entity_type(&entity.entity_type),
                entity_name: entity.entity_name.trim().to_string(),
                confidence: clamp_confidence(entity.confidence),
            })
            .collect::<Vec<_>>();

        if entities.is_empty() {
            return fallback;
        }

        let relations = response
            .relations
            .into_iter()
            .filter_map(|relation| {
                let source_index = find_entity_index(&entities, &relation.source_entity_name)?;
                let target_index = find_entity_index(&entities, &relation.target_entity_name)?;
                Some(ExtractedRelation {
                    relation_type: normalize_relation_type(&relation.relation_type),
                    source_index,
                    target_index,
                    confidence: clamp_confidence(relation.confidence),
                })
            })
            .collect::<Vec<_>>();

        ExtractionOutput {
            entities,
            relations,
            summary: response.summary.trim().to_string(),
            is_placeholder: false,
            model_contract: self.resolved_contract().await,
        }
    }

    pub async fn recommend_case_action(
        &self,
        input: &RecommendationInput,
    ) -> RecommendationOutput {
        let fallback = fallback_recommendation(input, self.configured_contract());
        let reference_cases = if input.reference_cases.is_empty() {
            "None".to_string()
        } else {
            input.reference_cases.join("\n")
        };

        let prompt = format!(
            "Case title: {}\nArea: {}\nRisk level: {}\nSource type: {}\nEntity count: {}\nAlert count: {}\nDispatch count: {}\nReference cases:\n{}\n\
             Return JSON only with keys reason_summary and disposal_advice.\n\
             disposal_advice must be an array with 3 concise action items.",
            input.title,
            input.area_name,
            input.risk_level,
            input.source_type,
            input.entity_count,
            input.alert_count,
            input.dispatch_count,
            reference_cases
        );

        let system = "You are JusticeAI risk analyst. Return strict JSON only, focused on prosecutorial grassroots governance risk handling.";
        match self
            .chat_json::<RecommendationModelResponse>(system, &prompt, 0.1, 600)
            .await
        {
            Ok(response) if !response.reason_summary.trim().is_empty() => RecommendationOutput {
                reason_summary: response.reason_summary.trim().to_string(),
                disposal_advice: normalize_disposal_advice(response.disposal_advice),
                is_placeholder: false,
                model_contract: self.resolved_contract().await,
            },
            Ok(_) => fallback,
            Err(error) => {
                warn!(error = %error, "AI recommendation request failed, using deterministic fallback");
                fallback
            }
        }
    }

    pub async fn generate_report(&self, input: &ReportInput) -> ReportOutput {
        let fallback = fallback_report(input, self.configured_contract());
        let prompt = format!(
            "Title: {}\nReport type: {}\nPeriod: {}\nCase count: {}\nHigh risk count: {}\nAlert count: {}\nDispatch count: {}\n\
             Return JSON only with keys summary and content_markdown.\n\
             content_markdown should be a complete markdown report.",
            input.title,
            input.report_type,
            input.period,
            input.case_count,
            input.high_risk_count,
            input.alert_count,
            input.dispatch_count
        );

        let system = "You are JusticeAI reporting engine. Return strict JSON only, and make content_markdown suitable for direct persistence.";
        match self
            .chat_json::<ReportModelResponse>(system, &prompt, 0.2, 1400)
            .await
        {
            Ok(response)
                if !response.summary.trim().is_empty()
                    && !response.content_markdown.trim().is_empty() =>
            {
                ReportOutput {
                    summary: response.summary.trim().to_string(),
                    content: response.content_markdown.trim().to_string(),
                    is_placeholder: false,
                    model_contract: self.resolved_contract().await,
                }
            }
            Ok(_) => fallback,
            Err(error) => {
                warn!(error = %error, "AI report request failed, using deterministic fallback");
                fallback
            }
        }
    }

    async fn resolved_contract(&self) -> ModelContract {
        let resolved_model_name = self
            .resolve_model_name()
            .await
            .unwrap_or_else(|_| self.preferred_model_name.clone());

        ModelContract {
            model_name: resolved_model_name,
            ..self.configured_contract()
        }
    }

    async fn resolve_model_name(&self) -> Result<String, String> {
        if self.preferred_model_name.trim().is_empty() {
            let models = self.fetch_models().await?;
            return models
                .into_iter()
                .next()
                .ok_or_else(|| "no models returned by OpenAI-compatible endpoint".to_string());
        }

        let preferred = self.preferred_model_name.trim();
        let models = self.fetch_models().await?;
        if models.iter().any(|model| model == preferred) {
            return Ok(preferred.to_string());
        }

        models
            .into_iter()
            .next()
            .ok_or_else(|| format!("preferred model '{preferred}' not found and no fallback model available"))
    }

    async fn fetch_models(&self) -> Result<Vec<String>, String> {
        let url = format!("{}/models", self.base_url);
        let mut request = self.client.get(url).timeout(Duration::from_secs(20));
        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await.map_err(|error| error.to_string())?;
        let status = response.status();
        let body = response.text().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("model discovery failed with status {status}: {body}"));
        }

        let parsed: ModelListResponse = serde_json::from_str(&body).map_err(|error| error.to_string())?;
        Ok(parsed.data.into_iter().map(|model| model.id).collect())
    }

    async fn chat_json<T>(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<T, String>
    where
        T: for<'de> Deserialize<'de>,
    {
        let model = self.resolve_model_name().await?;
        let url = format!("{}/chat/completions", self.base_url);
        let body = ChatCompletionRequest {
            model,
            messages: vec![
                ChatMessageRequest {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessageRequest {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            temperature,
            max_tokens,
        };

        let mut request = self
            .client
            .post(url)
            .timeout(Duration::from_secs(90))
            .json(&body);
        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }

        let response = request.send().await.map_err(|error| error.to_string())?;
        let status = response.status();
        let raw_body = response.text().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("chat completion failed with status {status}: {raw_body}"));
        }

        let completion: ChatCompletionResponse =
            serde_json::from_str(&raw_body).map_err(|error| format!("invalid completion body: {error}"))?;
        let text = completion
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .map(normalize_chat_content)
            .ok_or_else(|| "chat completion returned no choices".to_string())?;
        let json_text = extract_json_block(&text)?;
        serde_json::from_str(&json_text)
            .map_err(|error| format!("failed to parse model JSON '{json_text}': {error}"))
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessageRequest>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct ChatMessageRequest {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoiceResponse>,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceResponse {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ModelListResponse {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ExtractionModelResponse {
    summary: String,
    entities: Vec<ExtractionEntityPayload>,
    relations: Vec<ExtractionRelationPayload>,
}

#[derive(Debug, Deserialize)]
struct ExtractionEntityPayload {
    entity_type: String,
    entity_name: String,
    #[serde(default = "default_confidence")]
    confidence: f64,
}

#[derive(Debug, Deserialize)]
struct ExtractionRelationPayload {
    relation_type: String,
    source_entity_name: String,
    target_entity_name: String,
    #[serde(default = "default_confidence")]
    confidence: f64,
}

#[derive(Debug, Deserialize)]
struct RecommendationModelResponse {
    reason_summary: String,
    disposal_advice: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ReportModelResponse {
    summary: String,
    content_markdown: String,
}

fn normalize_chat_content(content: serde_json::Value) -> String {
    match content {
        serde_json::Value::String(text) => text,
        serde_json::Value::Array(items) => items
            .into_iter()
            .filter_map(|item| match item {
                serde_json::Value::Object(map) => map
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                serde_json::Value::String(text) => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(""),
        other => other.to_string(),
    }
}

fn extract_json_block(text: &str) -> Result<String, String> {
    let trimmed = text.trim();
    let candidate = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(str::trim)
        .and_then(|value| value.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);

    if candidate.starts_with('{') && candidate.ends_with('}') {
        return Ok(candidate.to_string());
    }

    let start = candidate
        .find('{')
        .ok_or_else(|| format!("no JSON object found in model response: {candidate}"))?;
    let end = candidate
        .rfind('}')
        .ok_or_else(|| format!("no JSON object end found in model response: {candidate}"))?;
    Ok(candidate[start..=end].to_string())
}

fn normalize_entity_type(entity_type: &str) -> String {
    match entity_type.trim().to_ascii_lowercase().as_str() {
        "person" | "people" | "individual" => "person".to_string(),
        "organization" | "org" => "organization".to_string(),
        "department" => "department".to_string(),
        "area" | "location" | "region" => "area".to_string(),
        "risk_factor" | "riskfactor" | "issue" | "problem" => "risk_factor".to_string(),
        "event" => "event".to_string(),
        _ => entity_type.trim().to_ascii_lowercase(),
    }
}

fn normalize_relation_type(relation_type: &str) -> String {
    let relation_type = relation_type.trim().to_ascii_lowercase();
    if relation_type.is_empty() {
        "related_to".to_string()
    } else {
        relation_type
    }
}

fn normalize_disposal_advice(items: Vec<String>) -> Vec<String> {
    let normalized = items
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .take(5)
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        vec![
            "start manual review and verify recurrence signals".to_string(),
            "coordinate disposal responsibility across departments".to_string(),
            "track closure and feedback in the supervision dashboard".to_string(),
        ]
    } else {
        normalized
    }
}

fn clamp_confidence(confidence: f64) -> f64 {
    confidence.clamp(0.0, 1.0)
}

fn default_confidence() -> f64 {
    0.85
}

fn find_entity_index(entities: &[ExtractedEntity], target_name: &str) -> Option<usize> {
    let target = target_name.trim();
    entities
        .iter()
        .position(|entity| entity.entity_name.eq_ignore_ascii_case(target))
        .or_else(|| {
            let lowered = target.to_ascii_lowercase();
            entities.iter().position(|entity| {
                entity.entity_name.to_ascii_lowercase().contains(&lowered)
                    || lowered.contains(&entity.entity_name.to_ascii_lowercase())
            })
        })
}

fn fallback_extraction(input: &ExtractionInput, contract: ModelContract) -> ExtractionOutput {
    let mut entities = vec![
        ExtractedEntity {
            entity_type: "event".to_string(),
            entity_name: input.title.clone(),
            confidence: 0.96,
        },
        ExtractedEntity {
            entity_type: "area".to_string(),
            entity_name: input.area_name.clone(),
            confidence: 0.99,
        },
        ExtractedEntity {
            entity_type: "person".to_string(),
            entity_name: format!("{}-subject", input.title),
            confidence: 0.91,
        },
    ];

    if input.risk_level == "high" {
        entities.push(ExtractedEntity {
            entity_type: "risk_factor".to_string(),
            entity_name: "high_risk_escalation".to_string(),
            confidence: 0.87,
        });
    }

    let mut relations = vec![
        ExtractedRelation {
            relation_type: "event_area".to_string(),
            source_index: 0,
            target_index: 1,
            confidence: 0.94,
        },
        ExtractedRelation {
            relation_type: "person_event".to_string(),
            source_index: 2,
            target_index: 0,
            confidence: 0.9,
        },
    ];

    if input.risk_level == "high" {
        relations.push(ExtractedRelation {
            relation_type: "event_risk_factor".to_string(),
            source_index: 0,
            target_index: 3,
            confidence: 0.86,
        });
    }

    let mut fallback_contract = contract;
    fallback_contract.is_placeholder = true;

    ExtractionOutput {
        entities,
        relations,
        summary: format!(
            "fallback extraction completed for '{}' because the model response was unavailable",
            input.title
        ),
        is_placeholder: true,
        model_contract: fallback_contract,
    }
}

fn fallback_recommendation(
    input: &RecommendationInput,
    contract: ModelContract,
) -> RecommendationOutput {
    let mut fallback_contract = contract;
    fallback_contract.is_placeholder = true;

    let references = if input.reference_cases.is_empty() {
        "no similar cases retrieved".to_string()
    } else {
        format!("similar cases: {}", input.reference_cases.join("; "))
    };

    RecommendationOutput {
        reason_summary: format!(
            "{} is currently tagged as {} risk in {} from {}. Retrieval context: {}.",
            input.title, input.risk_level, input.area_name, input.source_type, references
        ),
        disposal_advice: vec![
            format!("verify whether {} is recurring in {}", input.title, input.area_name),
            "coordinate disposal and supervision with the responsible department".to_string(),
            "track alert closure and resident feedback in the next reporting cycle".to_string(),
        ],
        is_placeholder: true,
        model_contract: fallback_contract,
    }
}

fn fallback_report(input: &ReportInput, contract: ModelContract) -> ReportOutput {
    let mut fallback_contract = contract;
    fallback_contract.is_placeholder = true;

    let summary = format!(
        "{} for {} covers {} cases, {} high-risk cases, {} alerts and {} dispatch tasks.",
        input.report_type,
        input.period,
        input.case_count,
        input.high_risk_count,
        input.alert_count,
        input.dispatch_count
    );

    ReportOutput {
        summary: summary.clone(),
        content: format!(
            "# {}\n\n\
             - report_type: {}\n\
             - period: {}\n\
             - case_count: {}\n\
             - high_risk_count: {}\n\
             - alert_count: {}\n\
             - dispatch_count: {}\n\
             - generation_mode: fallback\n\n\
             ## Summary\n{}\n",
            input.title,
            input.report_type,
            input.period,
            input.case_count,
            input.high_risk_count,
            input.alert_count,
            input.dispatch_count,
            summary
        ),
        is_placeholder: true,
        model_contract: fallback_contract,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_block_reads_fenced_json() {
        let text = "```json\n{\"ok\":true}\n```";
        let json = extract_json_block(text).unwrap();
        assert_eq!(json, "{\"ok\":true}");
    }

    #[test]
    fn extract_json_block_reads_inline_json() {
        let text = "prefix {\"name\":\"justiceai\"} suffix";
        let json = extract_json_block(text).unwrap();
        assert_eq!(json, "{\"name\":\"justiceai\"}");
    }

    #[test]
    fn normalize_chat_content_handles_array_shape() {
        let value = serde_json::json!([
            { "text": "hello" },
            { "text": " world" }
        ]);
        assert_eq!(normalize_chat_content(value), "hello world");
    }
}

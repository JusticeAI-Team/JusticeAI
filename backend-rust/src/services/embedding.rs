use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct EmbeddingContract {
    pub provider_style: String,
    pub base_url: String,
    pub endpoint: String,
    pub model_name: String,
    pub api_key_configured: bool,
    pub is_placeholder: bool,
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleEmbeddingService {
    client: Client,
    base_url: String,
    endpoint: String,
    model_name: String,
    api_key: Option<String>,
}

impl OpenAiCompatibleEmbeddingService {
    pub fn new(
        client: Client,
        base_url: impl Into<String>,
        endpoint: impl Into<String>,
        model_name: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            endpoint: endpoint.into(),
            model_name: model_name.into(),
            api_key: api_key
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        }
    }

    pub fn contract(&self) -> EmbeddingContract {
        EmbeddingContract {
            provider_style: "openai_embeddings_compatible".to_string(),
            base_url: self.base_url.clone(),
            endpoint: self.endpoint.clone(),
            model_name: self.model_name.clone(),
            api_key_configured: self.api_key.is_some(),
            is_placeholder: false,
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.base_url.trim().is_empty() && !self.model_name.trim().is_empty()
    }

    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>, String> {
        let mut result = self.embed_texts(&[text.to_string()]).await?;
        let first = result
            .drain(..)
            .next()
            .ok_or_else(|| "embedding service returned no vectors".to_string())?;
        Ok(first)
    }

    pub async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        if !self.is_configured() {
            return Err("embedding service is not configured".to_string());
        }

        let url = format!("{}{}", self.base_url, normalize_endpoint(&self.endpoint));
        let body = EmbeddingRequest {
            model: self.model_name.clone(),
            input: texts.to_vec(),
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
            return Err(format!(
                "embedding request failed with status {status}: {raw_body}"
            ));
        }

        let parsed: EmbeddingResponse = serde_json::from_str(&raw_body)
            .map_err(|error| format!("invalid embedding response: {error}: {raw_body}"))?;
        if parsed.data.is_empty() {
            return Err("embedding service returned empty data".to_string());
        }

        Ok(parsed.data.into_iter().map(|item| item.embedding).collect())
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingItem>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingItem {
    embedding: Vec<f32>,
}

fn normalize_endpoint(endpoint: &str) -> String {
    if endpoint.starts_with('/') {
        endpoint.to_string()
    } else {
        format!("/{}", endpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_endpoint_keeps_leading_slash() {
        assert_eq!(normalize_endpoint("/embeddings"), "/embeddings");
    }

    #[test]
    fn normalize_endpoint_adds_leading_slash() {
        assert_eq!(normalize_endpoint("embeddings"), "/embeddings");
    }
}

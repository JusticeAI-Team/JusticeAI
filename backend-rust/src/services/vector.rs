use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct VectorCaseDocument {
    pub id: String,
    pub case_id: String,
    pub case_code: String,
    pub title: String,
    pub summary: String,
    pub risk_level: String,
    pub source_type: String,
    pub area_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCaseHit {
    pub id: String,
    pub case_id: String,
    pub case_code: String,
    pub title: String,
    pub risk_level: String,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct VectorSyncResult {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct MilvusVectorStore {
    client: Client,
    address: String,
    token: Option<String>,
    collection_name: String,
    dimension: usize,
}

impl MilvusVectorStore {
    pub fn new(
        client: Client,
        address: impl Into<String>,
        token: Option<String>,
        collection_name: impl Into<String>,
        dimension: usize,
    ) -> Self {
        Self {
            client,
            address: address.into().trim_end_matches('/').to_string(),
            token: token
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            collection_name: collection_name.into(),
            dimension,
        }
    }

    pub async fn upsert_case_vector(
        &self,
        document: &VectorCaseDocument,
    ) -> Result<VectorSyncResult, String> {
        if self.address.trim().is_empty() {
            return Ok(VectorSyncResult {
                status: "not_configured".to_string(),
                message: "Milvus address is not configured".to_string(),
            });
        }

        self.ensure_collection().await?;

        let vector = vectorize_text(&format!(
            "{}\n{}\n{}\n{}\n{}",
            document.title, document.summary, document.area_name, document.risk_level, document.source_type
        ), self.dimension);

        let payload = json!({
            "collectionName": self.collection_name,
            "data": [
                {
                    "id": document.id,
                    "case_id": document.case_id,
                    "case_code": document.case_code,
                    "title": document.title,
                    "summary": document.summary,
                    "risk_level": document.risk_level,
                    "source_type": document.source_type,
                    "area_name": document.area_name,
                    "embedding": vector
                }
            ]
        });

        self.post_json("/v2/vectordb/entities/upsert", payload).await?;
        Ok(VectorSyncResult {
            status: "indexed".to_string(),
            message: format!("indexed case {} into Milvus", document.case_code),
        })
    }

    pub async fn search_similar_cases(
        &self,
        query_text: &str,
        exclude_case_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SimilarCaseHit>, String> {
        if self.address.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.ensure_collection().await?;
        let vector = vectorize_text(query_text, self.dimension);
        let payload = json!({
            "collectionName": self.collection_name,
            "data": [vector],
            "annsField": "embedding",
            "limit": limit as i64,
            "outputFields": ["id", "case_id", "case_code", "title", "risk_level"]
        });

        let response = self.post_json("/v2/vectordb/entities/search", payload).await?;
        let mut hits = parse_search_hits(response);
        if let Some(exclude_case_id) = exclude_case_id {
            hits.retain(|hit| hit.case_id != exclude_case_id);
        }
        hits.truncate(limit);
        Ok(hits)
    }

    async fn ensure_collection(&self) -> Result<(), String> {
        let payload = json!({
            "collectionName": self.collection_name,
            "dimension": self.dimension as i64,
            "metricType": "COSINE",
            "idType": "VarChar",
            "autoID": false,
            "primaryFieldName": "id",
            "vectorFieldName": "embedding",
            "enableDynamicField": true
        });

        match self.post_json("/v2/vectordb/collections/create", payload).await {
            Ok(_) => Ok(()),
            Err(error) if error.contains("already exist") || error.contains("already exists") => Ok(()),
            Err(error) => Err(error),
        }
    }

    async fn post_json(&self, path: &str, payload: serde_json::Value) -> Result<serde_json::Value, String> {
        let url = format!("{}{}", self.address, path);
        let mut request = self.client.post(url).json(&payload);
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await.map_err(|error| error.to_string())?;
        let status = response.status();
        let body = response.text().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("Milvus request failed with status {status}: {body}"));
        }

        let value: serde_json::Value =
            serde_json::from_str(&body).map_err(|error| format!("invalid Milvus response: {error}: {body}"))?;
        if let Some(code) = value.get("code").and_then(serde_json::Value::as_i64) {
            if code != 0 && code != 200 {
                return Err(format!("Milvus returned non-zero code {code}: {body}"));
            }
        }
        Ok(value)
    }
}

pub fn vectorize_text(text: &str, dimension: usize) -> Vec<f32> {
    let mut vector = vec![0.0_f32; dimension];
    for token in tokenize(text) {
        let mut hash = 2166136261_u32;
        for byte in token.as_bytes() {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(16777619);
        }
        let index = (hash as usize) % dimension;
        vector[index] += 1.0;
    }

    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_alphanumeric())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn parse_search_hits(value: serde_json::Value) -> Vec<SimilarCaseHit> {
    let candidates = value
        .get("data")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    candidates
        .into_iter()
        .flat_map(|entry| match entry {
            serde_json::Value::Array(items) => items,
            item => vec![item],
        })
        .filter_map(|entry| {
            let entity = entry
                .get("entity")
                .and_then(serde_json::Value::as_object)
                .cloned()
                .unwrap_or_default();
            let id = string_field(&entry, "id").or_else(|| entity.get("id").and_then(serde_json::Value::as_str).map(str::to_string))?;
            let case_id = string_field(&entry, "case_id")
                .or_else(|| entity.get("case_id").and_then(serde_json::Value::as_str).map(str::to_string))
                .unwrap_or_default();
            let case_code = string_field(&entry, "case_code")
                .or_else(|| entity.get("case_code").and_then(serde_json::Value::as_str).map(str::to_string))
                .unwrap_or_default();
            let title = string_field(&entry, "title")
                .or_else(|| entity.get("title").and_then(serde_json::Value::as_str).map(str::to_string))
                .unwrap_or_default();
            let risk_level = string_field(&entry, "risk_level")
                .or_else(|| entity.get("risk_level").and_then(serde_json::Value::as_str).map(str::to_string))
                .unwrap_or_default();
            let score = entry
                .get("distance")
                .and_then(serde_json::Value::as_f64)
                .or_else(|| entry.get("score").and_then(serde_json::Value::as_f64))
                .unwrap_or(0.0);

            Some(SimilarCaseHit {
                id,
                case_id,
                case_code,
                title,
                risk_level,
                score,
            })
        })
        .collect()
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key).and_then(serde_json::Value::as_str).map(str::to_string)
}

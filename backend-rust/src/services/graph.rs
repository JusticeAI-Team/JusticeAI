use reqwest::Client;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct GraphEntitySync {
    pub entity_type: String,
    pub entity_name: String,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct GraphRelationSync {
    pub relation_type: String,
    pub source_entity_name: String,
    pub target_entity_name: String,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct GraphCaseSyncInput {
    pub case_id: String,
    pub case_code: String,
    pub title: String,
    pub area_name: String,
    pub risk_level: String,
    pub source_type: String,
    pub entities: Vec<GraphEntitySync>,
    pub relations: Vec<GraphRelationSync>,
}

#[derive(Debug, Clone)]
pub struct GraphSyncResult {
    pub status: String,
    pub vertex_count: usize,
    pub edge_count: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct HugeGraphSyncService {
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

impl HugeGraphSyncService {
    pub fn new(
        client: Client,
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            username: username.into(),
            password: password.into(),
        }
    }

    pub async fn sync_case_graph(
        &self,
        input: &GraphCaseSyncInput,
    ) -> Result<GraphSyncResult, String> {
        if self.base_url.trim().is_empty() {
            return Ok(GraphSyncResult {
                status: "not_configured".to_string(),
                vertex_count: 0,
                edge_count: 0,
                message: "HugeGraph base URL is not configured".to_string(),
            });
        }

        self.ensure_schema().await?;
        self.drop_case_graph(input).await?;
        self.create_case_vertex(input).await?;

        for entity in &input.entities {
            self.create_entity_vertex(input, entity).await?;
            self.create_case_entity_edge(input, entity).await?;
        }

        for relation in &input.relations {
            self.create_entity_relation_edge(input, relation).await?;
        }

        Ok(GraphSyncResult {
            status: "synced".to_string(),
            vertex_count: 1 + input.entities.len(),
            edge_count: input.entities.len() + input.relations.len(),
            message: format!(
                "synced case {} into HugeGraph with {} vertices and {} edges",
                input.case_code,
                1 + input.entities.len(),
                input.entities.len() + input.relations.len()
            ),
        })
    }

    async fn ensure_schema(&self) -> Result<(), String> {
        for property in [
            json!({ "name": "case_id", "data_type": "TEXT", "cardinality": "SINGLE" }),
            json!({ "name": "case_code", "data_type": "TEXT", "cardinality": "SINGLE" }),
            json!({ "name": "title", "data_type": "TEXT", "cardinality": "SINGLE" }),
            json!({ "name": "area_name", "data_type": "TEXT", "cardinality": "SINGLE" }),
            json!({ "name": "risk_level", "data_type": "TEXT", "cardinality": "SINGLE" }),
            json!({ "name": "source_type", "data_type": "TEXT", "cardinality": "SINGLE" }),
            json!({ "name": "entity_type", "data_type": "TEXT", "cardinality": "SINGLE" }),
            json!({ "name": "entity_name", "data_type": "TEXT", "cardinality": "SINGLE" }),
            json!({ "name": "confidence", "data_type": "DOUBLE", "cardinality": "SINGLE" }),
            json!({ "name": "relation_type", "data_type": "TEXT", "cardinality": "SINGLE" }),
            json!({ "name": "updated_at", "data_type": "TEXT", "cardinality": "SINGLE" }),
        ] {
            let _ = self.post_schema("propertykeys", property).await;
        }

        let _ = self
            .post_schema(
                "vertexlabels",
                json!({
                    "name": "risk_case",
                    "id_strategy": "CUSTOMIZE_STRING",
                    "properties": ["case_id", "case_code", "title", "area_name", "risk_level", "source_type", "updated_at"],
                    "nullable_keys": ["area_name", "risk_level", "source_type", "updated_at"]
                }),
            )
            .await;

        let _ = self
            .post_schema(
                "vertexlabels",
                json!({
                    "name": "kg_entity",
                    "id_strategy": "CUSTOMIZE_STRING",
                    "properties": ["case_id", "entity_type", "entity_name", "confidence", "updated_at"],
                    "nullable_keys": ["confidence", "updated_at"]
                }),
            )
            .await;

        let _ = self
            .post_schema(
                "edgelabels",
                json!({
                    "name": "case_has_entity",
                    "source_label": "risk_case",
                    "target_label": "kg_entity",
                    "frequency": "SINGLE",
                    "properties": ["case_id", "relation_type", "confidence", "updated_at"],
                    "nullable_keys": ["confidence", "updated_at"]
                }),
            )
            .await;

        let _ = self
            .post_schema(
                "edgelabels",
                json!({
                    "name": "entity_related_to",
                    "source_label": "kg_entity",
                    "target_label": "kg_entity",
                    "frequency": "SINGLE",
                    "properties": ["case_id", "relation_type", "confidence", "updated_at"],
                    "nullable_keys": ["confidence", "updated_at"]
                }),
            )
            .await;

        Ok(())
    }

    async fn drop_case_graph(&self, input: &GraphCaseSyncInput) -> Result<(), String> {
        for entity in &input.entities {
            let _ = self
                .delete_graph_path("vertices", &entity_vertex_id(&input.case_id, &entity.entity_name))
                .await;
        }
        let _ = self
            .delete_graph_path("vertices", &case_vertex_id(&input.case_id))
            .await;
        Ok(())
    }

    async fn create_case_vertex(&self, input: &GraphCaseSyncInput) -> Result<(), String> {
        self.post_graph(
            "vertices",
            json!({
                "id": case_vertex_id(&input.case_id),
                "label": "risk_case",
                "properties": {
                    "case_id": input.case_id,
                    "case_code": input.case_code,
                    "title": input.title,
                    "area_name": input.area_name,
                    "risk_level": input.risk_level,
                    "source_type": input.source_type,
                    "updated_at": chrono::Utc::now().to_rfc3339()
                }
            }),
        )
        .await
        .map(|_| ())
    }

    async fn create_entity_vertex(
        &self,
        input: &GraphCaseSyncInput,
        entity: &GraphEntitySync,
    ) -> Result<(), String> {
        self.post_graph(
            "vertices",
            json!({
                "id": entity_vertex_id(&input.case_id, &entity.entity_name),
                "label": "kg_entity",
                "properties": {
                    "case_id": input.case_id,
                    "entity_type": entity.entity_type,
                    "entity_name": entity.entity_name,
                    "confidence": entity.confidence,
                    "updated_at": chrono::Utc::now().to_rfc3339()
                }
            }),
        )
        .await
        .map(|_| ())
    }

    async fn create_case_entity_edge(
        &self,
        input: &GraphCaseSyncInput,
        entity: &GraphEntitySync,
    ) -> Result<(), String> {
        self.post_graph(
            "edges",
            json!({
                "label": "case_has_entity",
                "outV": case_vertex_id(&input.case_id),
                "outVLabel": "risk_case",
                "inV": entity_vertex_id(&input.case_id, &entity.entity_name),
                "inVLabel": "kg_entity",
                "properties": {
                    "case_id": input.case_id,
                    "relation_type": "case_has_entity",
                    "confidence": entity.confidence,
                    "updated_at": chrono::Utc::now().to_rfc3339()
                }
            }),
        )
        .await
        .map(|_| ())
    }

    async fn create_entity_relation_edge(
        &self,
        input: &GraphCaseSyncInput,
        relation: &GraphRelationSync,
    ) -> Result<(), String> {
        self.post_graph(
            "edges",
            json!({
                "label": "entity_related_to",
                "outV": entity_vertex_id(&input.case_id, &relation.source_entity_name),
                "outVLabel": "kg_entity",
                "inV": entity_vertex_id(&input.case_id, &relation.target_entity_name),
                "inVLabel": "kg_entity",
                "properties": {
                    "case_id": input.case_id,
                    "relation_type": relation.relation_type,
                    "confidence": relation.confidence,
                    "updated_at": chrono::Utc::now().to_rfc3339()
                }
            }),
        )
        .await
        .map(|_| ())
    }

    async fn post_schema(&self, resource: &str, payload: Value) -> Result<Value, String> {
        self.request_json(
            reqwest::Method::POST,
            &format!("/graphs/hugegraph/schema/{resource}"),
            Some(payload),
        )
        .await
        .or_else(|error| {
            if error.contains("existed") || error.contains("already") || error.contains("Conflict") {
                Ok(json!({ "status": "exists" }))
            } else {
                Err(error)
            }
        })
    }

    async fn post_graph(&self, resource: &str, payload: Value) -> Result<Value, String> {
        self.request_json(
            reqwest::Method::POST,
            &format!("/graphs/hugegraph/graph/{resource}"),
            Some(payload),
        )
        .await
    }

    async fn delete_graph_path(&self, resource: &str, id: &str) -> Result<Value, String> {
        let encoded_id = urlencoding::encode(id);
        self.request_json(
            reqwest::Method::DELETE,
            &format!("/graphs/hugegraph/graph/{resource}/{encoded_id}"),
            None,
        )
        .await
    }

    async fn request_json(
        &self,
        method: reqwest::Method,
        path: &str,
        payload: Option<Value>,
    ) -> Result<Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self
            .client
            .request(method, url)
            .basic_auth(self.username.clone(), Some(self.password.clone()));
        if let Some(payload) = payload {
            request = request.json(&payload);
        }

        let response = request.send().await.map_err(|error| error.to_string())?;
        let status = response.status();
        let body = response.text().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("HugeGraph request failed with status {status}: {body}"));
        }
        serde_json::from_str(&body).map_err(|error| format!("invalid HugeGraph response: {error}: {body}"))
    }
}

fn case_vertex_id(case_id: &str) -> String {
    format!("case:{case_id}")
}

fn entity_vertex_id(case_id: &str, entity_name: &str) -> String {
    format!("entity:{}:{}", case_id, sanitize_key(entity_name))
}

fn sanitize_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_key_replaces_non_ascii_separator() {
        assert_eq!(sanitize_key("case A/1"), "case_A_1");
    }

    #[test]
    fn entity_vertex_key_is_stable() {
        assert_eq!(
            entity_vertex_id("case-1", "School Gate"),
            "entity:case-1:School_Gate"
        );
    }
}

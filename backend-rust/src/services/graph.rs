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
        let script = r#"
schema = graph.schema();
schema.propertyKey('vertex_key').asText().ifNotExist().create();
schema.propertyKey('case_id').asText().ifNotExist().create();
schema.propertyKey('case_code').asText().ifNotExist().create();
schema.propertyKey('title').asText().ifNotExist().create();
schema.propertyKey('area_name').asText().ifNotExist().create();
schema.propertyKey('risk_level').asText().ifNotExist().create();
schema.propertyKey('source_type').asText().ifNotExist().create();
schema.propertyKey('entity_type').asText().ifNotExist().create();
schema.propertyKey('entity_name').asText().ifNotExist().create();
schema.propertyKey('confidence').asDouble().ifNotExist().create();
schema.propertyKey('relation_type').asText().ifNotExist().create();
schema.propertyKey('updated_at').asText().ifNotExist().create();
schema.vertexLabel('risk_case')
    .properties('vertex_key', 'case_id', 'case_code', 'title', 'area_name', 'risk_level', 'source_type', 'updated_at')
    .primaryKeys('vertex_key')
    .nullableKeys('area_name', 'risk_level', 'source_type', 'updated_at')
    .ifNotExist()
    .create();
schema.vertexLabel('kg_entity')
    .properties('vertex_key', 'case_id', 'entity_type', 'entity_name', 'confidence', 'updated_at')
    .primaryKeys('vertex_key')
    .nullableKeys('confidence', 'updated_at')
    .ifNotExist()
    .create();
schema.edgeLabel('case_has_entity')
    .sourceLabel('risk_case')
    .targetLabel('kg_entity')
    .properties('case_id', 'relation_type', 'confidence', 'updated_at')
    .nullableKeys('confidence', 'updated_at')
    .ifNotExist()
    .create();
schema.edgeLabel('entity_related_to')
    .sourceLabel('kg_entity')
    .targetLabel('kg_entity')
    .properties('case_id', 'relation_type', 'confidence', 'updated_at')
    .nullableKeys('confidence', 'updated_at')
    .ifNotExist()
    .create();
        "#;
        self.run_gremlin(script).await.map(|_| ())
    }

    async fn drop_case_graph(&self, input: &GraphCaseSyncInput) -> Result<(), String> {
        let script = format!(
            "g.V().has('kg_entity', 'case_id', {case_id}).drop().iterate(); \
             g.V().has('risk_case', 'vertex_key', {case_key}).drop().iterate();",
            case_id = quoted(&input.case_id),
            case_key = quoted(&case_vertex_key(&input.case_id)),
        );
        self.run_gremlin(&script).await.map(|_| ())
    }

    async fn create_case_vertex(&self, input: &GraphCaseSyncInput) -> Result<(), String> {
        let script = format!(
            "g.addV('risk_case')\
                .property('vertex_key', {vertex_key})\
                .property('case_id', {case_id})\
                .property('case_code', {case_code})\
                .property('title', {title})\
                .property('area_name', {area_name})\
                .property('risk_level', {risk_level})\
                .property('source_type', {source_type})\
                .property('updated_at', {updated_at})\
                .iterate();",
            vertex_key = quoted(&case_vertex_key(&input.case_id)),
            case_id = quoted(&input.case_id),
            case_code = quoted(&input.case_code),
            title = quoted(&input.title),
            area_name = quoted(&input.area_name),
            risk_level = quoted(&input.risk_level),
            source_type = quoted(&input.source_type),
            updated_at = quoted(&chrono::Utc::now().to_rfc3339()),
        );
        self.run_gremlin(&script).await.map(|_| ())
    }

    async fn create_entity_vertex(
        &self,
        input: &GraphCaseSyncInput,
        entity: &GraphEntitySync,
    ) -> Result<(), String> {
        let script = format!(
            "g.addV('kg_entity')\
                .property('vertex_key', {vertex_key})\
                .property('case_id', {case_id})\
                .property('entity_type', {entity_type})\
                .property('entity_name', {entity_name})\
                .property('confidence', {confidence})\
                .property('updated_at', {updated_at})\
                .iterate();",
            vertex_key = quoted(&entity_vertex_key(&input.case_id, &entity.entity_name)),
            case_id = quoted(&input.case_id),
            entity_type = quoted(&entity.entity_type),
            entity_name = quoted(&entity.entity_name),
            confidence = entity.confidence,
            updated_at = quoted(&chrono::Utc::now().to_rfc3339()),
        );
        self.run_gremlin(&script).await.map(|_| ())
    }

    async fn create_case_entity_edge(
        &self,
        input: &GraphCaseSyncInput,
        entity: &GraphEntitySync,
    ) -> Result<(), String> {
        let script = format!(
            "g.V().has('risk_case', 'vertex_key', {case_key}).as('c')\
                .V().has('kg_entity', 'vertex_key', {entity_key})\
                .addE('case_has_entity').from('c')\
                .property('case_id', {case_id})\
                .property('relation_type', 'case_has_entity')\
                .property('confidence', {confidence})\
                .property('updated_at', {updated_at})\
                .iterate();",
            case_key = quoted(&case_vertex_key(&input.case_id)),
            entity_key = quoted(&entity_vertex_key(&input.case_id, &entity.entity_name)),
            case_id = quoted(&input.case_id),
            confidence = entity.confidence,
            updated_at = quoted(&chrono::Utc::now().to_rfc3339()),
        );
        self.run_gremlin(&script).await.map(|_| ())
    }

    async fn create_entity_relation_edge(
        &self,
        input: &GraphCaseSyncInput,
        relation: &GraphRelationSync,
    ) -> Result<(), String> {
        let script = format!(
            "g.V().has('kg_entity', 'vertex_key', {source_key}).as('s')\
                .V().has('kg_entity', 'vertex_key', {target_key})\
                .addE('entity_related_to').from('s')\
                .property('case_id', {case_id})\
                .property('relation_type', {relation_type})\
                .property('confidence', {confidence})\
                .property('updated_at', {updated_at})\
                .iterate();",
            source_key = quoted(&entity_vertex_key(&input.case_id, &relation.source_entity_name)),
            target_key = quoted(&entity_vertex_key(&input.case_id, &relation.target_entity_name)),
            case_id = quoted(&input.case_id),
            relation_type = quoted(&relation.relation_type),
            confidence = relation.confidence,
            updated_at = quoted(&chrono::Utc::now().to_rfc3339()),
        );
        self.run_gremlin(&script).await.map(|_| ())
    }

    async fn run_gremlin(&self, script: &str) -> Result<Value, String> {
        let url = format!("{}/gremlin", self.base_url);
        let request = self
            .client
            .post(url)
            .basic_auth(self.username.clone(), Some(self.password.clone()))
            .json(&json!({
                "gremlin": script,
                "language": "gremlin-groovy",
                "bindings": {}
            }));

        let response = request.send().await.map_err(|error| error.to_string())?;
        let status = response.status();
        let body = response.text().await.map_err(|error| error.to_string())?;
        if !status.is_success() {
            return Err(format!("HugeGraph request failed with status {status}: {body}"));
        }

        serde_json::from_str(&body).map_err(|error| format!("invalid HugeGraph response: {error}: {body}"))
    }
}

fn case_vertex_key(case_id: &str) -> String {
    format!("case:{case_id}")
}

fn entity_vertex_key(case_id: &str, entity_name: &str) -> String {
    format!(
        "entity:{}:{}",
        case_id,
        sanitize_key(entity_name)
    )
}

fn sanitize_key(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn quoted(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

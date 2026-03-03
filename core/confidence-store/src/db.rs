use anidb_shared_types::confidence::ConfidenceScore;
use anidb_shared_types::FactRecord;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ConfidenceStoreError;

#[derive(Debug, sqlx::FromRow)]
struct FactRow {
    id: Uuid,
    entity_id: Uuid,
    entity_type: String,
    fact_key: String,
    fact_value: serde_json::Value,
    confidence_value: f64,
    confidence_source: String,
    confidence_last_verified: DateTime<Utc>,
    confidence_derivation: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl FactRow {
    fn into_fact_record(self) -> Result<FactRecord, ConfidenceStoreError> {
        let derivation: Vec<Uuid> =
            serde_json::from_value(self.confidence_derivation)?;
        Ok(FactRecord {
            id: self.id,
            entity_id: self.entity_id,
            entity_type: self.entity_type,
            fact_key: self.fact_key,
            fact_value: self.fact_value,
            confidence: ConfidenceScore {
                value: self.confidence_value,
                source: self.confidence_source,
                last_verified: self.confidence_last_verified,
                derivation,
            },
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

pub struct FactRepository {
    pool: PgPool,
}

impl FactRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        entity_id: Uuid,
        entity_type: &str,
        fact_key: &str,
        fact_value: &serde_json::Value,
        confidence_value: f64,
        confidence_source: &str,
        derivation: &[Uuid],
    ) -> Result<FactRecord, ConfidenceStoreError> {
        let derivation_json = serde_json::to_value(derivation)?;
        let row = sqlx::query_as::<_, FactRow>(
            r#"
            INSERT INTO confidence.facts
                (entity_id, entity_type, fact_key, fact_value,
                 confidence_value, confidence_source, confidence_derivation)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(entity_id)
        .bind(entity_type)
        .bind(fact_key)
        .bind(fact_value)
        .bind(confidence_value)
        .bind(confidence_source)
        .bind(&derivation_json)
        .fetch_one(&self.pool)
        .await?;

        row.into_fact_record()
    }

    pub async fn upsert(
        &self,
        entity_id: Uuid,
        entity_type: &str,
        fact_key: &str,
        fact_value: &serde_json::Value,
        confidence_value: f64,
        confidence_source: &str,
        derivation: &[Uuid],
    ) -> Result<FactRecord, ConfidenceStoreError> {
        let derivation_json = serde_json::to_value(derivation)?;
        let row = sqlx::query_as::<_, FactRow>(
            r#"
            INSERT INTO confidence.facts
                (entity_id, entity_type, fact_key, fact_value,
                 confidence_value, confidence_source, confidence_derivation)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (entity_id, fact_key) DO UPDATE SET
                fact_value = EXCLUDED.fact_value,
                confidence_value = EXCLUDED.confidence_value,
                confidence_source = EXCLUDED.confidence_source,
                confidence_derivation = EXCLUDED.confidence_derivation,
                confidence_last_verified = NOW(),
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(entity_id)
        .bind(entity_type)
        .bind(fact_key)
        .bind(fact_value)
        .bind(confidence_value)
        .bind(confidence_source)
        .bind(&derivation_json)
        .fetch_one(&self.pool)
        .await?;

        row.into_fact_record()
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<FactRecord, ConfidenceStoreError> {
        let row = sqlx::query_as::<_, FactRow>(
            "SELECT * FROM confidence.facts WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ConfidenceStoreError::NotFound(id.to_string()))?;

        row.into_fact_record()
    }

    pub async fn get_fact(
        &self,
        entity_id: Uuid,
        fact_key: &str,
    ) -> Result<FactRecord, ConfidenceStoreError> {
        let row = sqlx::query_as::<_, FactRow>(
            "SELECT * FROM confidence.facts WHERE entity_id = $1 AND fact_key = $2",
        )
        .bind(entity_id)
        .bind(fact_key)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            ConfidenceStoreError::NotFound(format!("{}/{}", entity_id, fact_key))
        })?;

        row.into_fact_record()
    }

    pub async fn get_for_entity(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<FactRecord>, ConfidenceStoreError> {
        let rows = sqlx::query_as::<_, FactRow>(
            "SELECT * FROM confidence.facts WHERE entity_id = $1 ORDER BY fact_key",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_fact_record()).collect()
    }

    pub async fn get_by_type(
        &self,
        entity_type: &str,
    ) -> Result<Vec<FactRecord>, ConfidenceStoreError> {
        let rows = sqlx::query_as::<_, FactRow>(
            "SELECT * FROM confidence.facts WHERE entity_type = $1 ORDER BY entity_id, fact_key",
        )
        .bind(entity_type)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_fact_record()).collect()
    }

    pub async fn get_bulk(
        &self,
        entity_ids: &[Uuid],
    ) -> Result<Vec<FactRecord>, ConfidenceStoreError> {
        let rows = sqlx::query_as::<_, FactRow>(
            "SELECT * FROM confidence.facts WHERE entity_id = ANY($1) ORDER BY entity_id, fact_key",
        )
        .bind(entity_ids)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_fact_record()).collect()
    }

    pub async fn update_confidence(
        &self,
        fact_id: Uuid,
        confidence_value: f64,
        confidence_source: &str,
        derivation: &[Uuid],
    ) -> Result<FactRecord, ConfidenceStoreError> {
        let derivation_json = serde_json::to_value(derivation)?;
        let row = sqlx::query_as::<_, FactRow>(
            r#"
            UPDATE confidence.facts SET
                confidence_value = $2,
                confidence_source = $3,
                confidence_derivation = $4,
                confidence_last_verified = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(fact_id)
        .bind(confidence_value)
        .bind(confidence_source)
        .bind(&derivation_json)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ConfidenceStoreError::NotFound(fact_id.to_string()))?;

        row.into_fact_record()
    }
}

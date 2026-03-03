CREATE TABLE IF NOT EXISTS confidence.facts (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id               UUID NOT NULL,
    entity_type             TEXT NOT NULL,
    fact_key                TEXT NOT NULL,
    fact_value              JSONB NOT NULL,
    confidence_value        DOUBLE PRECISION NOT NULL CHECK (confidence_value >= 0.0 AND confidence_value <= 1.0),
    confidence_source       TEXT NOT NULL,
    confidence_last_verified TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confidence_derivation   JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (entity_id, fact_key)
);

CREATE INDEX idx_facts_entity_id ON confidence.facts (entity_id);
CREATE INDEX idx_facts_entity_type ON confidence.facts (entity_type);
CREATE INDEX idx_facts_fact_key ON confidence.facts (fact_key);
CREATE INDEX idx_facts_confidence_value ON confidence.facts (confidence_value);

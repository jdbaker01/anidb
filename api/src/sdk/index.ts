// ANIDB Agent SDK
// TypeScript client for agents to interact with the ANIDB API.
// Implemented in Phase 6.

export interface IntentQuery {
  intent: string;
  context: {
    decision_class?: "churn" | "pricing" | "capacity";
    entity_refs?: string[];
    time_horizon?: string;
    min_confidence?: number;
  };
}

export interface ContextBundle {
  decision_class: string;
  facts: Array<{
    key: string;
    value: unknown;
    confidence: number;
    source: string;
    last_verified: string;
    derivation: string[];
  }>;
  causal_context: string;
  suggested_queries: string[];
}

use neo4rs::{ConfigBuilder, Graph, Query, query};

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Neo4j connection error: {0}")]
    Connection(String),

    #[error("Neo4j query error: {0}")]
    Query(String),

    #[error("Neo4j deserialization error: {0}")]
    Deserialization(String),
}

impl From<neo4rs::Error> for GraphError {
    fn from(e: neo4rs::Error) -> Self {
        GraphError::Query(e.to_string())
    }
}

impl From<neo4rs::DeError> for GraphError {
    fn from(e: neo4rs::DeError) -> Self {
        GraphError::Deserialization(e.to_string())
    }
}

#[derive(Clone)]
pub struct GraphClient {
    graph: Graph,
}

impl GraphClient {
    pub async fn new(uri: &str, user: &str, password: &str) -> Result<Self, GraphError> {
        let config = ConfigBuilder::default()
            .uri(uri)
            .user(user)
            .password(password)
            .db("neo4j")
            .max_connections(10)
            .build()
            .map_err(|e| GraphError::Connection(format!("{:?}", e)))?;

        let graph = Graph::connect(config)
            .await
            .map_err(|e| GraphError::Connection(e.to_string()))?;

        Ok(Self { graph })
    }

    /// Run a query that does not return results (CREATE, MERGE, etc.).
    pub async fn run(&self, q: Query) -> Result<(), GraphError> {
        self.graph.run(q).await?;
        Ok(())
    }

    /// Execute a query and collect all rows as JSON values.
    pub async fn execute_collect(
        &self,
        q: Query,
    ) -> Result<Vec<neo4rs::Row>, GraphError> {
        let mut stream = self.graph.execute(q).await?;
        let mut rows = Vec::new();
        while let Some(row) = stream.next().await? {
            rows.push(row);
        }
        Ok(rows)
    }

    /// Run multiple Cypher statements sequentially.
    pub async fn run_all(&self, queries: Vec<Query>) -> Result<(), GraphError> {
        for q in queries {
            self.graph.run(q).await?;
        }
        Ok(())
    }

    /// Health check: verify connectivity.
    pub async fn health_check(&self) -> Result<bool, GraphError> {
        let mut stream = self.graph.execute(query("RETURN 1 AS n")).await?;
        if let Some(row) = stream.next().await? {
            let n: i64 = row.get("n")?;
            return Ok(n == 1);
        }
        Ok(false)
    }
}

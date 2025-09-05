use neo4rs::{Graph, ConfigBuilder, Query};
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use thiserror::Error;
use crate::config::MemgraphPerformanceConfig;

/// Connection pool errors
#[derive(Error, Debug)]
pub enum PoolError {
    #[error("Connection pool exhausted")]
    Exhausted,
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),
    
    #[error("Pool is closed")]
    PoolClosed,
}

/// Health status of a connection
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionHealth {
    Healthy,
    Degraded,
    Failed,
}

/// Connection wrapper with health tracking
struct PooledConnection {
    graph: Graph,
    created_at: Instant,
    last_used: Instant,
    health: ConnectionHealth,
    consecutive_failures: u32,
}

impl PooledConnection {
    fn new(graph: Graph) -> Self {
        let now = Instant::now();
        Self {
            graph,
            created_at: now,
            last_used: now,
            health: ConnectionHealth::Healthy,
            consecutive_failures: 0,
        }
    }

    fn mark_used(&mut self) {
        self.last_used = Instant::now();
    }

    fn mark_failure(&mut self) {
        self.consecutive_failures += 1;
        self.health = if self.consecutive_failures > 3 {
            ConnectionHealth::Failed
        } else {
            ConnectionHealth::Degraded
        };
    }

    fn mark_success(&mut self) {
        self.consecutive_failures = 0;
        self.health = ConnectionHealth::Healthy;
    }

    fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed() > max_age
    }

    fn is_idle(&self, max_idle: Duration) -> bool {
        self.last_used.elapsed() > max_idle
    }

    async fn health_check(&mut self) -> Result<(), PoolError> {
        match self.graph.execute(Query::new("RETURN 1 as health".to_string())).await {
            Ok(_) => {
                self.mark_success();
                Ok(())
            }
            Err(e) => {
                self.mark_failure();
                Err(PoolError::HealthCheckFailed(e.to_string()))
            }
        }
    }
}

/// High-performance connection pool for Memgraph
#[derive(Clone)]
pub struct ConnectionPool {
    connections: Arc<RwLock<Vec<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
    config: ConnectionPoolConfig,
    uri: String,
    username: String,
    password: String,
    is_closed: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    pub min_connections: u32,
    pub max_connections: u32,
    pub connection_timeout: Duration,
    pub query_timeout: Duration,
    pub max_connection_age: Duration,
    pub max_idle_time: Duration,
    pub health_check_interval: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            connection_timeout: Duration::from_secs(5),
            query_timeout: Duration::from_secs(30),
            max_connection_age: Duration::from_secs(3600), // 1 hour
            max_idle_time: Duration::from_secs(300),       // 5 minutes
            health_check_interval: Duration::from_secs(30),
        }
    }
}

impl From<&MemgraphPerformanceConfig> for ConnectionPoolConfig {
    fn from(perf_config: &MemgraphPerformanceConfig) -> Self {
        Self {
            min_connections: 2.min(perf_config.connection_pool_size),
            max_connections: perf_config.connection_pool_size,
            connection_timeout: Duration::from_millis(perf_config.connection_timeout_ms),
            query_timeout: Duration::from_millis(perf_config.query_timeout_ms),
            max_connection_age: Duration::from_secs(3600),
            max_idle_time: Duration::from_secs(300),
            health_check_interval: Duration::from_secs(30),
        }
    }
}

impl ConnectionPool {
    /// Create a new connection pool
    pub async fn new(
        uri: &str,
        username: &str,
        password: &str,
        config: ConnectionPoolConfig,
    ) -> Result<Self> {
        let pool = Self {
            connections: Arc::new(RwLock::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(config.max_connections as usize)),
            config: config.clone(),
            uri: uri.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            is_closed: Arc::new(RwLock::new(false)),
        };

        // Initialize minimum connections
        pool.ensure_min_connections().await?;

        // Start background health check task
        pool.start_health_check_task().await;

        Ok(pool)
    }

    /// Create connection pool from performance config
    pub async fn from_performance_config(
        uri: &str,
        username: &str,
        password: &str,
        perf_config: &MemgraphPerformanceConfig,
    ) -> Result<Self> {
        let pool_config = ConnectionPoolConfig::from(perf_config);
        Self::new(uri, username, password, pool_config).await
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<PooledGraph> {
        // Check if pool is closed
        if *self.is_closed.read().await {
            return Err(PoolError::PoolClosed.into());
        }

        // Acquire semaphore permit
        let permit = self.semaphore.clone()
            .acquire_owned()
            .await
            .map_err(|_| PoolError::Exhausted)?;

        // Try to get existing healthy connection
        if let Some(mut conn) = self.pop_healthy_connection().await {
            conn.mark_used();
            return Ok(PooledGraph {
                connection: Some(conn),
                pool: self.connections.clone(),
                _permit: permit,
            });
        }

        // Create new connection if none available
        match self.create_connection().await {
            Ok(conn) => Ok(PooledGraph {
                connection: Some(conn),
                pool: self.connections.clone(),
                _permit: permit,
            }),
            Err(e) => {
                eprintln!("Failed to create connection: {}", e);
                Err(PoolError::ConnectionFailed(e.to_string()).into())
            }
        }
    }

    /// Create a new connection
    async fn create_connection(&self) -> Result<PooledConnection> {
        let graph_config = ConfigBuilder::default()
            .uri(&self.uri)
            .user(&self.username)
            .password(&self.password)
            .db("memgraph")
            .build()?;

        let graph = tokio::time::timeout(
            self.config.connection_timeout,
            Graph::connect(graph_config)
        ).await
        .map_err(|_| PoolError::ConnectionFailed("Connection timeout".to_string()))??;

        let mut connection = PooledConnection::new(graph);

        // Perform initial health check
        connection.health_check().await?;

        Ok(connection)
    }

    /// Get a healthy connection from the pool
    async fn pop_healthy_connection(&self) -> Option<PooledConnection> {
        let mut connections = self.connections.write().await;
        
        // Find and remove first healthy connection
        for i in 0..connections.len() {
            if connections[i].health == ConnectionHealth::Healthy {
                return Some(connections.remove(i));
            }
        }

        None
    }

    /// Return a connection to the pool
    async fn return_connection(&self, connection: PooledConnection) {
        let mut connections = self.connections.write().await;
        
        // Check if connection should be retained
        if !connection.is_expired(self.config.max_connection_age) 
            && connection.health != ConnectionHealth::Failed {
            connections.push(connection);
        }
        // Expired or failed connections are dropped
    }

    /// Ensure minimum connections are maintained
    async fn ensure_min_connections(&self) -> Result<()> {
        let current_count = self.connections.read().await.len();
        
        if (current_count as u32) < self.config.min_connections {
            let needed = self.config.min_connections - current_count as u32;
            
            for _ in 0..needed {
                match self.create_connection().await {
                    Ok(conn) => {
                        self.connections.write().await.push(conn);
                    }
                    Err(e) => {
                        eprintln!("Failed to create minimum connection: {}", e);
                        // Continue trying to create other connections
                    }
                }
            }
        }

        Ok(())
    }

    /// Start background health check task
    async fn start_health_check_task(&self) {
        let connections = self.connections.clone();
        let interval = self.config.health_check_interval;
        let max_idle = self.config.max_idle_time;
        let max_age = self.config.max_connection_age;
        let is_closed = self.is_closed.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;

                // Check if pool is closed
                if *is_closed.read().await {
                    break;
                }

                let mut connections_guard = connections.write().await;
                let mut healthy_connections = Vec::new();

                for mut conn in connections_guard.drain(..) {
                    // Remove expired or idle connections
                    if conn.is_expired(max_age) || conn.is_idle(max_idle) {
                        continue; // Connection dropped
                    }

                    // Perform health check
                    if conn.health_check().await.is_ok() {
                        healthy_connections.push(conn);
                    }
                    // Failed connections are dropped
                }

                *connections_guard = healthy_connections;
            }
        });
    }

    /// Get pool statistics
    pub async fn stats(&self) -> ConnectionPoolStats {
        let connections = self.connections.read().await;
        let healthy = connections.iter()
            .filter(|c| c.health == ConnectionHealth::Healthy)
            .count();
        let degraded = connections.iter()
            .filter(|c| c.health == ConnectionHealth::Degraded)
            .count();
        let failed = connections.iter()
            .filter(|c| c.health == ConnectionHealth::Failed)
            .count();

        ConnectionPoolStats {
            total_connections: connections.len(),
            healthy_connections: healthy,
            degraded_connections: degraded,
            failed_connections: failed,
            available_permits: self.semaphore.available_permits(),
            max_connections: self.config.max_connections,
        }
    }

    /// Close the connection pool
    pub async fn close(&self) {
        *self.is_closed.write().await = true;
        self.connections.write().await.clear();
    }
}

/// Statistics about the connection pool
#[derive(Debug)]
pub struct ConnectionPoolStats {
    pub total_connections: usize,
    pub healthy_connections: usize,
    pub degraded_connections: usize,
    pub failed_connections: usize,
    pub available_permits: usize,
    pub max_connections: u32,
}

/// A pooled graph connection that returns to pool when dropped
pub struct PooledGraph {
    connection: Option<PooledConnection>,
    pool: Arc<RwLock<Vec<PooledConnection>>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl PooledGraph {
    /// Run a query on this connection (no result stream)
    pub async fn run(&mut self, query: Query) -> Result<()> {
        if let Some(ref mut conn) = self.connection {
            conn.mark_used();
            match conn.graph.run(query).await {
                Ok(_) => {
                    conn.mark_success();
                    Ok(())
                }
                Err(e) => {
                    conn.mark_failure();
                    Err(e.into())
                }
            }
        } else {
            Err(PoolError::PoolClosed.into())
        }
    }

    /// Start a transaction on this connection
    pub async fn start_txn(&mut self) -> Result<neo4rs::Txn> {
        if let Some(ref mut conn) = self.connection {
            conn.mark_used();
            match conn.graph.start_txn().await {
                Ok(txn) => {
                    conn.mark_success();
                    Ok(txn)
                }
                Err(e) => {
                    conn.mark_failure();
                    Err(e.into())
                }
            }
        } else {
            Err(PoolError::PoolClosed.into())
        }
    }
}

impl Drop for PooledGraph {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            let pool = self.pool.clone();
            
            // Return connection to pool in background
            tokio::spawn(async move {
                let mut connections = pool.write().await;
                connections.push(connection);
            });
        }
    }
}
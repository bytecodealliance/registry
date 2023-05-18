//! # Monitoring
//!
//! The `monitoring` mod is a collection of utilities to set up and monitoring use cases such as:
//!
//! * health checks, e.g., `/livez` (server healthy but not serving), `/readyz` (server healthy and
//!   serving)
//! * shutdown grace period, i.e., time needed for load balancer to recognize need to pull an
//!   instance from a service pool when `/readyz` returns error

use std::sync::Arc;

use anyhow::Result;
use axum::{body::Body, extract::State, response::IntoResponse, routing::get, Router};
use clap::ValueEnum;
use reqwest::StatusCode;
use tokio::sync::{broadcast::Sender, Mutex};

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitoringKind {
    HealthChecks,
    // TODO: Support metrics via at least one of OpenTelemetry Metrics, Prometheus, etc.
    // Metrics,
    // TODO: Support tracing via at least one of OpenTelemetry Tracing, Jaeger, etc.
    // Tracing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Stage {
    NotLive,
    Live,
    Ready,
    ShuttingDown,
    Terminating,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    ShuttingDown,
    Terminating,
}

struct LifecycleState {
    stage: Stage,
}

struct Lifecycle {
    tx: Sender<Event>,
    state: Mutex<LifecycleState>,
}

pub struct LifecycleManager {
    lifecycle: Arc<Lifecycle>,
    shutdown_grace_period: Option<std::time::Duration>,
}

pub struct Config {
    pub shutdown_grace_period: Option<std::time::Duration>,
}

impl LifecycleManager {
    pub fn new(config: Config) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel::<Event>(1);
        LifecycleManager {
            lifecycle: Arc::new(Lifecycle {
                tx,
                state: Mutex::new(LifecycleState {
                    stage: Stage::NotLive,
                }),
            }),
            shutdown_grace_period: config.shutdown_grace_period,
        }
    }

    pub fn has_graceful_shutdown(&self) -> bool {
        self.shutdown_grace_period.is_some()
    }

    #[allow(dead_code)]
    pub async fn set_live(&self) -> Result<()> {
        let mut state = self.lifecycle.state.lock().await;
        match state.stage {
            Stage::ShuttingDown => Ok(()),
            _ => {
                state.stage = Stage::Live;
                Ok(())
            }
        }
    }

    pub async fn set_ready(&self) -> Result<()> {
        let mut state = self.lifecycle.state.lock().await;
        match state.stage {
            Stage::ShuttingDown => Ok(()),
            _ => {
                state.stage = Stage::Ready;
                Ok(())
            }
        }
    }

    /// Initiates the lifecycle shutdown sequence.
    ///
    /// A `ShuttingDown` event will be sent followed by `Terminating` after the optionally
    /// configured shutdown grace period.
    pub async fn shutdown(&self) -> Result<()> {
        let mut state = self.lifecycle.state.lock().await;
        match state.stage {
            Stage::ShuttingDown => Ok(()),
            _ => {
                tracing::debug!("shutting down");
                state.stage = Stage::ShuttingDown;
                self.lifecycle.tx.send(Event::ShuttingDown).map(|_| ())?;

                if let Some(shutdown_grace_period) = self.shutdown_grace_period {
                    tracing::info!(
                        "shutting down with grace period {:?}",
                        shutdown_grace_period
                    );
                    let lifecycle = self.lifecycle.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(shutdown_grace_period).await;
                        let mut state = lifecycle.state.lock().await;
                        tracing::info!("terminating");
                        state.stage = Stage::Terminating;
                        lifecycle.tx.send(Event::Terminating).unwrap();
                    });
                } else {
                    tracing::info!("shutting down without grace period");
                    tracing::info!("terminating");
                    state.stage = Stage::Terminating;
                    self.lifecycle.tx.send(Event::Terminating).map(|_| ())?;
                }

                Ok(())
            }
        }
    }

    /// Completes when services should immediately drain clients.
    pub async fn drain_signal(&self) {
        let event = if self.has_graceful_shutdown() {
            Event::Terminating
        } else {
            Event::ShuttingDown
        };
        self.signal(event).await;
    }

    /// Completes when shutdown event occurs or the lifecycle broadcast channel is closed.
    pub async fn shutdown_signal(&self) {
        self.signal(Event::ShuttingDown).await
    }

    /// Completes when termination event occurs or the lifecycle broadcast channel is closed.
    pub async fn terminate_signal(&self) {
        self.signal(Event::Terminating).await
    }

    async fn signal(&self, event: Event) {
        let mut server_rx = self.lifecycle.tx.subscribe();
        loop {
            match server_rx.recv().await {
                Ok(e) => match e {
                    e if e == event => return,
                    _ => continue,
                },
                Err(s) => match s {
                    tokio::sync::broadcast::error::RecvError::Closed => return,
                    tokio::sync::broadcast::error::RecvError::Lagged(_) => continue,
                },
            }
        }
    }

    pub fn health_checks_router(&self) -> Router<(), Body> {
        axum::Router::new()
            .route("/livez", get(livez))
            .route("/readyz", get(readyz))
            .with_state(self.lifecycle.clone())
    }
}

async fn livez(State(lifecycle): State<Arc<Lifecycle>>) -> impl IntoResponse {
    // TODO: Support a verbose option for human-readable details.
    let state = lifecycle.state.lock().await;
    match state.stage {
        Stage::Live | Stage::Ready => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn readyz(State(lifecycle): State<Arc<Lifecycle>>) -> impl IntoResponse {
    // TODO: Support a verbose option for human-readable details.
    let state = lifecycle.state.lock().await;
    match state.stage {
        Stage::Ready => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

//! # Requiem Backend Library
//!
//! مكتبة تعرّض وحدات المشروع للاختبارات التكاملية (tests/).
//! الوحدة البرمجية الفعلية (الخادم) تبقى في src/main.rs (bin target).

pub mod agent;
pub mod auth;
pub mod collaborative_agents;
pub mod crypto;
pub mod db;
pub mod db_pool;
pub mod enforce;
pub mod error;
pub mod formats;
pub mod llm_stream;
pub mod metrics;
pub mod migrate;
pub mod models;
pub mod orchestrator;
pub mod path_safety;
pub mod plugins;
pub mod rate_limit;
pub mod react_loop;
pub mod routes;
pub mod sandbox;
pub mod self_improvement;
pub mod storage;
pub mod tools;
pub mod webhooks;

pub use db::AppState;

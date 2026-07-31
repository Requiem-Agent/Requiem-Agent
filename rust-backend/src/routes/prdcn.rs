use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;

pub async fn prdcn_health(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let client = reqwest::Client::new();
    let prdcn_url = format!(
        "https://{}.hf.space/health",
        state.hf_space_prdcn.replace('/', "-")
    );
    let resp = client
        .get(&prdcn_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;
    match resp {
        Ok(r) => {
            let body: Value = r.json().await.unwrap_or(json!({"status":"unknown"}));
            Ok(Json(body))
        }
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            Json(json!({"error": format!("Prdcn unreachable: {e}")})),
        )),
    }
}

pub async fn deploy_bot_to_prdcn(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let prdcn_url = format!(
        "https://{}.hf.space/deploy",
        state.hf_space_prdcn.replace('/', "-")
    );
    let client = reqwest::Client::new();
    let resp = client
        .post(&prdcn_url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await;
    match resp {
        Ok(r) => {
            let body: Value = r.json().await.unwrap_or(json!({"status":"unknown"}));
            Ok(Json(body))
        }
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            Json(json!({"error": format!("Deploy failed: {e}")})),
        )),
    }
}

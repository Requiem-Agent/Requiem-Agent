//! # Model Registry
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    pub id: &'static str, pub name: &'static str,
    pub roles: Vec<&'static str>, pub vision: bool,
    pub speed: u8, pub reliability: f32,
    pub max_tokens: u32, pub cost_weight: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelHealth {
    pub model_id: String, pub available: bool,
    pub last_check: String, pub latency_ms: u64,
    pub success_rate: f32, pub total_calls: u64,
    pub failed_calls: u64, pub avg_tokens_per_call: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelSelection {
    pub model_id: String, pub reason: String,
    pub fallback_chain: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ModelEvent { Online, Offline, Slow{latency_ms:u64}, Error{error:String}, Success{latency_ms:u64,tokens:u32} }

// backward compat
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelInfo {
    pub id: String, pub name: String,
    #[serde(rename = "assignedRole")] pub assigned_role: String,
    #[serde(rename = "contextWindow")] pub context_window: Option<i64>,
    pub available: bool,
}
pub fn assign_role(mid: &str, mname: &str) -> &'static str {
    let l = format!("{} {}", mid, mname).to_lowercase();
    if l.contains("deepseek")||l.contains("code")||l.contains("bigpickle"){"coder"}
    else if l.contains("mimo")||l.contains("orchestrat"){"orchestrator"}
    else if l.contains("hy3")||l.contains("reasoning")||l.contains("planner")||l.contains("o3"){"planner"}
    else if l.contains("north")||l.contains("review")||l.contains("audit"){"reviewer"}
    else if l.contains("nometron")||l.contains("debug")||l.contains("large"){"debugger"}
    else if l.contains("design")||l.contains("vision"){"designer"}
    else if l.contains("research")||l.contains("web")||l.contains("search"){"researcher"}
    else if l.contains("secur")||l.contains("cyber"){"security"}else{"general"}
}

pub struct ModelRegistry {
    capabilities: Vec<ModelCapability>,
    health: RwLock<HashMap<String, ModelHealth>>,
    stats: RwLock<HashMap<String, ModelStats>>,
    last_probe: AtomicU64,
}

#[derive(Debug, Clone)]
struct ModelStats{total_calls:u64,failed_calls:u64,total_latency_ms:u64,total_tokens:u64,last_latency_ms:u64}

impl ModelRegistry {
    pub fn new() -> Arc<Self> {
        let capabilities = vec![
            ModelCapability{id:"deepseek-v4-flash-free",name:"DeepSeek V4 Flash",roles:vec!["code","general","debug","explore","security"],vision:false,speed:9,reliability:0.85,max_tokens:32768,cost_weight:1},
            ModelCapability{id:"big-pickle",name:"Big Pickle",roles:vec!["multi-file","code","review"],vision:false,speed:7,reliability:0.80,max_tokens:65536,cost_weight:2},
            ModelCapability{id:"mimo-v2.5-free",name:"Mimo V2.5",roles:vec!["vision","code","general"],vision:true,speed:6,reliability:0.82,max_tokens:32768,cost_weight:2},
            ModelCapability{id:"hy3-free",name:"Hy3",roles:vec!["plan","research","debug","review","security"],vision:false,speed:5,reliability:0.90,max_tokens:65536,cost_weight:3},
            ModelCapability{id:"north-mini-code-free",name:"North Mini Code",roles:vec!["review","code","explore"],vision:false,speed:8,reliability:0.78,max_tokens:16384,cost_weight:1},
            ModelCapability{id:"nemotron-3-ultra-free",name:"Nemotron Ultra",roles:vec!["debug","security","plan"],vision:false,speed:4,reliability:0.88,max_tokens:131072,cost_weight:4},
        ];
        let mut health = HashMap::new();
        let mut stats = HashMap::new();
        for c in &capabilities {
            health.insert(c.id.to_string(), ModelHealth{model_id:c.id.to_string(),available:true,last_check:"never".into(),latency_ms:0,success_rate:1.0,total_calls:0,failed_calls:0,avg_tokens_per_call:0});
            stats.insert(c.id.to_string(), ModelStats{total_calls:0,failed_calls:0,total_latency_ms:0,total_tokens:0,last_latency_ms:0});
        }
        info!("Model Registry: {} models", capabilities.len());
        Arc::new(Self{capabilities,health:RwLock::new(health),stats:RwLock::new(stats),last_probe:AtomicU64::new(0)})
    }

    /// Get supported categories (roles) from all models
    pub fn supported_categories() -> Vec<String> {
        let mut categories = std::collections::HashSet::new();
        for cap in &MODEL_REGISTRY.get().map(|r| r.capabilities.clone()).unwrap_or_default() {
            for role in &cap.roles {
                categories.insert(role.to_string());
            }
        }
        categories.into_iter().collect()
    }

    pub fn get_capability(&self, id: &str) -> Option<&ModelCapability> { self.capabilities.iter().find(|m| m.id == id) }

    pub async fn list_all(&self) -> Vec<serde_json::Value> {
        let h = self.health.read().await;
        self.capabilities.iter().map(|c| {
            let hh = h.get(c.id);
            serde_json::json!({"id":c.id,"name":c.name,"roles":c.roles,"vision":c.vision,
                "speed":c.speed,"reliability":c.reliability,"max_tokens":c.max_tokens,
                "health":hh.map(|h2| serde_json::json!({"available":h2.available,"latency_ms":h2.latency_ms,"success_rate":h2.success_rate,"calls":h2.total_calls}))})
        }).collect()
    }

    pub async fn select_for_task(&self, category: &str, effort: &str) -> ModelSelection {
        let health = self.health.read().await;
        let heavy = matches!(effort, "high"|"max");
        let cand: Vec<&ModelCapability> = self.capabilities.iter().filter(|c| c.roles.contains(&category)).collect();
        if cand.is_empty() {
            // Fallback to first available model instead of recursion
            let fallback_id = self.capabilities.iter()
                .find(|c| health.get(c.id).map(|h| h.available).unwrap_or(false))
                .map(|c| c.id.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return ModelSelection{model_id:fallback_id,reason:format!("no '{}' category",category),fallback_chain:self.capabilities.iter().map(|m|m.id.to_string()).collect()};
        }
        let mut scored: Vec<(&ModelCapability, f32)> = cand.iter().filter_map(|c| {
            let h = health.get(c.id)?;
            if !h.available { return None; }
            let rel = h.success_rate * c.reliability;
            let s = if heavy { rel * (c.max_tokens as f32 / 32768.0).min(4.0) } else { rel * (c.speed as f32 / 10.0) };
            Some((*c, s))
        }).collect();
        scored.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        if scored.is_empty() {
            let f = self.capabilities.iter().find(|c| health.get(c.id).map(|h|h.available).unwrap_or(false)).unwrap_or(&self.capabilities[0]);
            return ModelSelection{model_id:f.id.to_string(),reason:"force".into(),fallback_chain:self.capabilities.iter().map(|m|m.id.to_string()).collect()};
        }
        ModelSelection{model_id:scored[0].0.id.to_string(),reason:format!("{:.2}",scored[0].1),
            fallback_chain:scored.iter().skip(1).take(3).map(|(m,_)|m.id.to_string()).collect()}
    }

    pub async fn record_event(&self, model_id: &str, event: ModelEvent) {
        let mut s = self.stats.write().await;
        let st = s.entry(model_id.to_string()).or_insert(ModelStats{total_calls:0,failed_calls:0,total_latency_ms:0,total_tokens:0,last_latency_ms:0});
        match event {
            ModelEvent::Success{latency_ms,tokens}=>{st.total_calls+=1;st.total_latency_ms+=latency_ms;st.total_tokens+=tokens as u64;st.last_latency_ms=latency_ms;}
            ModelEvent::Error{..}=>{st.total_calls+=1;st.failed_calls+=1;}
            _=>{}
        }
        let mut health = self.health.write().await;
        if let Some(h) = health.get_mut(model_id) {
            h.total_calls=st.total_calls;h.failed_calls=st.failed_calls;
            h.success_rate=if st.total_calls>0{(st.total_calls-st.failed_calls)as f32/st.total_calls as f32}else{1.0};
            h.latency_ms=st.last_latency_ms;
            match event {
                ModelEvent::Online=>h.available=true,
                ModelEvent::Offline=>{h.available=false;warn!("{} OFFLINE", model_id);}
                ModelEvent::Error{..}=>{if h.failed_calls>3&&h.success_rate<0.3{h.available=false;warn!("{} auto-offline", model_id);}}
                _=>{}
            }
        }
    }

    pub async fn probe_model(&self, model_id: &str) -> bool {
        let start = Instant::now();
        let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().unwrap_or_default();
        let ok = client.post("https://opencode.ai/zen/v1/chat/completions")
            .header("Content-Type","application/json").header("Authorization","Bearer public")
            .json(&serde_json::json!({"model":model_id,"messages":[{"role":"user","content":"hi"}],"max_tokens":1}))
            .send().await.is_ok();
        let ms = start.elapsed().as_millis() as u64;
        if ok { self.record_event(model_id, ModelEvent::Success{latency_ms:ms,tokens:1}).await; }
        else { self.record_event(model_id, ModelEvent::Error{error:"probe".into()}).await; }
        self.health.read().await.get(model_id).map(|h|h.available).unwrap_or(false)
    }

    pub async fn probe_all(&self) -> Vec<(String, bool)> {
        let mut r = Vec::new();
        for c in &self.capabilities { r.push((c.id.to_string(), self.probe_model(c.id).await)); }
        self.last_probe.store(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(), Ordering::Relaxed);
        r
    }

    pub async fn get_stats(&self) -> serde_json::Value {
        let health = self.health.read().await;
        let avail=health.values().filter(|h|h.available).count();
        let calls:u64=health.values().map(|h|h.total_calls).sum();
        serde_json::json!({"total":self.capabilities.len(),"available":avail,"total_calls":calls,
            "models":health.values().map(|h|serde_json::json!({"id":h.model_id,"available":h.available,"success_rate":h.success_rate,"latency_ms":h.latency_ms,"calls":h.total_calls})).collect::<Vec<_>>()})
    }
}

use std::sync::OnceLock;
static MODEL_REGISTRY: OnceLock<Arc<ModelRegistry>> = OnceLock::new();
pub fn registry() -> Arc<ModelRegistry> { MODEL_REGISTRY.get_or_init(||ModelRegistry::new()).clone() }
pub fn init_registry() { registry(); info!("Model Registry ready"); }
pub async fn pick_model(category:&str,effort:&str)->ModelSelection{registry().select_for_task(category,effort).await}
pub async fn probe_all_models()->Vec<(String,bool)>{registry().probe_all().await}

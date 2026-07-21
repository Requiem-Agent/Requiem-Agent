use anyhow::Result;
use axum::{
    Router,
    Extension,
    middleware,
    routing::{delete, get, patch, post, put},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod auth;
mod models;
mod routes;
mod storage;
mod path_safety;
mod orchestrator;
mod tools;
mod sandbox;
mod formats;
mod enforce;
mod agent;
// S2-01: Error handling موحد — يستبدل unwrap() في production code
mod error;
// S2-02: ReAct Loop Engine — Reasoning + Acting pattern
mod react_loop;
// S2-03: Rate Limiting — Sliding Window per IP/user
mod rate_limit;
// S3-04: Prometheus Metrics endpoint
mod metrics;
// S3-05: Database Migration Runner
mod migrate;

pub use db::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Immediate stderr output so HF logs show something even on fast crash
    eprintln!("[requiem] main() started");

    // ── Panic hook ─────────────────────────────────────────────────────────────
    // By default a Rust panic prints the message and then calls process::abort()
    // (exit ~134). With this hook we get a clean stderr line AND force exit(2)
    // so HF Spaces shows "Exit code: 2" — clearly distinguishing panics from
    // the mysterious "Exit code: 0" that we have been observing.
    std::panic::set_hook(Box::new(|info| {
        eprintln!("[requiem] FATAL PANIC: {info}");
        std::process::exit(2);
    }));

    dotenvy::dotenv().ok();

    // Write tracing to stderr (HF captures stderr in container logs)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "requiem_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    eprintln!("[requiem] tracing initialized");

    let turso_url = match std::env::var("TURSO_URL") {
        Ok(u) => { eprintln!("[requiem] TURSO_URL set: {}", &u[..u.len().min(30)]); u }
        Err(e) => { eprintln!("[requiem] TURSO_URL missing: {e}"); std::process::exit(1); }
    };
    let turso_token = std::env::var("TURSO_AUTH_TOKEN").ok();
    eprintln!("[requiem] TURSO_AUTH_TOKEN present: {}", turso_token.is_some());

    eprintln!("[requiem] connecting to Turso...");
    let state = match AppState::new(&turso_url, turso_token).await {
        Ok(s) => { eprintln!("[requiem] Turso connected"); s }
        Err(e) => { eprintln!("[requiem] Turso connect FAILED: {e:#}"); std::process::exit(1); }
    };

    eprintln!("[requiem] running init_schema...");
    if let Err(e) = state.init_schema().await {
        eprintln!("[requiem] init_schema FAILED: {e:#}");
        std::process::exit(1);
    }
    eprintln!("[requiem] schema ready");

    // S3-05: تشغيل الـ migrations تلقائياً
    eprintln!("[requiem] running database migrations...");
    if let Err(e) = migrate::run(&state.conn).await {
        // الـ migrations غير حرجة — نُسجِّل الخطأ ونكمل
        eprintln!("[requiem] migration warning (non-fatal): {e:#}");
    } else {
        eprintln!("[requiem] migrations complete");
    }

    let state = Arc::new(state);

    // تهيئة الساندبوكس
    sandbox::init_sandbox();
    // تهيئة سجل النماذج
    models::init_registry();
    // تهيئة سجل التدقيق
    let audit_log = routes::enforce::create_audit_log(10_000);
    tracing::info!("✅ Audit log initialized (max 10,000 entries)");

    // تهيئة محرك الوكيل
    let agent_engine = Arc::new(RwLock::new(
        crate::agent::AgentEngine::new("system", crate::agent::protocol::mode::AgentMode::Autonomous, audit_log.clone(), state.conn.clone())
    ));
    tracing::info!("✅ Agent engine initialized (mode: autonomous)");

    // تهيئة مخزن الأسئلة
    let question_store = routes::user_questions::create_question_store(100);
    tracing::info!("✅ Question store initialized (max 100 questions)");

    // تهيئة حالة المهام
    let task_state: routes::tasks::SharedTaskState = Arc::new(RwLock::new(routes::tasks::TaskState::new()));
    tracing::info!("✅ Task state initialized");

    // ─── Phase 15 — Anti-Printer & Compiler Advanced ───────────────────
    let pipeline: routes::anti_printer::SharedPipeline = Arc::new(RwLock::new(
        crate::agent::anti_printer::CompilerPipeline::new()
    ));
    let context_router: routes::anti_printer::SharedRouter = Arc::new(RwLock::new(
        crate::agent::anti_printer::ContextRouter::new()
    ));
    let pattern_detector: routes::anti_printer::SharedDetector = Arc::new(RwLock::new(
        crate::agent::anti_printer::PatternDetector::new()
    ));
    let semantic_engine: routes::anti_printer::SharedSemantic = Arc::new(RwLock::new(
        crate::agent::anti_printer::SemanticEngine::new()
    ));
    tracing::info!("✅ Phase 15 — Anti-Printer & Router initialized");

    // ─── Phase 16 — Model Synergy Engine ─────────────────────────────
    let synergy_coordinator: routes::synergy::SharedSynergy = Arc::new(RwLock::new(
        crate::agent::synergy::ModelSynergyCoordinator::new()
    ));
    tracing::info!("✅ Phase 16 — Model Synergy Engine initialized");

    // S1-04: تقييد CORS — بدلاً من Any نستخدم قائمة محددة
    let allowed_origins = std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "https://requiem-agent.github.io,https://web.telegram.org".to_string());
    
    let origins: Vec<axum::http::HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|o| o.trim().parse().ok())
        .collect();
    
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true);

    let public_router = Router::new()
        .route("/healthz", get(routes::health::health_check))
        .route("/auth", post(routes::auth::telegram_auth))
        .route("/models", get(routes::models::list_models))
        // S3-04: Prometheus metrics endpoint (public — لا يحتاج auth)
        .route("/metrics", get(metrics::metrics_handler));

    // ── AXUM 0.8 ROUTER RULE ─────────────────────────────────────────────────
    // ALL .route() calls MUST come before .route_layer() and .layer() calls.
    // Adding routes after .layer() panics at runtime in Axum 0.8 and was the
    // primary cause of the HF Space "RUNTIME_ERROR / Exit code: 0".
    // Correct order: routes → .route_layer(auth) → .layer(Extensions)
    // ─────────────────────────────────────────────────────────────────────────
    let protected_router = Router::new()
        // ─── Core API Routes ─────────────────────────────────────────
        .route("/sessions", get(routes::sessions::list_sessions))
        .route("/sessions", post(routes::sessions::create_session))
        .route("/sessions/{id}", get(routes::sessions::get_session))
        .route("/sessions/{id}", patch(routes::sessions::update_session))
        .route("/sessions/{id}", delete(routes::sessions::delete_session))
        .route("/sessions/{id}/messages", get(routes::messages::list_messages))
        .route("/sessions/{id}/messages", post(routes::messages::add_message))
        .route("/bots", get(routes::bots::list_bots))
        .route("/bots", post(routes::bots::create_bot))
        .route("/bots/provision", post(routes::bots::provision_bot))
        .route("/bots/{id}", get(routes::bots::get_bot))
        .route("/bots/{id}", delete(routes::bots::delete_bot))
        .route("/bots/{id}/deploy", post(routes::bots::deploy_bot))
        .route("/bots/{id}/link-token", post(routes::bots::link_bot_token))
        .route("/usage", get(routes::usage::get_usage))
        .route("/zen/chat", post(routes::zen::chat_handler))
        .route("/sessions/{id}/files", get(routes::files::list_files))
        .route("/sessions/{id}/files", post(routes::files::save_file_body))
        .route("/sessions/{id}/files/{name}", get(routes::files::get_file))
        .route("/sessions/{id}/files/{name}", post(routes::files::save_file))
        .route("/sessions/{id}/files/{name}", delete(routes::files::delete_file))
        .route("/sessions/{id}/context", get(routes::files::get_context))
        .route("/sessions/{id}/context", post(routes::files::save_context))
        .route("/storage/usage", get(routes::files::get_storage_usage))
        // ─── Global Files Routes (frontend /api/files) ────────────────
        .route("/files",         get(routes::files::list_user_files))
        .route("/files/upload",  post(routes::files::upload_user_file))
        .route("/files/{name}",  get(routes::files::get_user_file).delete(routes::files::delete_user_file))
        // ─── Orchestrator + Tools Routes ─────────────────────────────
        .route("/tools", get(routes::tools::list_tools))
        .route("/tools/validate", post(routes::tools::validate_tool))
        .route("/tools/diff", post(routes::tools::diff_texts))
        .route("/tools/parse", post(routes::tools::parse_code_handler))
        .route("/orchestrator/classify", get(routes::tools::classify_task))
        // ─── Sandbox Routes ──────────────────────────────────────────
        .route("/sandbox/exec", post(routes::sandbox::execute_code))
        .route("/sandbox/cleanup", post(routes::sandbox::cleanup_sandbox))
        .route("/sandbox/status", get(routes::sandbox::sandbox_status))
        .route("/sandbox/stats", get(routes::sandbox::sandbox_stats))
        // ─── Model Registry Routes ───────────────────────────────────
        .route("/models/health", get(routes::models::models_health))
        .route("/models/select", get(routes::models::select_model))
        .route("/models/probe", post(routes::models::probe_model))
        .route("/models/probe-all", post(routes::models::probe_all_models))
        // ─── Format Support Routes ───────────────────────────────────
        .route("/formats", get(routes::formats::list_formats))
        .route("/formats/detect", post(routes::formats::detect_format))
        .route("/formats/{name}/validate", post(routes::formats::validate_format))
        .route("/formats/{name}/format", post(routes::formats::format_content))
        .route("/formats/{name}/convert", post(routes::formats::convert_format))
        .route("/formats/markdown/preview", post(routes::formats::preview_markdown))
        // .route("/formats/chart", post(routes::formats::generate_chart))  // TODO: Fix svg_charts
        .route("/formats/sql/exec", post(routes::formats::execute_sql))
        .route("/formats/csv/table", post(routes::formats::csv_to_table))
        // ─── Enforce Routes (Programmatic Enforcement) ────────────────
        .route("/enforce/audit", get(routes::enforce::get_audit_log))
        .route("/enforce/audit/user/{user_id}", get(routes::enforce::get_audit_by_user))
        .route("/enforce/audit/recent/{n}", get(routes::enforce::get_recent_audit))
        .route("/enforce/audit/stats", get(routes::enforce::get_audit_stats))
        .route("/enforce/check-security", post(routes::enforce::check_security))
        .route("/enforce/validate-path", post(routes::enforce::enforce_validate_path))
        // ─── Agent Protocol Routes ────────────────────────────────────
        .route("/agent/status", get(routes::agent_protocol::agent_status))
        .route("/agent/mode/set", post(routes::agent_protocol::set_mode))
        .route("/agent/mode/current", get(routes::agent_protocol::current_mode))
        .route("/agent/mode/auto-switch", post(routes::agent_protocol::auto_switch_mode))
        .route("/agent/mode/history", get(routes::agent_protocol::mode_history))
        .route("/agent/protocol/thinking/validate", post(routes::agent_protocol::validate_thinking))
        .route("/agent/compiler/correct", post(routes::agent_protocol::correct_json))
        .route("/agent/compiler/compile", post(routes::agent_protocol::compile_output))
        .route("/agent/sub/spawn", post(routes::agent_protocol::spawn_sub_agent))
        .route("/agent/sub/{id}/progress", get(routes::agent_protocol::sub_agent_progress))
        .route("/agent/sub/{id}/cancel", post(routes::agent_protocol::cancel_sub_agent))
        .route("/agent/sub/merge", post(routes::agent_protocol::merge_sub_agents))
        .route("/agent/sub/list", get(routes::agent_protocol::list_sub_agents))
        .route("/agent/environment", get(routes::agent_env::get_environment))
        .route("/agent/skills", get(routes::agent_env::list_skills))
        .route("/agent/skills/suggest", post(routes::agent_env::suggest_skills))
        .route("/agent/skills/execute", post(routes::agent_env::execute_skill))
        // ─── Tasks Routes ─────────────────────────────────────────────
        .route("/tasks/decompose", post(routes::tasks::decompose_task))
        .route("/tasks/{id}", get(routes::tasks::get_task_tree))
        .route("/tasks/{id}/status", patch(routes::tasks::update_task_status))
        .route("/tasks/{id}/assign", post(routes::tasks::assign_task))
        .route("/tasks/{id}/progress", get(routes::tasks::task_progress))
        .route("/tasks/ready", get(routes::tasks::ready_tasks))
        // ─── User Questions Routes ────────────────────────────────────
        .route("/user/question", post(routes::user_questions::ask_question))
        .route("/user/question/{id}", get(routes::user_questions::get_question))
        .route("/user/question/{id}/answer", put(routes::user_questions::answer_question))
        .route("/user/question/pending", get(routes::user_questions::pending_questions))
        .route("/user/question/{id}/cancel", post(routes::user_questions::cancel_question))
        .route("/user/question/stats", get(routes::user_questions::question_stats))
        // ─── Phase 15 — Anti-Printer Routes ───────────────────────────
        .route("/agent/semantic/analyze", post(routes::anti_printer::semantic_analyze))
        .route("/agent/semantic/context", get(routes::anti_printer::semantic_context))
        .route("/agent/anti-printer/check", post(routes::anti_printer::anti_printer_check))
        .route("/agent/anti-printer/pipeline", post(routes::anti_printer::anti_printer_pipeline))
        .route("/agent/router/route", post(routes::anti_printer::router_route))
        .route("/agent/router/models", get(routes::anti_printer::router_models))
        .route("/agent/router/strategy", post(routes::anti_printer::router_strategy))
        .route("/agent/router/outcome", post(routes::anti_printer::router_outcome))
        // ─── Phase 16 — Synergy Routes ────────────────────────────────
        .route("/agent/synergy/run", post(routes::synergy::synergy_run))
        .route("/agent/synergy/pattern", put(routes::synergy::synergy_set_pattern))
        .route("/agent/synergy/history", get(routes::synergy::synergy_history))
        .route("/agent/synergy/report", get(routes::synergy::synergy_report))
        .route("/agent/synergy/performance", get(routes::synergy::synergy_performance))
        .route("/agent/synergy/load", get(routes::synergy::synergy_load))
        // ─── Phase 5 — RAG Routes ──────────────────────────────────────
        .route("/rag/store",          post(routes::rag::store_memory))
        .route("/rag/search",         post(routes::rag::search_memories))
        .route("/rag/memories",       get(routes::rag::list_memories))
        .route("/rag/inject-context", post(routes::rag::inject_context))
        .route("/rag/auto-store",     post(routes::rag::auto_store))
        .route("/rag/memory/{id}",    get(routes::rag::get_memory).delete(routes::rag::delete_memory))
        .route("/rag/stats",          get(routes::rag::get_stats))
        .route("/rag/clear",          post(routes::rag::clear_memory))
        // ─── Phase 7 — Strict Locks Routes ────────────────────────────
        .route("/locks/check", post(routes::strict_locks::check_locks))
        .route("/locks/stats", get(routes::strict_locks::get_stats))
        .route("/locks/context", post(routes::strict_locks::get_lock_context))
        // ─── Identity Shield v3 Routes ─────────────────────────────────
        .route("/identity/check", post(routes::identity_shield::check_identity))
        .route("/identity/stats", get(routes::identity_shield::get_stats))
        .route("/identity/prompt", get(routes::identity_shield::get_identity_prompt))
        .route("/identity/cutoff", post(routes::identity_shield::check_cutoff))
        .route("/identity/developer", get(routes::identity_shield::get_developer_info))
        // ─── Agent Chat (tool-use loop) ────────────────────────────────────────
        .route("/agent/chat", post(routes::agent_chat::agent_chat_handler))
        // S4-03: WebSocket real-time agent streaming
        .route("/ws/agent", get(routes::ws_agent::ws_handler::<AppState>))
         // ─── Workspace Routes ─────────────────────────────────────────────────
         // Axum 0.8: same path must chain methods — separate .route() on same path panics at runtime
         .route("/workspaces",
                get(routes::workspaces::list_workspaces).post(routes::workspaces::create_workspace))
         .route("/workspaces/{id}",
                get(routes::workspaces::get_workspace)
                .patch(routes::workspaces::update_workspace)
                .delete(routes::workspaces::delete_workspace))
        .route("/workspaces/{id}/tree",              get(routes::workspaces::get_tree))
        .route("/workspaces/{id}/files/{*path}",       get(routes::workspaces::read_file)
                                                        .put(routes::workspaces::write_file)
                                                        .delete(routes::workspaces::delete_file))
        .route("/workspaces/{id}/mkdir/{*path}",       post(routes::workspaces::mkdir))
        .route("/workspaces/{id}/clone",             post(routes::workspaces::clone_repo))
        // ── Auth middleware — MUST come after all .route() calls (Axum 0.8 rule) ──
        // route_layer applies to every route defined above it in this builder chain.
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            routes::auth_middleware,
        ))
        // ── Extensions — added after route_layer; available to all handlers above ──
        .layer(Extension(audit_log))
        .layer(Extension(agent_engine.clone()))
        .layer(Extension(question_store.clone()))
        .layer(Extension(task_state.clone()))
        .layer(Extension(pipeline))
        .layer(Extension(context_router))
        .layer(Extension(pattern_detector))
        .layer(Extension(semantic_engine))
        .layer(Extension(synergy_coordinator));

    let api_router = Router::new()
        .merge(public_router)
        .merge(protected_router)
        .with_state(state.clone());

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "7860".to_string())
        .parse::<u16>()
        .unwrap_or(7860);

    let app = Router::new()
        .nest("/api", api_router)
        // Axum 0.8: nest_service("/", ...) at root is removed — use fallback_service instead
        .fallback_service(ServeDir::new("public"))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    eprintln!("[requiem] binding to 0.0.0.0:{port}");
    let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await {
        Ok(l) => { eprintln!("[requiem] listening on port {port}"); l }
        Err(e) => { eprintln!("[requiem] bind FAILED: {e}"); std::process::exit(1); }
    };

    tracing::info!("Requiem Agent server running on port {port}");
    eprintln!("[requiem] calling axum::serve — server is live");

    // ── Graceful shutdown on SIGTERM ────────────────────────────────────────────
    // Without an explicit SIGTERM handler, Tokio (with features = ["full"]) may
    // intercept SIGTERM silently and cause axum::serve to return Ok(()) which HF
    // Spaces interprets as "Exit code: 0 — RUNTIME_ERROR".
    // We handle it explicitly: log the receipt, let axum drain connections,
    // then force exit(1) so HF always sees a non-zero code and restarts cleanly.
    let shutdown_signal = async {
        let _ = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("[requiem] failed to install SIGTERM handler")
            .recv()
            .await;
        eprintln!("[requiem] SIGTERM received — beginning graceful shutdown");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    // axum::serve MUST NOT return normally for a healthy long-running server.
    // If we reach here the server shut down (SIGTERM or unexpected listener error).
    // Force exit(1) so HF Spaces shows a non-zero code and schedules a restart,
    // rather than silently logging "Exit code: 0" and leaving the space dark.
    eprintln!("[requiem] axum::serve returned — forcing exit(1) for clean HF restart");
    std::process::exit(1);
}
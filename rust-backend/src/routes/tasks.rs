//! # Task Routes — مسارات إدارة المهام
//!
//! POST /api/tasks/decompose        → تحليل طلب إلى شجرة مهام
//! GET  /api/tasks/:id              → عرض شجرة المهام
//! PATCH /api/tasks/:id/status      → تحديث حالة مهمة
//! POST /api/tasks/:id/assign       → تعيين مهمة لنموذج/وكيل
//! GET  /api/tasks/:id/progress     → تقرير التقدم
//! GET  /api/tasks/:id/ready        → المهام الجاهزة للتنفيذ

use axum::{Extension, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::routes::AuthUser;
use crate::agent::tasks::tree::TaskTree;
use crate::agent::tasks::scheduler::{TaskScheduler, SchedulerConfig};

/// مشاركة حالة المهام عبر التطبيق
pub type SharedTaskState = Arc<RwLock<TaskState>>;

/// حالة المهام الكاملة
pub struct TaskState {
    pub tree: Option<TaskTree>,
    pub scheduler: TaskScheduler,
    pub trees: Vec<(String, TaskTree)>,  // تاريخ الأشجار (للتتبع)
}

impl TaskState {
    pub fn new() -> Self {
        Self {
            tree: None,
            scheduler: TaskScheduler::new(SchedulerConfig::default()),
            trees: Vec::new(),
        }
    }
}

/// POST /api/tasks/decompose
pub async fn decompose_task(
    Extension(_auth): Extension<AuthUser>,
    Extension(state): Extension<SharedTaskState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let description = body["description"].as_str().unwrap_or("");
    let owner = body["owner"].as_str().unwrap_or("default");

    if description.is_empty() {
        return Json(json!({ "success": false, "error": "description مطلوب" }));
    }

    let mut task_state = state.write().await;
    let tree = TaskTree::new(description, owner);
    let tree_id = format!("tree-{}", task_state.trees.len() + 1);

    task_state.trees.push((tree_id.clone(), tree));

    Json(json!({
        "success": true,
        "tree_id": tree_id,
        "tree": task_state.trees.last().map(|(_, t)| t.to_json()),
    }))
}

/// GET /api/tasks/:id
pub async fn get_task_tree(
    Extension(_auth): Extension<AuthUser>,
    Extension(state): Extension<SharedTaskState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<Value> {
    let task_state = state.read().await;
    if id == "current" {
        match &task_state.tree {
            Some(tree) => Json(json!({ "success": true, "tree": tree.to_json() })),
            None => Json(json!({ "success": false, "error": "لا توجد مهمة حالية" })),
        }
    } else {
        match task_state.trees.iter().find(|(tid, _)| tid == &id) {
            Some((_, tree)) => Json(json!({ "success": true, "tree": tree.to_json() })),
            None => Json(json!({ "success": false, "error": format!("شجرة المهام {} غير موجودة", id) })),
        }
    }
}

/// PATCH /api/tasks/:id/status
pub async fn update_task_status(
    Extension(_auth): Extension<AuthUser>,
    Extension(state): Extension<SharedTaskState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    use crate::agent::tasks::tree::TaskStatus;
    let status_str = body["status"].as_str().unwrap_or("");
    let tree_id = body["tree_id"].as_str().unwrap_or("current");

    let status = match status_str {
        "pending" => TaskStatus::Pending,
        "in_progress" => TaskStatus::InProgress,
        "completed" => TaskStatus::Completed,
        "cancelled" => TaskStatus::Cancelled,
        _ => return Json(json!({ "success": false, "error": format!("حالة غير معروفة: {status_str}") })),
    };

    let mut task_state = state.write().await;
    let tree = if tree_id == "current" {
        task_state.tree.as_mut()
    } else {
        task_state.trees.iter_mut().find(|(tid, _)| tid == tree_id).map(|(_, t)| t)
    };

    match tree {
        Some(tree) => match tree.update_status(&task_id, status) {
            Ok(_) => {
                let progress = tree.progress_report();
                Json(json!({
                    "success": true,
                    "task_id": task_id,
                    "status": status_str,
                    "progress": progress,
                }))
            }
            Err(e) => Json(json!({ "success": false, "error": e })),
        },
        None => Json(json!({ "success": false, "error": "شجرة المهام غير موجودة" })),
    }
}

/// POST /api/tasks/:id/assign
pub async fn assign_task(
    Extension(_auth): Extension<AuthUser>,
    Extension(state): Extension<SharedTaskState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let model_id = body["model_id"].as_str();
    let sub_agent_id = body["sub_agent_id"].as_str();

    let mut task_state = state.write().await;
    let tree = task_state.tree.as_mut();

    match tree {
        Some(tree) => {
            if let Some(model) = model_id {
                let _ = tree.assign_model(&task_id, model);
            }
            if let Some(sub) = sub_agent_id {
                let _ = tree.assign_sub_agent(&task_id, sub);
            }
            Json(json!({ "success": true, "task_id": task_id }))
        }
        None => Json(json!({ "success": false, "error": "لا توجد مهمة حالية" })),
    }
}

/// GET /api/tasks/:id/progress
pub async fn task_progress(
    Extension(_auth): Extension<AuthUser>,
    Extension(state): Extension<SharedTaskState>,
    axum::extract::Path(tree_id): axum::extract::Path<String>,
) -> Json<Value> {
    let task_state = state.read().await;
    let tree = if tree_id == "current" {
        task_state.tree.as_ref()
    } else {
        task_state.trees.iter().find(|(tid, _)| tid == &tree_id).map(|(_, t)| t)
    };

    match tree {
        Some(tree) => Json(json!({
            "success": true,
            "progress": tree.progress_report(),
            "ready_tasks": tree.ready_tasks().iter().map(|t| serde_json::json!({
                "id": t.id,
                "content": t.content,
                "priority": format!("{:?}", t.priority),
            })).collect::<Vec<_>>(),
        })),
        None => Json(json!({ "success": false, "error": "شجرة المهام غير موجودة" })),
    }
}

/// GET /api/tasks/:id/ready
pub async fn ready_tasks(
    Extension(_auth): Extension<AuthUser>,
    Extension(state): Extension<SharedTaskState>,
) -> Json<Value> {
    let task_state = state.read().await;
    match &task_state.tree {
        Some(tree) => {
            let ready = tree.ready_tasks();
            Json(json!({
                "success": true,
                "count": ready.len(),
                "tasks": ready.iter().map(|t| serde_json::json!({
                    "id": t.id,
                    "content": t.content,
                    "priority": format!("{:?}", t.priority),
                })).collect::<Vec<_>>(),
            }))
        }
        None => Json(json!({ "success": false, "error": "لا توجد مهمة حالية" })),
    }
}

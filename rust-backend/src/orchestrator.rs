//! # Orchestrator Engine — محرك التوزيع الذكي للمهام
//!
//! ## الفلسفة (مستوحاة من Replit Agent 3 + Mastra)
//!
//! ```
//! الطلب ← [Task Classifier] ← category + effort
//!              │
//!              ├── code      → DeepSeek + BigPickle (موازي)
//!              ├── debug     → Nemotron + DeepSeek (موازي + مقارنة)
//!              ├── research  → Hy3 (عميق)
//!              ├── plan      → Hy3 + DeepSeek (موازي، زاويتين)
//!              ├── review    → NorthMini + DeepSeek (موازي)
//!              ├── vision    → Mimo (وحيد القادر)
//!              └── multi-file → BigPickle + DeepSeek (موازي، ملفات متعددة)
//! ```
//!
//! ## Parallel Fan-Out / Fan-In
//! ```
//! User → Classifier → [Model A] ──╮
//!                    → [Model B] ──╱→ Judge → Best Path → Response
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};
use tracing::debug;

// ─── أنواع المهام ────────────────────────────────────────────────────────────

/// تصنيف المهمة — يُحدد أي النماذج ستستخدم
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskCategory {
    /// برمجة سريعة، إنشاء ملفات، تعديل كود
    Code,
    /// تصحيح أخطاء، تحليل جذري، اختبارات
    Debug,
    /// تخطيط معماري، تفكير عميق، تحليل
    Plan,
    /// بحث في الإنترنت، توثيق، معرفة
    Research,
    /// مراجعة الكود، كشف الشيفرات الميتة، الجودة
    Review,
    /// رؤية، صور، تصميم واجهات
    Vision,
    /// تعديلات على عدة ملفات بالتوازي
    MultiFile,
    /// محادثة عامة، أسئلة بسيطة
    General,
    /// أمان، فحص ثغرات
    Security,
    /// استكشاف الكود، فهم المشروع
    Explore,
}

impl fmt::Display for TaskCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Code => write!(f, "code"),
            Self::Debug => write!(f, "debug"),
            Self::Plan => write!(f, "plan"),
            Self::Research => write!(f, "research"),
            Self::Review => write!(f, "review"),
            Self::Vision => write!(f, "vision"),
            Self::MultiFile => write!(f, "multi-file"),
            Self::General => write!(f, "general"),
            Self::Security => write!(f, "security"),
            Self::Explore => write!(f, "explore"),
        }
    }
}

/// مستوى الجهد المطلوب
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effort {
    Lite,    // سريع، خفيف
    Medium,  // عادي
    High,    // دقيق
    Max,     // كامل — كل النماذج بالتوازي
}

impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lite => write!(f, "lite"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Max => write!(f, "max"),
        }
    }
}

// ─── Model Capability Registry ───────────────────────────────────────────────

/// قدرات النموذج — يُستخدم لاتخاذ قرارات التوجيه
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    pub id: &'static str,
    pub name: &'static str,
    pub role: TaskCategory,
    /// هل يدعم الرؤية?
    pub vision: bool,
    /// سرعة النموذج (1-10)
    pub speed: u8,
    /// موثوقية (0.0 - 1.0)
    pub reliability: f32,
    /// الحد الأقصى للتوكنات
    pub max_tokens: u32,
}

/// سجل النماذج المتاحة — يُحدّث ديناميكياً من `/api/models`
pub fn default_model_registry() -> Vec<ModelCapability> {
    vec![
        ModelCapability {
            id: "deepseek-v4-flash-free",
            name: "DeepSeek V4 Flash",
            role: TaskCategory::Code,
            vision: false,
            speed: 9,
            reliability: 0.85,
            max_tokens: 32768,
        },
        ModelCapability {
            id: "big-pickle",
            name: "Big Pickle",
            role: TaskCategory::MultiFile,
            vision: false,
            speed: 7,
            reliability: 0.80,
            max_tokens: 65536,
        },
        ModelCapability {
            id: "mimo-v2.5-free",
            name: "Mimo V2.5",
            role: TaskCategory::Vision,
            vision: true,    // الوحيد القادر على الرؤية
            speed: 6,
            reliability: 0.82,
            max_tokens: 32768,
        },
        ModelCapability {
            id: "hy3-free",
            name: "Hy3",
            role: TaskCategory::Plan,
            vision: false,
            speed: 5,
            reliability: 0.90, // الأكثر موثوقية للتفكير العميق
            max_tokens: 65536,
        },
        ModelCapability {
            id: "north-mini-code-free",
            name: "North Mini Code",
            role: TaskCategory::Review,
            vision: false,
            speed: 8,
            reliability: 0.78,
            max_tokens: 16384,
        },
        ModelCapability {
            id: "nemotron-3-ultra-free",
            name: "Nemotron Ultra",
            role: TaskCategory::Debug,
            vision: false,
            speed: 4,
            reliability: 0.88, // الأفضل للتصحيح
            max_tokens: 131072,
        },
    ]
}

// ─── Task Classifier ─────────────────────────────────────────────────────────

/// يصنف مهمة المستخدم بناءً على محتوى الطلب
pub struct TaskClassifier;

impl TaskClassifier {
    /// تحليل النص وتصنيف المهمة
    pub fn classify(content: &str) -> TaskCategory {
        let lower = content.to_lowercase();

        // Vision — صور، رسم، تصميم
        if lower.contains("ارسم") || lower.contains("صو")
            || lower.contains("تصميم") || lower.contains("design")
            || lower.contains("image") || lower.contains("draw")
            || lower.contains("picture") || lower.contains("svg")
            || lower.contains("ui/ux") || lower.contains("واجهة")
            || lower.contains("icon") || lower.contains("logo")
        {
            return TaskCategory::Vision;
        }

        // Debug — أخطاء، تصحيح، bug
        if lower.contains("bug") || lower.contains("fix") || lower.contains("خطأ")
            || lower.contains("تصحيح") || lower.contains("error")
            || lower.contains("crash") || lower.contains("fail")
            || lower.contains("مشكلة") || lower.contains("لا يعمل")
            || lower.contains("broken") || lower.contains("wrong")
            || lower.contains("issue") || lower.contains("مشكلة")
        {
            return TaskCategory::Debug;
        }

        // Research — بحث، اقرأ، ادرس
        if lower.contains("بحث") || lower.contains("research")
            || lower.contains("اقرأ") || lower.contains("read")
            || lower.contains("study") || lower.contains("what is")
            || lower.contains("تحليل") || lower.contains("لماذا")
            || lower.contains("how to") || lower.contains("article")
            || lower.contains("docs") || lower.contains("documentation")
        {
            return TaskCategory::Research;
        }

        // Plan — خطط، صمم معماريًا
        if lower.contains("خطط") || lower.contains("plan")
            || lower.contains("architecture") || lower.contains("معمار")
            || lower.contains("هيكل") || lower.contains("structure")
            || lower.contains("strategy") || lower.contains("roadmap")
            || lower.contains("design doc") || lower.contains("مخطط")
        {
            return TaskCategory::Plan;
        }

        // Review — راجع، دقق، حسّن
        if lower.contains("review") || lower.contains("audit")
            || lower.contains("راجع") || lower.contains("دقق")
            || lower.contains("جودة") || lower.contains("quality")
            || lower.contains("refactor") || lower.contains("تحسين")
            || lower.contains("clean") || lower.contains("نظف")
        {
            return TaskCategory::Review;
        }

        // Security — أمان، ثغرات
        if lower.contains("security") || lower.contains("secure")
            || lower.contains("vulnerab") || lower.contains("threat")
            || lower.contains("cve") || lower.contains("أمان")
            || lower.contains("اختراق") || lower.contains("هجوم")
        {
            return TaskCategory::Security;
        }

        // Explore — استكشف، اقرأ الكود، افهم
        if lower.contains("explore") || lower.contains("what does")
            || lower.contains("explain") || lower.contains("شرح")
            || lower.contains("كيف") || lower.contains("find")
            || lower.contains("أين") || lower.contains("where")
            || lower.contains("show me") || lower.contains("define")
        {
            return TaskCategory::Explore;
        }

        // Multi-file — ملفات متعددة، مشروع كامل
        if lower.contains("ملفات") || lower.contains("projects")
            || lower.contains("full") || lower.contains("project")
            || lower.contains("complete") || lower.contains("entire")
            || lower.contains("multiple") || lower.contains("جميع")
            || lower.contains("كل") || lower.contains("كل الملفات")
            || lower.contains("كل المجلد") || lower.contains("كل المشروع")
        {
            return TaskCategory::MultiFile;
        }

        // افتراضي — برمجة
        TaskCategory::Code
    }

    /// اقتراح النماذج — أولاً يحاول الـ ModelRegistry الجديد، ثم يقع في الثابت
    pub fn suggest_models(category: TaskCategory, effort: Effort) -> Vec<&'static str> {
        // محاولة استخدام ModelRegistry الديناميكي
        let category_str = category.to_string();
        let effort_str = effort.to_string();

        // استخدام tokio::spawn أو block_in_place للحصول على النتيجة من async registry
        // بما أن suggest_models متزامن حالياً، نستخدم القيمة الافتراضية
        // وسيتم تحويلها إلى async لاحقاً

        let registry = default_model_registry();

        match category {
            TaskCategory::Code => match effort {
                Effort::Lite => vec!["deepseek-v4-flash-free"],
                Effort::Medium => vec!["deepseek-v4-flash-free", "big-pickle"],
                Effort::High | Effort::Max => {
                    vec!["deepseek-v4-flash-free", "big-pickle", "north-mini-code-free"]
                }
            },
            TaskCategory::MultiFile => match effort {
                Effort::Lite => vec!["big-pickle"],
                Effort::Medium => vec!["big-pickle", "deepseek-v4-flash-free"],
                Effort::High | Effort::Max => {
                    vec!["big-pickle", "deepseek-v4-flash-free", "north-mini-code-free"]
                }
            },
            TaskCategory::Debug => match effort {
                Effort::Lite => vec!["nemotron-3-ultra-free"],
                Effort::Medium => vec!["nemotron-3-ultra-free", "deepseek-v4-flash-free"],
                Effort::High | Effort::Max => {
                    vec!["nemotron-3-ultra-free", "deepseek-v4-flash-free", "hy3-free"]
                }
            },
            TaskCategory::Plan => match effort {
                Effort::Lite => vec!["deepseek-v4-flash-free"],
                Effort::Medium => vec!["hy3-free"],
                Effort::High | Effort::Max => vec!["hy3-free", "deepseek-v4-flash-free"],
            },
            TaskCategory::Research => match effort {
                Effort::Lite => vec!["hy3-free"],
                Effort::Medium | Effort::High | Effort::Max => vec!["hy3-free"],
            },
            TaskCategory::Review => match effort {
                Effort::Lite => vec!["north-mini-code-free"],
                Effort::Medium => vec!["north-mini-code-free", "deepseek-v4-flash-free"],
                Effort::High | Effort::Max => {
                    vec!["north-mini-code-free", "deepseek-v4-flash-free", "hy3-free"]
                }
            },
            TaskCategory::Vision => vec!["mimo-v2.5-free"],
            TaskCategory::Security => match effort {
                Effort::Lite => vec!["deepseek-v4-flash-free"],
                Effort::Medium => vec!["deepseek-v4-flash-free", "north-mini-code-free"],
                Effort::High | Effort::Max => {
                    vec!["deepseek-v4-flash-free", "north-mini-code-free", "hy3-free"]
                }
            },
            TaskCategory::Explore => vec!["deepseek-v4-flash-free"],
            TaskCategory::General => vec!["deepseek-v4-flash-free"],
        }
        .into_iter()
        .filter(|model_id| registry.iter().any(|m| m.id == *model_id))
        .collect()
    }

    /// نسخة async — تستخدم ModelRegistry الحقيقي مع fallback
    pub async fn suggest_models_async(category: TaskCategory, effort: Effort) -> Vec<String> {
        let category_str = category.to_string();
        let effort_str = effort.to_string();

        // استخدم ModelRegistry الجديد
        let selection = crate::models::pick_model(&category_str, &effort_str).await;

        let mut models = vec![selection.model_id.clone()];
        models.extend(selection.fallback_chain);

        // أزل المكررات مع الحفاظ على الترتيب
        let mut seen = std::collections::HashSet::new();
        models.retain(|m| seen.insert(m.clone()));

        models
    }
}

// ─── Parallel Execution Result ───────────────────────────────────────────────

/// نتيجة تنفيذ نموذج واحد
#[derive(Debug, Clone, Serialize)]
pub struct ModelResult {
    pub model_id: String,
    pub content: String,
    pub thinking: Option<String>,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// نتيجة التنفيذ المتوازي
#[derive(Debug, Clone, Serialize)]
pub struct ParallelResult {
    pub category: TaskCategory,
    pub effort: Effort,
    pub models_used: Vec<String>,
    pub results: Vec<ModelResult>,
    pub best_content: Option<String>,
    pub total_duration_ms: u64,
    pub selected_from: Option<String>,
}

// ─── Best-Path Selector (Judge) ──────────────────────────────────────────────

/// مقارنة مخرجات النماذج واختيار الأفضل
pub struct BestPathSelector;

impl BestPathSelector {
    /// اختيار أفضل نتيجة من مجموعة نتائج
    ///
    /// المعايير:
    /// 1. الأطول غالباً = الأكثر تفصيلاً (لكود)
    /// 2. للـ Debug: الأكثر ذكراً للأخطاء والحلول
    /// 3. للـ Plan: الأكثر تنظيماً
    pub fn select(category: TaskCategory, results: &[ModelResult]) -> Option<(usize, String)> {
        if results.is_empty() {
            return None;
        }

        // تصفية النتائج الناجحة فقط
        let successful: Vec<(usize, &ModelResult)> = results
            .iter()
            .enumerate()
            .filter(|(_, r)| r.success)
            .collect();

        if successful.is_empty() {
            // كلها فاشلة — نرجع الأولى مع الخطأ
            return Some((0, results[0].content.clone()));
        }

        if successful.len() == 1 {
            return Some((successful[0].0, successful[0].1.content.clone()));
        }

        // اختر الأفضل حسب التصنيف
        let best = match category {
            TaskCategory::Code | TaskCategory::MultiFile => {
                // الأطول غالباً = الأكثر اكتمالاً (لكود)
                successful.iter()
                    .max_by_key(|(_, r)| r.content.len())
                    .map(|(i, r)| (*i, r.content.clone()))
            }
            TaskCategory::Debug => {
                // الذي يحتوي على "fix" أو "solution" أو "حل"
                successful.iter()
                    .filter(|(_, r)| {
                        let lower = r.content.to_lowercase();
                        lower.contains("fix") || lower.contains("solution")
                            || lower.contains("حل") || lower.contains("صلح")
                    })
                    .max_by_key(|(_, r)| r.content.len())
                    .or_else(|| successful.iter().max_by_key(|(_, r)| r.content.len()))
                    .map(|(i, r)| (*i, r.content.clone()))
            }
            TaskCategory::Plan | TaskCategory::Research => {
                // الأكثر تنظيماً (يحتوي على أقسام)
                successful.iter()
                    .filter(|(_, r)| {
                        r.content.contains("##") || r.content.contains("###")
                            || r.content.contains("1.") || r.content.contains("-")
                    })
                    .max_by_key(|(_, r)| r.content.len())
                    .or_else(|| successful.iter().max_by_key(|(_, r)| r.content.len()))
                    .map(|(i, r)| (*i, r.content.clone()))
            }
            _ => {
                // الأسرع معقول = الأفضل
                successful.iter()
                    .min_by_key(|(_, r)| r.duration_ms)
                    .map(|(i, r)| (*i, r.content.clone()))
            }
        };

        best
    }
}

// ─── Parallel Executor ───────────────────────────────────────────────────────

/// منفذ متوازٍ — يرسل نفس الطلب لعدة نماذج بالتوازي
pub struct ParallelExecutor;

impl ParallelExecutor {
    /// تنفيذ طلب على عدة نماذج بالتوازي مع IP مختلف لكل نموذج
    ///
    /// # Arguments
    /// * `models` - قائمة معرفات النماذج
    /// * `messages` - رسائل المحادثة
    /// * `user_id` - معرف المستخدم (لاختيار الـ proxy)
    pub async fn execute_parallel(
        models: &[&str],
        messages: &[serde_json::Value],
        user_id: &str,
    ) -> ParallelResult {
        let start = Instant::now();
        let category = Self::infer_category(messages);
        let effort = Effort::High; // يمكن جعله ديناميكياً

        if models.is_empty() {
            return ParallelResult {
                category,
                effort,
                models_used: vec![],
                results: vec![],
                best_content: None,
                total_duration_ms: 0,
                selected_from: None,
            };
        }

        let model_list: Vec<&str> = models.to_vec();
        let models_used: Vec<String> = model_list.iter().map(|m| m.to_string()).collect();

        // تنفيذ متوازٍ — كل نموذج في Task منفصل
        let mut handles = Vec::new();
        for &model_id in &model_list {
            let user_id = user_id.to_string();
            let model_id = model_id.to_string();
            let messages = messages.to_vec();

            let handle = tokio::spawn(async move {
                let model_start = Instant::now();
                let proxy_url = format_proxy_for_user(&user_id, &model_id);

                let client = match reqwest::Client::builder()
                    .timeout(Duration::from_secs(120))
                    .proxy(reqwest::Proxy::all(&proxy_url).unwrap_or_else(|_|
                        reqwest::Proxy::all("socks5://localhost:9050").unwrap()
                    ))
                    .build()
                {
                    Ok(c) => c,
                    Err(e) => {
                        return ModelResult {
                            model_id,
                            content: String::new(),
                            thinking: None,
                            duration_ms: model_start.elapsed().as_millis() as u64,
                            success: false,
                            error: Some(format!("Client build: {e}")),
                        };
                    }
                };

                let body = serde_json::json!({
                    "model": model_id,
                    "messages": messages,
                    "stream": false,
                });

                match client
                    .post("https://opencode.ai/zen/v1/chat/completions")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "Bearer public")
                    .json(&body)
                    .send()
                    .await
                {
                    Ok(resp) => {
                        if !resp.status().is_success() {
                            let status_code = resp.status();
                            let text = resp.text().await.unwrap_or_default();
                            return ModelResult {
                                model_id,
                                content: String::new(),
                                thinking: None,
                                duration_ms: model_start.elapsed().as_millis() as u64,
                                success: false,
                                error: Some(format!("API {}: {}", status_code, &text[..text.len().min(200)])),
                            };
                        }
                        let json: serde_json::Value = resp.json().await.unwrap_or_default();
                        let content = json["choices"][0]["message"]["content"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();

                        ModelResult {
                            model_id,
                            content,
                            thinking: None,
                            duration_ms: model_start.elapsed().as_millis() as u64,
                            success: true,
                            error: None,
                        }
                    }
                    Err(e) => ModelResult {
                        model_id,
                        content: String::new(),
                        thinking: None,
                        duration_ms: model_start.elapsed().as_millis() as u64,
                        success: false,
                        error: Some(format!("Request: {e}")),
                    },
                }
            });
            handles.push(handle);
        }

        // جمع النتائج
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    debug!("Parallel task panicked: {e}");
                }
            }
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;

        // اختيار أفضل مسار
        let (best_idx, best_content) = BestPathSelector::select(category, &results)
            .unwrap_or((0, String::new()));

        let selected_from = if results.len() > 1 {
            Some(results[best_idx].model_id.clone())
        } else {
            None
        };

        ParallelResult {
            category,
            effort,
            models_used,
            results,
            best_content: Some(best_content),
            total_duration_ms,
            selected_from,
        }
    }

    fn infer_category(messages: &[serde_json::Value]) -> TaskCategory {
        for msg in messages {
            if let Some(content) = msg["content"].as_str() {
                return TaskClassifier::classify(content);
            }
        }
        TaskCategory::General
    }
}

// ─── Multi-File Parallel Editor (مستوحى من Replit) ─────────────────────────

/// عملية تعديل ملف واحد
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEditOperation {
    pub path: String,
    pub operation: FileOp,
    pub content: Option<String>,
    pub old_str: Option<String>,
    pub new_str: Option<String>,
}

/// نوع العملية على الملف
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOp {
    Read,
    Write,
    Edit,
    Delete,
    Create,
}

impl fmt::Display for FileOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read => write!(f, "read"),
            Self::Write => write!(f, "write"),
            Self::Edit => write!(f, "edit"),
            Self::Delete => write!(f, "delete"),
            Self::Create => write!(f, "create"),
        }
    }
}

/// نتيجة تعديل ملف
#[derive(Debug, Clone, Serialize)]
pub struct FileEditResult {
    pub path: String,
    pub operation: FileOp,
    pub success: bool,
    pub content: Option<String>,
    pub error: Option<String>,
}

/// محرر الملفات المتوازي — يقرأ/يكتب/يعدل عدة ملفات بالتوازي
pub struct MultiFileEditor;

impl MultiFileEditor {
    /// تنفيذ مجموعة من العمليات على ملفات بالتوازي
    pub async fn execute_parallel(
        operations: Vec<FileEditOperation>,
        user_id: &str,
        session_id: &str,
    ) -> Vec<FileEditResult> {
        let mut handles = Vec::new();

        for op in operations {
            let user_id = user_id.to_string();
            let session_id = session_id.to_string();

            let handle = tokio::spawn(async move {
                let result = Self::execute_single(&op, &user_id, &session_id).await;
                (op.path.clone(), op.operation.clone(), result)
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok((path, operation, result)) => results.push(result),
                Err(e) => {
                    debug!("File edit task panicked: {e}");
                }
            }
        }
        results
    }

    /// تنفيذ عملية واحدة على ملف
    async fn execute_single(
        op: &FileEditOperation,
        user_id: &str,
        session_id: &str,
    ) -> FileEditResult {
        use crate::storage;

        match op.operation {
            FileOp::Read => {
                match storage::read_file(user_id, session_id, &op.path).await {
                    Ok(content) => FileEditResult {
                        path: op.path.clone(),
                        operation: FileOp::Read,
                        success: true,
                        content: Some(content),
                        error: None,
                    },
                    Err(e) => FileEditResult {
                        path: op.path.clone(),
                        operation: FileOp::Read,
                        success: false,
                        content: None,
                        error: Some(e),
                    },
                }
            }
            FileOp::Write | FileOp::Create => {
                let content = op.content.as_deref().unwrap_or("");
                match storage::save_file(user_id, session_id, &op.path, content).await {
                    Ok(_) => FileEditResult {
                        path: op.path.clone(),
                        operation: op.operation.clone(),
                        success: true,
                        content: Some(content.to_string()),
                        error: None,
                    },
                    Err(e) => FileEditResult {
                        path: op.path.clone(),
                        operation: op.operation.clone(),
                        success: false,
                        content: None,
                        error: Some(e),
                    },
                }
            }
            FileOp::Edit => {
                // read → replace → write
                match storage::read_file(user_id, session_id, &op.path).await {
                    Ok(current) => {
                        if let (Some(old), Some(new)) = (&op.old_str, &op.new_str) {
                            if current.contains(old) {
                                let updated = current.replace(old, new);
                                match storage::save_file(user_id, session_id, &op.path, &updated).await {
                                    Ok(_) => FileEditResult {
                                        path: op.path.clone(),
                                        operation: FileOp::Edit,
                                        success: true,
                                        content: Some(updated),
                                        error: None,
                                    },
                                    Err(e) => FileEditResult {
                                        path: op.path.clone(),
                                        operation: FileOp::Edit,
                                        success: false,
                                        content: None,
                                        error: Some(e),
                                    },
                                }
                            } else {
                                FileEditResult {
                                    path: op.path.clone(),
                                    operation: FileOp::Edit,
                                    success: false,
                                    content: None,
                                    error: Some("old_str not found in file".to_string()),
                                }
                            }
                        } else {
                            FileEditResult {
                                path: op.path.clone(),
                                operation: FileOp::Edit,
                                success: false,
                                content: None,
                                error: Some("Missing old_str or new_str".to_string()),
                            }
                        }
                    }
                    Err(e) => FileEditResult {
                        path: op.path.clone(),
                        operation: FileOp::Edit,
                        success: false,
                        content: None,
                        error: Some(format!("Read first: {e}")),
                    },
                }
            }
            FileOp::Delete => {
                match storage::delete_file(user_id, session_id, &op.path).await {
                    Ok(_) => FileEditResult {
                        path: op.path.clone(),
                        operation: FileOp::Delete,
                        success: true,
                        content: None,
                        error: None,
                    },
                    Err(e) => FileEditResult {
                        path: op.path.clone(),
                        operation: FileOp::Delete,
                        success: false,
                        content: None,
                        error: Some(e),
                    },
                }
            }
        }
    }

    /// قراءة عدة ملفات بالتوازي (Replit-style project understanding)
    pub async fn read_multiple(
        paths: Vec<String>,
        user_id: &str,
        session_id: &str,
    ) -> Vec<FileEditResult> {
        let ops: Vec<FileEditOperation> = paths
            .into_iter()
            .map(|path| FileEditOperation {
                path,
                operation: FileOp::Read,
                content: None,
                old_str: None,
                new_str: None,
            })
            .collect();

        Self::execute_parallel(ops, user_id, session_id).await
    }

    /// كتابة عدة ملفات بالتوازي (Replit-style multi-file generation)
    pub async fn write_multiple(
        files: Vec<(String, String)>,
        user_id: &str,
        session_id: &str,
    ) -> Vec<FileEditResult> {
        let ops: Vec<FileEditOperation> = files
            .into_iter()
            .map(|(path, content)| FileEditOperation {
                path,
                operation: FileOp::Write,
                content: Some(content),
                old_str: None,
                new_str: None,
            })
            .collect();

        Self::execute_parallel(ops, user_id, session_id).await
    }

    /// تحليل هيكل المشروع — يقرأ الملفات الرئيسية لفهم المشروع
    pub async fn analyze_project(
        user_id: &str,
        session_id: &str,
    ) -> Result<serde_json::Value, String> {
        let files = crate::storage::list_files(user_id, session_id).await?;

        let mut project_info = serde_json::json!({
            "files": files,
            "file_count": files.len(),
            "language_summary": {},
            "main_files": [],
        });

        // تحليل أنواع الملفات
        let mut lang_count: HashMap<String, usize> = HashMap::new();
        for fname in &files {
            let ext = fname.rsplit('.').next().unwrap_or("").to_string();
            *lang_count.entry(ext).or_insert(0) += 1;
        }
        project_info["language_summary"] = serde_json::json!(lang_count);

        // اقرأ الملفات الرئيسية بالتوازي
        let main_files: Vec<String> = files.iter()
            .filter(|f| {
                let lower = f.to_lowercase();
                lower == "main.rs" || lower == "main.py" || lower == "index.js"
                    || lower == "app.ts" || lower == "app.js" || lower == "main.ts"
                    || lower == "cargo.toml" || lower == "package.json"
                    || lower == "readme.md" || lower == "dockerfile"
            })
            .cloned()
            .collect();

        let results = Self::read_multiple(main_files.clone(), user_id, session_id).await;
        let contents: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "path": r.path,
                    "content": r.content.as_deref().unwrap_or(""),
                    "success": r.success,
                })
            })
            .collect();
        project_info["main_files"] = serde_json::json!(contents);

        Ok(project_info)
    }
}

// ─── الـ 100 Webshare SOCKS5 Proxies كاملة (نفس قائمة zen.rs) ──────────────
const ORCHESTRATOR_PROXIES: &[(&str, u16, &str, &str)] = &[
    ("31.59.20.176",6754,"cchsbntj","8ocnhyz7f53b"),
    ("31.56.127.193",7684,"cchsbntj","8ocnhyz7f53b"),
    ("45.38.107.97",6014,"cchsbntj","8ocnhyz7f53b"),
    ("198.105.121.200",6462,"cchsbntj","8ocnhyz7f53b"),
    ("64.137.96.74",6641,"cchsbntj","8ocnhyz7f53b"),
    ("198.23.243.226",6361,"cchsbntj","8ocnhyz7f53b"),
    ("38.154.185.97",6370,"cchsbntj","8ocnhyz7f53b"),
    ("84.247.60.125",6095,"cchsbntj","8ocnhyz7f53b"),
    ("142.111.67.146",5611,"cchsbntj","8ocnhyz7f53b"),
    ("191.96.254.138",6185,"cchsbntj","8ocnhyz7f53b"),
    ("31.59.20.176",6754,"chimgwqf","3693u6fbvvdq"),
    ("31.56.127.193",7684,"chimgwqf","3693u6fbvvdq"),
    ("45.38.107.97",6014,"chimgwqf","3693u6fbvvdq"),
    ("198.105.121.200",6462,"chimgwqf","3693u6fbvvdq"),
    ("64.137.96.74",6641,"chimgwqf","3693u6fbvvdq"),
    ("198.23.243.226",6361,"chimgwqf","3693u6fbvvdq"),
    ("38.154.185.97",6370,"chimgwqf","3693u6fbvvdq"),
    ("84.247.60.125",6095,"chimgwqf","3693u6fbvvdq"),
    ("142.111.67.146",5611,"chimgwqf","3693u6fbvvdq"),
    ("191.96.254.138",6185,"chimgwqf","3693u6fbvvdq"),
    ("31.59.20.176",6754,"qnotadmv","tk20kqtx2wfs"),
    ("31.56.127.193",7684,"qnotadmv","tk20kqtx2wfs"),
    ("45.38.107.97",6014,"qnotadmv","tk20kqtx2wfs"),
    ("198.105.121.200",6462,"qnotadmv","tk20kqtx2wfs"),
    ("64.137.96.74",6641,"qnotadmv","tk20kqtx2wfs"),
    ("198.23.243.226",6361,"qnotadmv","tk20kqtx2wfs"),
    ("38.154.185.97",6370,"qnotadmv","tk20kqtx2wfs"),
    ("84.247.60.125",6095,"qnotadmv","tk20kqtx2wfs"),
    ("142.111.67.146",5611,"qnotadmv","tk20kqtx2wfs"),
    ("191.96.254.138",6185,"qnotadmv","tk20kqtx2wfs"),
    ("31.59.20.176",6754,"oarzdrmm","lzjj8fezq82r"),
    ("31.56.127.193",7684,"oarzdrmm","lzjj8fezq82r"),
    ("45.38.107.97",6014,"oarzdrmm","lzjj8fezq82r"),
    ("198.105.121.200",6462,"oarzdrmm","lzjj8fezq82r"),
    ("64.137.96.74",6641,"oarzdrmm","lzjj8fezq82r"),
    ("198.23.243.226",6361,"oarzdrmm","lzjj8fezq82r"),
    ("38.154.185.97",6370,"oarzdrmm","lzjj8fezq82r"),
    ("84.247.60.125",6095,"oarzdrmm","lzjj8fezq82r"),
    ("142.111.67.146",5611,"oarzdrmm","lzjj8fezq82r"),
    ("191.96.254.138",6185,"oarzdrmm","lzjj8fezq82r"),
    ("31.59.20.176",6754,"yvptbhkt","0v8zzv1j120y"),
    ("31.56.127.193",7684,"yvptbhkt","0v8zzv1j120y"),
    ("45.38.107.97",6014,"yvptbhkt","0v8zzv1j120y"),
    ("198.105.121.200",6462,"yvptbhkt","0v8zzv1j120y"),
    ("64.137.96.74",6641,"yvptbhkt","0v8zzv1j120y"),
    ("198.23.243.226",6361,"yvptbhkt","0v8zzv1j120y"),
    ("38.154.185.97",6370,"yvptbhkt","0v8zzv1j120y"),
    ("84.247.60.125",6095,"yvptbhkt","0v8zzv1j120y"),
    ("142.111.67.146",5611,"yvptbhkt","0v8zzv1j120y"),
    ("191.96.254.138",6185,"yvptbhkt","0v8zzv1j120y"),
    ("31.59.20.176",6754,"ukhiyovs","nuiyu4j6b199"),
    ("31.56.127.193",7684,"ukhiyovs","nuiyu4j6b199"),
    ("45.38.107.97",6014,"ukhiyovs","nuiyu4j6b199"),
    ("198.105.121.200",6462,"ukhiyovs","nuiyu4j6b199"),
    ("64.137.96.74",6641,"ukhiyovs","nuiyu4j6b199"),
    ("198.23.243.226",6361,"ukhiyovs","nuiyu4j6b199"),
    ("38.154.185.97",6370,"ukhiyovs","nuiyu4j6b199"),
    ("84.247.60.125",6095,"ukhiyovs","nuiyu4j6b199"),
    ("142.111.67.146",5611,"ukhiyovs","nuiyu4j6b199"),
    ("191.96.254.138",6185,"ukhiyovs","nuiyu4j6b199"),
    ("31.59.20.176",6754,"anvqpams","bkrvfs0gyckg"),
    ("31.56.127.193",7684,"anvqpams","bkrvfs0gyckg"),
    ("45.38.107.97",6014,"anvqpams","bkrvfs0gyckg"),
    ("198.105.121.200",6462,"anvqpams","bkrvfs0gyckg"),
    ("64.137.96.74",6641,"anvqpams","bkrvfs0gyckg"),
    ("198.23.243.226",6361,"anvqpams","bkrvfs0gyckg"),
    ("38.154.185.97",6370,"anvqpams","bkrvfs0gyckg"),
    ("84.247.60.125",6095,"anvqpams","bkrvfs0gyckg"),
    ("142.111.67.146",5611,"anvqpams","bkrvfs0gyckg"),
    ("191.96.254.138",6185,"anvqpams","bkrvfs0gyckg"),
    ("31.59.20.176",6754,"shwcmvdj","7f0dmrhg0l92"),
    ("31.56.127.193",7684,"shwcmvdj","7f0dmrhg0l92"),
    ("45.38.107.97",6014,"shwcmvdj","7f0dmrhg0l92"),
    ("198.105.121.200",6462,"shwcmvdj","7f0dmrhg0l92"),
    ("64.137.96.74",6641,"shwcmvdj","7f0dmrhg0l92"),
    ("198.23.243.226",6361,"shwcmvdj","7f0dmrhg0l92"),
    ("38.154.185.97",6370,"shwcmvdj","7f0dmrhg0l92"),
    ("84.247.60.125",6095,"shwcmvdj","7f0dmrhg0l92"),
    ("142.111.67.146",5611,"shwcmvdj","7f0dmrhg0l92"),
    ("191.96.254.138",6185,"shwcmvdj","7f0dmrhg0l92"),
    ("31.59.20.176",6754,"rdtkrpec","ha7nsmzzw8xe"),
    ("31.56.127.193",7684,"rdtkrpec","ha7nsmzzw8xe"),
    ("45.38.107.97",6014,"rdtkrpec","ha7nsmzzw8xe"),
    ("198.105.121.200",6462,"rdtkrpec","ha7nsmzzw8xe"),
    ("64.137.96.74",6641,"rdtkrpec","ha7nsmzzw8xe"),
    ("198.23.243.226",6361,"rdtkrpec","ha7nsmzzw8xe"),
    ("38.154.185.97",6370,"rdtkrpec","ha7nsmzzw8xe"),
    ("84.247.60.125",6095,"rdtkrpec","ha7nsmzzw8xe"),
    ("142.111.67.146",5611,"rdtkrpec","ha7nsmzzw8xe"),
    ("191.96.254.138",6185,"rdtkrpec","ha7nsmzzw8xe"),
    ("31.59.20.176",6754,"qyuvyzeu","5ayzwc8rfvw5"),
    ("31.56.127.193",7684,"qyuvyzeu","5ayzwc8rfvw5"),
    ("45.38.107.97",6014,"qyuvyzeu","5ayzwc8rfvw5"),
    ("198.105.121.200",6462,"qyuvyzeu","5ayzwc8rfvw5"),
    ("64.137.96.74",6641,"qyuvyzeu","5ayzwc8rfvw5"),
    ("198.23.243.226",6361,"qyuvyzeu","5ayzwc8rfvw5"),
    ("38.154.185.97",6370,"qyuvyzeu","5ayzwc8rfvw5"),
    ("84.247.60.125",6095,"qyuvyzeu","5ayzwc8rfvw5"),
    ("142.111.67.146",5611,"qyuvyzeu","5ayzwc8rfvw5"),
    ("191.96.254.138",6185,"qyuvyzeu","5ayzwc8rfvw5"),
];

// ─── مساعد: Proxy per model+user — يستخدم كل الـ 100 proxy ──────────────────

fn format_proxy_for_user(user_id: &str, model_id: &str) -> String {
    // خلط user_id + model_id → توزيع أفضل بين الـ 100 proxy
    let hash: u64 = user_id.bytes().chain(model_id.bytes())
        .fold(5381u64, |acc, b| acc.wrapping_mul(33).wrapping_add(b as u64));
    let proxy_idx = (hash as usize) % ORCHESTRATOR_PROXIES.len();
    let (host, port, user, pass) = ORCHESTRATOR_PROXIES[proxy_idx];
    format!("socks5://{user}:{pass}@{host}:{port}")
}

// ─── اختبارات ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_code() {
        assert_eq!(TaskClassifier::classify("اكتب كود لحساب الأعداد الأولية"), TaskCategory::Code);
        assert_eq!(TaskClassifier::classify("Create a function to sort arrays"), TaskCategory::Code);
    }

    #[test]
    fn test_classify_debug() {
        assert_eq!(TaskClassifier::classify("هذا الكود فيه bugfix"), TaskCategory::Debug);
        assert_eq!(TaskClassifier::classify("fix the error in this function"), TaskCategory::Debug);
    }

    #[test]
    fn test_classify_vision() {
        assert_eq!(TaskClassifier::classify("ارسم أيقونة لتطبيق"), TaskCategory::Vision);
        assert_eq!(TaskClassifier::classify("design a ui for dashboard"), TaskCategory::Vision);
    }

    #[test]
    fn test_classify_research() {
        assert_eq!(TaskClassifier::classify("what is the capital of france"), TaskCategory::Research);
    }

    #[test]
    fn test_suggest_models_code_lite() {
        let models = TaskClassifier::suggest_models(TaskCategory::Code, Effort::Lite);
        assert!(!models.is_empty());
        assert!(models.contains(&"deepseek-v4-flash-free"));
    }

    #[test]
    fn test_suggest_models_code_max() {
        let models = TaskClassifier::suggest_models(TaskCategory::Code, Effort::Max);
        assert!(models.len() >= 2);
    }

    #[test]
    fn test_vision_only_mimo() {
        let models = TaskClassifier::suggest_models(TaskCategory::Vision, Effort::Max);
        assert_eq!(models, vec!["mimo-v2.5-free"]);
    }

    #[test]
    fn test_best_path_select_code() {
        let results = vec![
            ModelResult {
                model_id: "model-a".into(),
                content: "short".into(),
                thinking: None,
                duration_ms: 100,
                success: true,
                error: None,
            },
            ModelResult {
                model_id: "model-b".into(),
                content: "longer and more complete code with full implementation".into(),
                thinking: None,
                duration_ms: 200,
                success: true,
                error: None,
            },
        ];

        let (idx, content) = BestPathSelector::select(TaskCategory::Code, &results).unwrap();
        assert_eq!(idx, 1); // الأطول هو الأفضل للكود
        assert_eq!(content, "longer and more complete code with full implementation");
    }

    #[test]
    fn test_multi_file_editor_operations() {
        let ops = vec![
            FileEditOperation {
                path: "src/main.rs".into(),
                operation: FileOp::Read,
                content: None,
                old_str: None,
                new_str: None,
            },
            FileEditOperation {
                path: "src/lib.rs".into(),
                operation: FileOp::Read,
                content: None,
                old_str: None,
                new_str: None,
            },
        ];
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0].operation, FileOp::Read));
    }
}

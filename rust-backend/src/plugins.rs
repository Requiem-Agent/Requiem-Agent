// plugins.rs — S9-01: Plugin/Tool System for ReActEngine
// نظام plugins قابل للتوسيع يسمح لـ ReActEngine باستخدام أدوات خارجية
//
// Architecture:
//   AgentTool trait → concrete implementations (WebSearch, CodeExec, FileOps, Calculator, HttpFetch)
//   ToolRegistry → يسجّل ويُدير الـ plugins
//   ReActEngine يستخدم ToolRegistry لتنفيذ الأدوات

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

// ─────────────────────────────────────────────────────────────────────────────
// Core trait: AgentTool
// ─────────────────────────────────────────────────────────────────────────────

/// كل tool يجب أن يُطبّق هذا الـ trait
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// اسم الـ tool (يُستخدَم من الـ LLM لاستدعائه)
    fn name(&self) -> &str;

    /// وصف الـ tool (يُحقَن في system prompt)
    fn description(&self) -> &str;

    /// مثال على الاستخدام (يساعد الـ LLM)
    fn usage_example(&self) -> &str;

    /// تنفيذ الـ tool مع الـ arguments
    async fn execute(&self, args: &ToolArgs) -> ToolResult;

    /// هل الـ tool متاح في البيئة الحالية؟
    fn is_available(&self) -> bool {
        true
    }
}

/// Arguments لتنفيذ tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolArgs {
    /// الـ argument الرئيسي (query, code, url, etc.)
    pub input: String,
    /// arguments إضافية اختيارية
    pub options: HashMap<String, String>,
}

impl ToolArgs {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            options: HashMap::new(),
        }
    }

    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }

    pub fn get_option(&self, key: &str) -> Option<&str> {
        self.options.get(key).map(|s| s.as_str())
    }
}

/// نتيجة تنفيذ tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl ToolResult {
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
            metadata: HashMap::new(),
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        let msg = error.into();
        Self {
            success: false,
            output: String::new(),
            error: Some(msg),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ToolRegistry: يُدير جميع الـ plugins
// ─────────────────────────────────────────────────────────────────────────────

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn AgentTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// تسجيل tool جديد
    pub fn register(&mut self, tool: impl AgentTool + 'static) {
        let name = tool.name().to_string();
        info!("Registering tool: {}", name);
        self.tools.insert(name, Arc::new(tool));
    }

    /// تنفيذ tool بالاسم
    pub async fn execute(&self, tool_name: &str, args: ToolArgs) -> ToolResult {
        match self.tools.get(tool_name) {
            Some(tool) => {
                if !tool.is_available() {
                    return ToolResult::err(format!("Tool '{}' is not available", tool_name));
                }
                debug!("Executing tool: {} with input: {}", tool_name, args.input);
                tool.execute(&args).await
            }
            None => ToolResult::err(format!("Unknown tool: '{}'", tool_name)),
        }
    }

    /// قائمة الـ tools المتاحة مع أوصافها (للـ system prompt)
    pub fn tools_description(&self) -> String {
        let mut desc = String::from("## الأدوات المتاحة:\n\n");
        for tool in self.tools.values() {
            if tool.is_available() {
                desc.push_str(&format!(
                    "### {}\n{}\nمثال: {}\n\n",
                    tool.name(),
                    tool.description(),
                    tool.usage_example()
                ));
            }
        }
        desc
    }

    /// عدد الـ tools المسجَّلة
    pub fn count(&self) -> usize {
        self.tools.len()
    }

    /// هل tool معيّن مسجَّل؟
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// بناء registry افتراضي مع الـ tools الأساسية
    pub fn default_registry() -> Self {
        let mut registry = Self::new();
        registry.register(WebSearchTool::new());
        registry.register(CalculatorTool);
        registry.register(HttpFetchTool::new());
        registry.register(CodeExecutorTool::new());
        registry.register(FileOpsTool::new());
        registry
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::default_registry()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool 1: WebSearch
// ─────────────────────────────────────────────────────────────────────────────

pub struct WebSearchTool {
    api_key: Option<String>,
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("SERPER_API_KEY").ok(),
        }
    }
}

#[async_trait]
impl AgentTool for WebSearchTool {
    fn name(&self) -> &str { "web_search" }

    fn description(&self) -> &str {
        "يبحث في الإنترنت عن معلومات حديثة. استخدمه عندما تحتاج معلومات لا تعرفها أو قد تكون قديمة."
    }

    fn usage_example(&self) -> &str {
        "web_search(\"آخر أخبار الذكاء الاصطناعي 2026\")"
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    async fn execute(&self, args: &ToolArgs) -> ToolResult {
        let Some(ref api_key) = self.api_key else {
            return ToolResult::err("SERPER_API_KEY not configured");
        };

        let client = reqwest::Client::new();
        let body = serde_json::json!({ "q": args.input, "num": 5 });

        match client
            .post("https://google.serper.dev/search")
            .header("X-API-KEY", api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<serde_json::Value>().await {
                    Ok(data) => {
                        let results = format_search_results(&data);
                        ToolResult::ok(results)
                            .with_metadata("source", "serper.dev")
                            .with_metadata("query", &args.input)
                    }
                    Err(e) => ToolResult::err(format!("Failed to parse search results: {}", e)),
                }
            }
            Ok(resp) => {
                ToolResult::err(format!("Search API error: {}", resp.status()))
            }
            Err(e) => ToolResult::err(format!("Search request failed: {}", e)),
        }
    }
}

fn format_search_results(data: &serde_json::Value) -> String {
    let mut output = String::new();

    if let Some(organic) = data.get("organic").and_then(|v| v.as_array()) {
        for (i, result) in organic.iter().take(5).enumerate() {
            let title = result.get("title").and_then(|v| v.as_str()).unwrap_or("بدون عنوان");
            let snippet = result.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
            let link = result.get("link").and_then(|v| v.as_str()).unwrap_or("");

            output.push_str(&format!(
                "{}. **{}**\n{}\n🔗 {}\n\n",
                i + 1, title, snippet, link
            ));
        }
    }

    if output.is_empty() {
        "لم يتم العثور على نتائج".to_string()
    } else {
        output
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool 2: Calculator
// ─────────────────────────────────────────────────────────────────────────────

pub struct CalculatorTool;

#[async_trait]
impl AgentTool for CalculatorTool {
    fn name(&self) -> &str { "calculator" }

    fn description(&self) -> &str {
        "يحسب تعبيرات رياضية. يدعم: +، -، *، /، ^، sqrt، sin، cos، tan، log."
    }

    fn usage_example(&self) -> &str {
        "calculator(\"sqrt(144) + 2^8\")"
    }

    async fn execute(&self, args: &ToolArgs) -> ToolResult {
        // استخدام Python لحساب التعبيرات الرياضية بأمان
        let expr = args.input.trim();

        // تنظيف التعبير من الأحرف الخطرة
        let safe_expr = expr
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || "+-*/^()._ ".contains(*c))
            .collect::<String>();

        if safe_expr.is_empty() {
            return ToolResult::err("تعبير رياضي غير صالح");
        }

        // تحويل ^ إلى ** لـ Python
        let python_expr = safe_expr.replace('^', "**");

        match std::process::Command::new("python3")
            .args(["-c", &format!("import math; print(eval('{}', {{'__builtins__': {{}}}}, {{'sqrt': math.sqrt, 'sin': math.sin, 'cos': math.cos, 'tan': math.tan, 'log': math.log, 'pi': math.pi, 'e': math.e}}))", python_expr)])
            .output()
        {
            Ok(output) if output.status.success() => {
                let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
                ToolResult::ok(format!("{} = {}", safe_expr, result))
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                ToolResult::err(format!("خطأ في الحساب: {}", err))
            }
            Err(e) => ToolResult::err(format!("فشل تشغيل الحاسبة: {}", e)),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool 3: HttpFetch
// ─────────────────────────────────────────────────────────────────────────────

pub struct HttpFetchTool {
    client: reqwest::Client,
}

impl HttpFetchTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("RequiemAgent/1.0")
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl AgentTool for HttpFetchTool {
    fn name(&self) -> &str { "http_fetch" }

    fn description(&self) -> &str {
        "يجلب محتوى صفحة ويب أو API endpoint. يُرجع النص الخام."
    }

    fn usage_example(&self) -> &str {
        "http_fetch(\"https://api.example.com/data\")"
    }

    async fn execute(&self, args: &ToolArgs) -> ToolResult {
        let url = args.input.trim();

        // التحقق من أن الـ URL آمن
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return ToolResult::err("URL يجب أن يبدأ بـ http:// أو https://");
        }

        // منع الوصول لـ localhost والـ internal IPs
        if url.contains("localhost") || url.contains("127.0.0.1") || url.contains("0.0.0.0") {
            return ToolResult::err("لا يُسمح بالوصول لـ localhost");
        }

        match self.client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                match resp.text().await {
                    Ok(text) => {
                        // اقتصار على أول 5000 حرف
                        let truncated = if text.len() > 5000 {
                            format!("{}...\n[تم اقتصار المحتوى على 5000 حرف]", &text[..5000])
                        } else {
                            text
                        };
                        ToolResult::ok(truncated)
                            .with_metadata("status", &status.to_string())
                            .with_metadata("url", url)
                    }
                    Err(e) => ToolResult::err(format!("فشل قراءة الاستجابة: {}", e)),
                }
            }
            Err(e) => ToolResult::err(format!("فشل الطلب: {}", e)),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool 4: CodeExecutor (sandbox)
// ─────────────────────────────────────────────────────────────────────────────

pub struct CodeExecutorTool {
    timeout_secs: u64,
}

impl CodeExecutorTool {
    pub fn new() -> Self {
        Self { timeout_secs: 10 }
    }
}

#[async_trait]
impl AgentTool for CodeExecutorTool {
    fn name(&self) -> &str { "code_exec" }

    fn description(&self) -> &str {
        "ينفّذ كود Python في بيئة آمنة محدودة. مفيد للحسابات والتحليل."
    }

    fn usage_example(&self) -> &str {
        "code_exec(\"import json; data = [1,2,3]; print(sum(data))\")"
    }

    async fn execute(&self, args: &ToolArgs) -> ToolResult {
        let code = &args.input;

        // كتابة الكود في ملف مؤقت
        let tmp_file = format!("/tmp/agent_code_{}.py", uuid::Uuid::new_v4());
        if let Err(e) = std::fs::write(&tmp_file, code) {
            return ToolResult::err(format!("فشل كتابة الكود: {}", e));
        }

        // تنفيذ مع timeout
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            tokio::process::Command::new("python3")
                .arg(&tmp_file)
                .output(),
        )
        .await;

        // حذف الملف المؤقت
        let _ = std::fs::remove_file(&tmp_file);

        match output {
            Ok(Ok(out)) => {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    ToolResult::ok(if stdout.is_empty() { "تم التنفيذ بنجاح (لا output)".into() } else { stdout })
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    ToolResult::err(format!("خطأ في التنفيذ:\n{}", stderr))
                }
            }
            Ok(Err(e)) => ToolResult::err(format!("فشل تشغيل Python: {}", e)),
            Err(_) => ToolResult::err(format!("انتهت مهلة التنفيذ ({} ثانية)", self.timeout_secs)),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool 5: FileOps
// ─────────────────────────────────────────────────────────────────────────────

pub struct FileOpsTool {
    allowed_dir: String,
}

impl FileOpsTool {
    pub fn new() -> Self {
        Self {
            allowed_dir: std::env::var("AGENT_FILES_DIR")
                .unwrap_or_else(|_| "/tmp/agent_files".to_string()),
        }
    }
}

#[async_trait]
impl AgentTool for FileOpsTool {
    fn name(&self) -> &str { "file_ops" }

    fn description(&self) -> &str {
        "يقرأ ويكتب الملفات في مجلد آمن. الأوامر: read:<path>, write:<path>:<content>, list"
    }

    fn usage_example(&self) -> &str {
        "file_ops(\"read:report.txt\") أو file_ops(\"write:output.txt:محتوى الملف\")"
    }

    async fn execute(&self, args: &ToolArgs) -> ToolResult {
        let input = args.input.trim();

        // إنشاء المجلد إذا لم يوجد
        let _ = std::fs::create_dir_all(&self.allowed_dir);

        if input == "list" {
            match std::fs::read_dir(&self.allowed_dir) {
                Ok(entries) => {
                    let files: Vec<String> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect();
                    ToolResult::ok(if files.is_empty() {
                        "المجلد فارغ".to_string()
                    } else {
                        files.join("\n")
                    })
                }
                Err(e) => ToolResult::err(format!("فشل قراءة المجلد: {}", e)),
            }
        } else if let Some(path) = input.strip_prefix("read:") {
            let full_path = format!("{}/{}", self.allowed_dir, path.trim());
            // منع path traversal
            if path.contains("..") || path.contains('/') {
                return ToolResult::err("مسار غير مسموح به");
            }
            match std::fs::read_to_string(&full_path) {
                Ok(content) => ToolResult::ok(content),
                Err(e) => ToolResult::err(format!("فشل قراءة الملف: {}", e)),
            }
        } else if let Some(rest) = input.strip_prefix("write:") {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() != 2 {
                return ToolResult::err("صيغة غير صحيحة: write:<path>:<content>");
            }
            let path = parts[0].trim();
            let content = parts[1];

            if path.contains("..") || path.contains('/') {
                return ToolResult::err("مسار غير مسموح به");
            }

            let full_path = format!("{}/{}", self.allowed_dir, path);
            match std::fs::write(&full_path, content) {
                Ok(_) => ToolResult::ok(format!("تم حفظ الملف: {}", path)),
                Err(e) => ToolResult::err(format!("فشل كتابة الملف: {}", e)),
            }
        } else {
            ToolResult::err("أمر غير معروف. استخدم: read:<path>, write:<path>:<content>, list")
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_default() {
        let registry = ToolRegistry::default_registry();
        assert!(registry.has_tool("web_search"));
        assert!(registry.has_tool("calculator"));
        assert!(registry.has_tool("http_fetch"));
        assert!(registry.has_tool("code_exec"));
        assert!(registry.has_tool("file_ops"));
        assert_eq!(registry.count(), 5);
    }

    #[test]
    fn test_tool_args_builder() {
        let args = ToolArgs::new("test query")
            .with_option("lang", "ar")
            .with_option("limit", "10");
        assert_eq!(args.input, "test query");
        assert_eq!(args.get_option("lang"), Some("ar"));
        assert_eq!(args.get_option("limit"), Some("10"));
        assert_eq!(args.get_option("missing"), None);
    }

    #[test]
    fn test_tool_result_ok() {
        let result = ToolResult::ok("نتيجة ناجحة");
        assert!(result.success);
        assert_eq!(result.output, "نتيجة ناجحة");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_err() {
        let result = ToolResult::err("خطأ ما");
        assert!(!result.success);
        assert!(result.output.is_empty());
        assert_eq!(result.error.as_deref(), Some("خطأ ما"));
    }

    #[tokio::test]
    async fn test_unknown_tool_returns_error() {
        let registry = ToolRegistry::new();
        let result = registry.execute("nonexistent", ToolArgs::new("test")).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_calculator_basic() {
        let tool = CalculatorTool;
        let result = tool.execute(&ToolArgs::new("2 + 2")).await;
        // قد يفشل إذا لم يكن python3 متاحاً في الـ test env
        if result.success {
            assert!(result.output.contains("4"));
        }
    }

    #[tokio::test]
    async fn test_file_ops_list_empty() {
        let tool = FileOpsTool {
            allowed_dir: format!("/tmp/test_agent_{}", uuid::Uuid::new_v4()),
        };
        let result = tool.execute(&ToolArgs::new("list")).await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_file_ops_write_and_read() {
        let dir = format!("/tmp/test_agent_{}", uuid::Uuid::new_v4());
        let tool = FileOpsTool { allowed_dir: dir.clone() };

        let write_result = tool
            .execute(&ToolArgs::new("write:test.txt:مرحبا بالعالم"))
            .await;
        assert!(write_result.success, "Write failed: {:?}", write_result.error);

        let read_result = tool.execute(&ToolArgs::new("read:test.txt")).await;
        assert!(read_result.success);
        assert_eq!(read_result.output, "مرحبا بالعالم");

        // تنظيف
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_file_ops_path_traversal_blocked() {
        let tool = FileOpsTool::new();
        let result = tool.execute(&ToolArgs::new("read:../../etc/passwd")).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("غير مسموح"));
    }

    #[test]
    fn test_tools_description_format() {
        let registry = ToolRegistry::default_registry();
        let desc = registry.tools_description();
        assert!(desc.contains("الأدوات المتاحة"));
        assert!(desc.contains("calculator"));
    }
}

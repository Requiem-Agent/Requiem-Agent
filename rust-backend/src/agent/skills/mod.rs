//! # Agent Skills — مهارات الوكيل
//!
//! كل مهارة هي Rust trait يمكن حقنها في الوكيل:
//! - `CodeSkill` — كتابة وتحليل الكود
//! - `EnvironmentSkill` — فهم بيئة Requiem
//! - `DesignSkill` — تصميم واجهات وتطوير UI
//! - `ResearchSkill` — بحث وجمع معلومات
//! - `PlanningSkill` — تخطيط معماري
//! - `SecuritySkill` — فحص أمني

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// سياق تنفيذ المهارة
#[derive(Debug, Clone)]
pub struct SkillContext {
    pub user_id: String,
    pub task: String,
    pub available_tools: Vec<String>,
    pub available_models: Vec<String>,
    pub environment: serde_json::Value,
}

/// مخرجات المهارة
#[derive(Debug, Clone)]
pub struct SkillOutput {
    pub success: bool,
    pub result: serde_json::Value,
    pub artifacts: Vec<SkillArtifact>,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
}

/// قطعة أثرية من المهارة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillArtifact {
    pub kind: String,  // "code", "svg", "text", "json", "mermaid"
    pub content: String,
    pub label: String,
}

/// خطأ في المهارة
#[derive(Debug, Clone)]
pub struct SkillError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

/// trait المهارة — كل مهارة تنفذ هذا
pub trait AgentSkill: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn required_tools(&self) -> Vec<&'static str>;
    fn execute(&self, context: &SkillContext) -> Result<SkillOutput, SkillError>;
}

// ─── المهارات المدمجة ─────────────────────────────────────────────────────

/// مهارة البيئة — وعي الوكيل ببيئة Requiem
pub struct EnvironmentSkill;

impl AgentSkill for EnvironmentSkill {
    fn name(&self) -> &'static str { "environment_skill" }
    fn description(&self) -> &'static str { "وعي كامل ببيئة Requiem: نظام الملفات المعزول، Turso DB، الساندبوكس متعدد الطبقات، حدود الموارد، النماذج المتاحة" }
    fn required_tools(&self) -> Vec<&'static str> { vec!["file_tree", "shell"] }

    fn execute(&self, _context: &SkillContext) -> Result<SkillOutput, SkillError> {
        Ok(SkillOutput {
            success: true,
            result: serde_json::json!({
                "environment": {
                    "platform": "requiem-agent",
                    "sandbox": {
                        "layers": ["landlock", "seccomp", "rlimit", "user_ns"],
                        "max_processes": 50,
                        "max_memory_mb": 4096,
                        "network": false,
                    },
                    "storage": {
                        "type": "turso",
                        "isolation": "per-user",
                    },
                    "resources": {
                        "cpu": 2,
                        "ram_gb": 16,
                        "concurrent_sandboxes": 4,
                    },
                    "models": ["deepseek-v4-flash-free", "hy3-free", "mimo-v2.5-free",
                               "nemotron-3-ultra-free", "north-mini-code-free", "big-pickle"],
                }
            }),
            artifacts: vec![],
            duration_ms: 0,
            warnings: vec![],
        })
    }
}

/// مهارة البرمجة
pub struct CodeSkill;

impl AgentSkill for CodeSkill {
    fn name(&self) -> &'static str { "code_skill" }
    fn description(&self) -> &'static str { "كتابة وتعديل وتحليل الكود مع اختبارات وتحسين الجودة" }
    fn required_tools(&self) -> Vec<&'static str> {
        vec!["code_editor", "shell", "file_tree", "project_analyze"]
    }

    fn execute(&self, context: &SkillContext) -> Result<SkillOutput, SkillError> {
        Ok(SkillOutput {
            success: true,
            result: serde_json::json!({
                "workflow": [
                    "فهم المتطلبات",
                    "تحليل المشروع الحالي",
                    "كتابة الكود مع اختبارات",
                    "تشغيل الاختبارات",
                    "تصحيح الأخطاء",
                    "تحسين الجودة",
                ],
                "tools_available": self.required_tools(),
                "task": context.task,
            }),
            artifacts: vec![],
            duration_ms: 0,
            warnings: vec![],
        })
    }
}

/// مهارة التصميم
pub struct DesignSkill;

impl AgentSkill for DesignSkill {
    fn name(&self) -> &'static str { "design_skill" }
    fn description(&self) -> &'static str { "تصميم واجهات، SVG charts، رسوم بيانية، تخطيط UI/UX" }
    fn required_tools(&self) -> Vec<&'static str> { vec!["code_editor", "shell"] }

    fn execute(&self, context: &SkillContext) -> Result<SkillOutput, SkillError> {
        Ok(SkillOutput {
            success: true,
            result: serde_json::json!({
                "capabilities": [
                    "SVG charts (Bar, Line, Pie)",
                    "HTML/CSS تخطيط",
                    "تصميم متجاوب",
                    "ألوان وثيمات",
                ],
                "task": context.task,
            }),
            artifacts: vec![],
            duration_ms: 0,
            warnings: vec![],
        })
    }
}

// ─── Skill Registry ────────────────────────────────────────────────────────

/// سجل المهارات
pub struct SkillRegistry {
    skills: HashMap<&'static str, Box<dyn AgentSkill>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        let mut skills: HashMap<&'static str, Box<dyn AgentSkill>> = HashMap::new();
        skills.insert("code", Box::new(CodeSkill));
        skills.insert("design", Box::new(DesignSkill));
        skills.insert("environment", Box::new(EnvironmentSkill));
        Self { skills }
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn AgentSkill>> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<SkillInfo> {
        self.skills.iter().map(|(name, skill)| SkillInfo {
            name: name.to_string(),
            description: skill.description().to_string(),
            required_tools: skill.required_tools().iter().map(|t| t.to_string()).collect(),
        }).collect()
    }

    /// اقتراح المهارات المناسبة لمهمة
    pub fn suggest_for_task(&self, task: &str) -> Vec<String> {
        let lower = task.to_lowercase();
        let mut suggested = Vec::new();

        if lower.contains("code") || lower.contains("برمجة") || lower.contains("rust")
            || lower.contains("python") || lower.contains("javascript") || lower.contains("كود") {
            suggested.push("code".to_string());
        }
        if lower.contains("design") || lower.contains("تصميم") || lower.contains("ui")
            || lower.contains("svg") || lower.contains("chart") || lower.contains("واجهة") {
            suggested.push("design".to_string());
        }
        if lower.contains("بيئة") || lower.contains("environment") || lower.contains("sandbox")
            || lower.contains("system") || lower.contains("نظام") {
            suggested.push("environment".to_string());
        }

        if suggested.is_empty() {
            suggested.push("environment".to_string()); // المهارة الافتراضية
        }

        suggested
    }

    pub fn count(&self) -> usize { self.skills.len() }
}

/// معلومات المهارة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub required_tools: Vec<String>,
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_skills() {
        let reg = SkillRegistry::new();
        assert!(reg.count() >= 3);
        assert!(reg.get("code").is_some());
        assert!(reg.get("environment").is_some());
    }

    #[test]
    fn test_suggest_for_task() {
        let reg = SkillRegistry::new();
        let skills = reg.suggest_for_task("اكتب كود Rust");
        assert!(skills.contains(&"code".to_string()));

        let skills = reg.suggest_for_task("svG chart");
        assert!(skills.contains(&"design".to_string()));

        let skills = reg.suggest_for_task("normal question");
        assert!(skills.contains(&"environment".to_string()));
    }

    #[test]
    fn test_environment_skill_execute() {
        let skill = EnvironmentSkill;
        let ctx = SkillContext {
            user_id: "test".into(),
            task: "أخبرني عن البيئة".into(),
            available_tools: vec![],
            available_models: vec![],
            environment: serde_json::json!({}),
        };
        let output = skill.execute(&ctx).unwrap();
        assert!(output.success);
        assert!(output.result["environment"]["sandbox"]["layers"].is_array());
    }
}

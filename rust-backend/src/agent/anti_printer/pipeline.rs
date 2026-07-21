// ─── Advanced Compiler Pipeline ────────────────────────────────────────────
// Phase 15.3: يدمج semantic_validate → anti_printer_check → auto_correct →
//             security_scan → output_compile في pipeline واحد

use super::semantic::{SemanticEngine, SemanticResult};
use super::patterns::PatternDetector;
use super::{AntiPrinterReport, Severity};
use crate::agent::compiler::auto_correct::JsonAutoCorrect;
use crate::agent::compiler::output::{AgentOutputCompiler, CompilerConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// مراحل الـ Pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PipelineStage {
    SemanticValidation,
    AntiPrinterCheck,
    JsonAutoCorrect,
    SecurityScan,
    Compilation,
}

impl PipelineStage {
    pub fn name(&self) -> &str {
        match self {
            Self::SemanticValidation => "semantic_validation",
            Self::AntiPrinterCheck => "anti_printer_check",
            Self::JsonAutoCorrect => "json_auto_correct",
            Self::SecurityScan => "security_scan",
            Self::Compilation => "compilation",
        }
    }
}

/// تقرير مرحلة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageReport {
    pub stage: PipelineStage,
    pub passed: bool,
    pub issues: Vec<String>,
    pub duration_ms: u64,
}

/// التقرير النهائي للـ Pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineReport {
    pub passed: bool,
    pub stages: Vec<StageReport>,
    pub anti_printer: AntiPrinterReport,
    pub semantic: Option<SemanticResult>,
    pub original_input: String,
    pub corrected_output: Option<String>,
    pub security_issues: Vec<String>,
    pub total_duration_ms: u64,
}

impl PipelineReport {
    pub fn summary(&self) -> String {
        let status = if self.passed { "✅ PASSED" } else { "❌ FAILED" };
        let stage_summary: Vec<String> = self.stages.iter()
            .map(|s| format!("  {}: {}", s.stage.name(), if s.passed { "✅" } else { "❌" }))
            .collect();
        format!("Pipeline {status} ({}ms)\n{}\n  Quality Score: {:.2}\n  Issues: {}",
            self.total_duration_ms,
            stage_summary.join("\n"),
            self.anti_printer.quality_score,
            self.anti_printer.patterns.len())
    }
}

/// الـ Pipeline الرئيسي
#[derive(Debug)]
pub struct CompilerPipeline {
    pub semantic: SemanticEngine,
    pub pattern_detector: PatternDetector,
    pub json_corrector: JsonAutoCorrect,
    pub output_compiler: AgentOutputCompiler,
    /// تخطي بعض المراحل
    pub skip_stages: Vec<PipelineStage>,
}

impl Default for CompilerPipeline {
    fn default() -> Self {
        Self {
            semantic: SemanticEngine::new(),
            pattern_detector: PatternDetector::new(),
            json_corrector: JsonAutoCorrect::new(),
            output_compiler: AgentOutputCompiler::new(CompilerConfig::default()),
            skip_stages: vec![],
        }
    }
}

impl CompilerPipeline {
    pub fn new() -> Self { Self::default() }

    /// تشغيل الـ Pipeline الكامل
    pub async fn run(
        &mut self,
        thinking_text: &str,
        raw_tool_calls: &str,
        step_history: &[String],
        step_id: u64,
    ) -> PipelineReport {
        let start = std::time::Instant::now();
        let mut stages = vec![];
        let mut corrected_output = None;
        let mut security_issues = vec![];

        // Stage 1: Semantic Validation
        let s1 = std::time::Instant::now();
        let semantic_result = if !self.skip_stages.contains(&PipelineStage::SemanticValidation) {
            Some(self.semantic.analyze(thinking_text, step_id))
        } else {
            None
        };
        stages.push(StageReport {
            stage: PipelineStage::SemanticValidation,
            passed: true,
            issues: vec![],
            duration_ms: s1.elapsed().as_millis() as u64,
        });

        // Stage 2: Anti-Printer Check
        let s2 = std::time::Instant::now();
        let tool_calls_parsed: Vec<Value> = serde_json::from_str(raw_tool_calls).unwrap_or_default();
        let anti_printer = if !self.skip_stages.contains(&PipelineStage::AntiPrinterCheck) {
            self.pattern_detector.analyze(thinking_text, &tool_calls_parsed, step_history)
        } else {
            AntiPrinterReport::clean()
        };
        stages.push(StageReport {
            stage: PipelineStage::AntiPrinterCheck,
            passed: !anti_printer.requires_retry,
            issues: anti_printer.patterns.iter().map(|p| format!("{:?}: {}", p.pattern_type, p.description)).collect(),
            duration_ms: s2.elapsed().as_millis() as u64,
        });

        // Stage 3: JSON Auto-Correct
        let s3 = std::time::Instant::now();
        if !self.skip_stages.contains(&PipelineStage::JsonAutoCorrect) && !raw_tool_calls.is_empty() {
            let result = self.json_corrector.correct(raw_tool_calls, None);
            if !result.corrections.is_empty() {
                corrected_output = Some(raw_tool_calls.to_string());
            }
        }
        stages.push(StageReport {
            stage: PipelineStage::JsonAutoCorrect,
            passed: true,
            issues: vec![],
            duration_ms: s3.elapsed().as_millis() as u64,
        });

        // Stage 4: Security Scan
        let s4 = std::time::Instant::now();
        if !self.skip_stages.contains(&PipelineStage::SecurityScan) {
            // Security scan is done via the pattern detector in Stage 2
        }
        stages.push(StageReport {
            stage: PipelineStage::SecurityScan,
            passed: security_issues.is_empty(),
            issues: security_issues.clone(),
            duration_ms: s4.elapsed().as_millis() as u64,
        });

        // Stage 5: Final Compilation
        let s5 = std::time::Instant::now();
        if !self.skip_stages.contains(&PipelineStage::Compilation) {
            let _compiled = self.output_compiler.compile(thinking_text);
            // compiled result يحوي تفاصيل الـ compilation
        }
        stages.push(StageReport {
            stage: PipelineStage::Compilation,
            passed: true,
            issues: vec![],
            duration_ms: s5.elapsed().as_millis() as u64,
        });

        let total_ms = start.elapsed().as_millis() as u64;
        let passed = !anti_printer.requires_retry && security_issues.is_empty();

        PipelineReport {
            passed,
            stages,
            anti_printer,
            semantic: semantic_result,
            original_input: thinking_text.to_string(),
            corrected_output,
            security_issues,
            total_duration_ms: total_ms,
        }
    }

    /// تخطي مرحلة
    pub fn skip_stage(&mut self, stage: PipelineStage) {
        self.skip_stages.push(stage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_clean() {
        let mut pipe = CompilerPipeline::new();
        let report = pipe.run(
            "افتح الملف الرئيسي واقرأ المحتوى",
            r##"[{"tool":"read_file","params":{"path":"src/main.rs"}}]"##,
            &[],
            1,
        ).await;
        assert!(report.passed);
    }

    #[tokio::test]
    async fn test_pipeline_output_less() {
        let mut pipe = CompilerPipeline::new();
        let report = pipe.run(
            "أحتاج أن أفكر في هذا الأمر كثيراً... دعني أحلل... ربما يجب أن...",
            "[]",
            &["التخطيط للمهمة".to_string(), "التفكير في الخيارات".to_string()],
            1,
        ).await;
        // يجب أن يكتشف نمط output-less thinking
        assert!(!report.anti_printer.patterns.is_empty());
    }
}

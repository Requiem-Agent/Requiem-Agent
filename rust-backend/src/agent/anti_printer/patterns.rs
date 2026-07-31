// ─── Anti-Printer Pattern Detection ────────────────────────────────────────
// Phase 15.2: يكشف 6 أنماط طباعة فارغة في تفكير وأفعال الوكيل

use super::{DetectedPattern, PatternType, Severity, AntiPrinterReport};
use serde_json::Value;

/// مدقق الأنماط
#[derive(Debug, Clone)]
pub struct PatternDetector {
    /// عتبة تكرار الكلمات للكشف عن التكرار
    repetition_threshold: usize,
    /// الحد الأقصى لخطوات التخطيط بدون تنفيذ
    max_plan_only_steps: usize,
    /// الحد الأدنى لطول الرد لاعتباره سطحيًا
    shallow_response_max_len: usize,
    /// كلمات ممنوعة للـ verbosity
    verbose_words: Vec<&'static str>,
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self {
            repetition_threshold: 3,
            max_plan_only_steps: 2,
            shallow_response_max_len: 50,
            verbose_words: vec![
                "essentially", "basically", "actually", "literally",
                "virtually", "practically", "simply", "just", "very",
                "really", "quite", "somewhat", "rather", "pretty",
                "definitely", "absolutely", "undoubtedly", "certainly",
            ],
        }
    }
}

impl PatternDetector {
    pub fn new() -> Self { Self::default() }

    /// فحص شامل للنص
    pub fn analyze(&self, thinking_text: &str, tool_calls: &[Value], step_history: &[String]) -> AntiPrinterReport {
        let mut patterns = vec![];

        // 1. Output-Less Thinking
        self.check_output_less(thinking_text, tool_calls, &mut patterns);

        // 2. Circular Reasoning
        self.check_circular(thinking_text, step_history, &mut patterns);

        // 3. Shallow Response
        self.check_shallow(thinking_text, &mut patterns);

        // 4. Verbose No Action
        self.check_verbose(thinking_text, tool_calls, &mut patterns);

        // 5. Plan-Only Loop
        self.check_plan_only(step_history, &mut patterns);

        // 6. Repetitive Content
        self.check_repetitive(thinking_text, &mut patterns);

        let has_issues = !patterns.is_empty();
        let quality_score = self.compute_score(patterns.len(), &patterns);
        let requires_retry = patterns.iter().any(|p| matches!(p.severity, Severity::Error | Severity::Critical));

        AntiPrinterReport {
            has_issues,
            patterns,
            quality_score,
            requires_retry,
            suggested_action: if requires_retry { "retry_with_correction" } else { "proceed" }.to_string(),
        }
    }

    /// 1. تفكير بدون إخراج: الوكيل يفكر لكن لا ينفذ أداة
    fn check_output_less(&self, text: &str, tools: &[Value], patterns: &mut Vec<DetectedPattern>) {
        if text.len() > 100 && tools.is_empty() {
            patterns.push(DetectedPattern {
                pattern_type: PatternType::OutputLessThinking,
                description: "تفكير مطول بدون أي تنفيذ أداة. الوكيل يجب أن يتخذ إجراءً.".to_string(),
                severity: Severity::Error,
                location: Some("step".to_string()),
                suggestion: "نفّذ أداة فوراً: إما read_file / write_file / run_command / search".to_string(),
            });
        }
    }

    /// 2. تفكير دائري: نفس المعلومة تتكرر
    fn check_circular(&self, text: &str, history: &[String], patterns: &mut Vec<DetectedPattern>) {
        if history.len() < 2 { return; }
        let recent: Vec<&str> = history.iter().rev().take(3).map(|s| s.as_str()).collect();
        if recent.len() >= 2 {
            let similarity = self.jaccard_similarity(text, recent[0]);
            if similarity > 0.65 {
                patterns.push(DetectedPattern {
                    pattern_type: PatternType::CircularReasoning,
                    description: format!("تفكير دائري: تشابه {:.0}% مع الخطوة السابقة. الوكيل عالق في حلقة.", similarity * 100.0),
                    severity: Severity::Warning,
                    location: Some("context_switch".to_string()),
                    suggestion: "غيّر نهج التحليل أو انتقل للتنفيذ مباشرة".to_string(),
                });
            }
        }
    }

    /// 3. ردود سطحية: قصيرة جداً أو عامة بدون تحليل
    fn check_shallow(&self, text: &str, patterns: &mut Vec<DetectedPattern>) {
        let stripped = text.trim();
        if stripped.len() < self.shallow_response_max_len && !stripped.is_empty() {
            patterns.push(DetectedPattern {
                pattern_type: PatternType::ShallowResponse,
                description: format!("رد سطحي ({} حرف فقط). الوكيل لم يقدم تحليلاً كافياً.", stripped.len()),
                severity: Severity::Warning,
                location: None,
                suggestion: "وسّع التحليل: اشرح المنطق، اذكر الخيارات، قدّم دليلاً".to_string(),
            });
        }
    }

    /// 4. إسهاب بدون فعل: كلام كثير مع كلمات حشو وأفعال قليلة
    fn check_verbose(&self, text: &str, tools: &[Value], patterns: &mut Vec<DetectedPattern>) {
        let word_count = text.split_whitespace().count();
        let verbose_count = self.verbose_words.iter().filter(|w| text.contains(*w)).count();
        if word_count > 100 && verbose_count > 3 && tools.is_empty() {
            patterns.push(DetectedPattern {
                pattern_type: PatternType::VerboseNoAction,
                description: format!("إسهاب ({} كلمة) مع {} كلمة حشو بدون أي فعل تنفيذي.", word_count, verbose_count),
                severity: Severity::Warning,
                location: Some(format!("{} words", word_count)),
                suggestion: "خفّف الكلام الزائد ونفّذ الخطوة التالية مباشرة".to_string(),
            });
        }
    }

    /// 5. تخطيط فقط بدون تنفيذ: خطط متتالية بدون أداة
    fn check_plan_only(&self, history: &[String], patterns: &mut Vec<DetectedPattern>) {
        let plan_keywords = ["plan", "خطة", "سأفعل", "أخطط", "first", "then", "next", "finally"];
        let plan_steps: Vec<&String> = history.iter()
            .filter(|s| plan_keywords.iter().any(|k| s.contains(k)))
            .collect();
        if plan_steps.len() >= self.max_plan_only_steps {
            patterns.push(DetectedPattern {
                pattern_type: PatternType::PlanOnlyLoop,
                description: format!("{} خطط متتالية بدون تنفيذ. الوكيل يخطط فقط ولا ينفذ.", plan_steps.len()),
                severity: Severity::Error,
                location: Some(format!("{} plan steps", plan_steps.len())),
                suggestion: "نفّذ الخطوة الأولى الآن بدلاً من التخطيط لها مجدداً".to_string(),
            });
        }
    }

    /// 6. محتوى متكرر: تكرار نفس العبارات
    fn check_repetitive(&self, text: &str, patterns: &mut Vec<DetectedPattern>) {
        let sentences: Vec<&str> = text.split(|c: char| c == '.' || c == '!' || c == '؟').collect();
        if sentences.len() < 3 { return; }
        for (i, s1) in sentences.iter().enumerate() {
            for s2 in sentences.iter().skip(i + 1) {
                if s1.trim().len() > 10 && self.jaccard_similarity(s1, s2) > 0.7 {
                    patterns.push(DetectedPattern {
                        pattern_type: PatternType::RepetitiveContent,
                        description: "محتوى متكرر: جمل متطابقة تقريباً تتكرر في نفس الرسالة.".to_string(),
                        severity: Severity::Info,
                        location: Some(format!("sentence {}..{}", i, i + 1)),
                        suggestion: "أزل التكرار ووحّد الجمل المتشابهة".to_string(),
                    });
                    return;
                }
            }
        }
    }

    // ─── Utility ─────────────────────────────────────────────────────────

    /// معامل تشابه Jaccard بين نصين
    fn jaccard_similarity(&self, a: &str, b: &str) -> f64 {
        let set_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let set_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();
        if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
    }

    fn compute_score(&self, pattern_count: usize, patterns: &[DetectedPattern]) -> f64 {
        if pattern_count == 0 { return 1.0; }
        let mut penalty = pattern_count as f64 * 0.15;
        for p in patterns {
            penalty += match p.severity {
                Severity::Critical => 0.4,
                Severity::Error => 0.25,
                Severity::Warning => 0.1,
                Severity::Info => 0.05,
            };
        }
        (1.0 - penalty).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_less_thinking() {
        let d = PatternDetector::default();
        let report = d.analyze("هذا تحليل طويل جداً بدون أي أداة...", &[], &[]);
        assert!(report.has_issues);
        assert!(report.patterns.iter().any(|p| p.pattern_type == PatternType::OutputLessThinking));
    }

    #[test]
    fn test_clean_report() {
        let d = PatternDetector::default();
        let report = d.analyze("هذا أمر بفتح الملف", &[serde_json::json!({"tool": "read_file"})], &["فكر"]);
        assert!(!report.has_issues);
        assert_eq!(report.quality_score, 1.0);
    }
}

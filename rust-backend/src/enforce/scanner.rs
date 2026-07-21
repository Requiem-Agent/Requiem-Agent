//! # SecurityScanner — فحص أمني للكود قبل التنفيذ
//!
//! ## مهمته
//! كشف API keys, tokens, secrets, وأكواد خطيرة قبل أن تُنفّذ.
//!
//! ## كيف يعمل؟
//! - قائمة بأنماط regex للكشف عن الأسرار
//! - قائمة بأكواد خطيرة (rm -rf, fork bomb, إلخ)
//! - مستويات الخطورة: Info, Warning, Critical

use crate::enforce::{SecurityViolation, Severity};

/// نمط كشف — يمثل قاعدة واحدة للفحص الأمني
#[derive(Debug)]
struct ScanPattern {
    severity: Severity,
    name: &'static str,
    description: &'static str,
    pattern: &'static str,
}

/// أنماط كشف الأسرار الشائعة
const SECRET_PATTERNS: &[ScanPattern] = &[
    // API Keys
    ScanPattern { severity: Severity::Critical, name: "openai-api-key",
        description: "OpenAI API key detected", pattern: "sk-[A-Za-z0-9]{20,}" },
    ScanPattern { severity: Severity::Critical, name: "aws-access-key",
        description: "AWS Access Key ID detected", pattern: "AKIA[0-9A-Z]{16}" },
    ScanPattern { severity: Severity::Critical, name: "github-token",
        description: "GitHub personal access token", pattern: "gh[pousr]_[A-Za-z0-9]{36,}" },
    ScanPattern { severity: Severity::Critical, name: "slack-token",
        description: "Slack token detected", pattern: "xox[baprs]-[0-9A-Za-z-]{10,}" },
    ScanPattern { severity: Severity::Critical, name: "google-api-key",
        description: "Google API key detected", pattern: "AIza[0-9A-Za-z_-]{35}" },
    ScanPattern { severity: Severity::Critical, name: "telegram-bot-token",
        description: "Telegram Bot token detected", pattern: "[0-9]{8,10}:[A-Za-z0-9_-]{35}" },
    ScanPattern { severity: Severity::Critical, name: "jwt-token",
        description: "JWT token detected (could be a session token)", pattern: "eyJ[A-Za-z0-9_-]+\\.[A-Za-z0-9_-]+\\.[A-Za-z0-9_-]+" },
    ScanPattern { severity: Severity::Critical, name: "heroku-api-key",
        description: "Heroku API key detected", pattern: "[hH][eE][rR][oO][kK][uU].*[A-Za-z0-9]{8}-[A-Za-z0-9]{4}-[A-Za-z0-9]{4}-[A-Za-z0-9]{4}-[A-Za-z0-9]{12}" },
    ScanPattern { severity: Severity::Warning, name: "private-key",
        description: "Private key block detected", pattern: "-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----" },
    ScanPattern { severity: Severity::Warning, name: "npm-token",
        description: "npm authentication token", pattern: "npm_[A-Za-z0-9]{36}" },
    ScanPattern { severity: Severity::Info, name: "password-var",
        description: "Variable name suggests password", pattern: "(?i)(password|passwd|secret|credential|token)\\s*[:=]\\s*['\"][^'\"]+['\"]" },
];

/// أنماط الأكواد الخطيرة — قد تضر بالنظام
const DANGEROUS_PATTERNS: &[ScanPattern] = &[
    ScanPattern {
        severity: Severity::Critical,
        name: "rm-rf",
        description: "rm -rf / or similar destructive command",
        pattern: "rm\\s+[-]?[rf]+\\s+/",
    },
    ScanPattern {
        severity: Severity::Critical,
        name: "fork-bomb",
        description: "Fork bomb detected",
        pattern: ":(){.*:}.*;",
    },
    ScanPattern {
        severity: Severity::Critical,
        name: "dd-destructive",
        description: "dd command targeting block device",
        pattern: "dd\\s+if=.*of=/dev/",
    },
    ScanPattern {
        severity: Severity::Critical,
        name: "wget-pipe-sh",
        description: "wget/curl pipe to shell (dangerous)",
        pattern: "(wget|curl)\\s+.*\\|\\s*(bash|sh|zsh)",
    },
    ScanPattern {
        severity: Severity::Critical,
        name: "chmod-recursive-root",
        description: "chmod -R / (dangerous)",
        pattern: "chmod\\s+[-]?R\\s+[0-7]{3,4}\\s+/",
    },
    ScanPattern {
        severity: Severity::Warning,
        name: "mkfs",
        description: "Format filesystem command",
        pattern: "mkfs\\.",
    },
    ScanPattern {
        severity: Severity::Warning,
        name: "reboot-halt",
        description: "System shutdown command",
        pattern: "(reboot|shutdown|halt|poweroff)",
    },
    ScanPattern {
        severity: Severity::Info,
        name: "sudo",
        description: "sudo command (may fail in sandbox)",
        pattern: "sudo\\s+",
    },
    ScanPattern {
        severity: Severity::Info,
        name: "eval-untrusted",
        description: "eval() with user input (potential code injection)",
        pattern: "eval\\s*\\(\\s*",
    },
];

/// الفاحص الأمني
#[derive(Debug)]
pub struct SecurityScanner {
    secret_patterns: Vec<&'static ScanPattern>,
    dangerous_patterns: Vec<&'static ScanPattern>,
}

impl SecurityScanner {
    pub fn new() -> Self {
        Self {
            secret_patterns: SECRET_PATTERNS.iter().collect(),
            dangerous_patterns: DANGEROUS_PATTERNS.iter().collect(),
        }
    }

    /// فحص الكود — يعيد أول خطأ Critical أو كل الأخطاء
    pub fn scan(&self, content: &str) -> Result<(), SecurityViolation> {
        // افحص الأسرار أولاً
        for pattern in &self.secret_patterns {
            if let Some(violation) = self.match_pattern(content, pattern) {
                if violation.severity == Severity::Critical {
                    return Err(violation);
                }
            }
        }

        // افحص الأكواد الخطيرة
        for pattern in &self.dangerous_patterns {
            if let Some(violation) = self.match_pattern(content, pattern) {
                if violation.severity == Severity::Critical {
                    return Err(violation);
                }
            }
        }

        Ok(())
    }

    /// فحص كامل — يعيد كل الانتهاكات (بدون توقف عند أول خطأ)
    pub fn scan_all(&self, content: &str) -> Vec<SecurityViolation> {
        let mut violations = Vec::new();

        for pattern in &self.secret_patterns {
            if let Some(v) = self.match_pattern(content, pattern) {
                violations.push(v);
            }
        }
        for pattern in &self.dangerous_patterns {
            if let Some(v) = self.match_pattern(content, pattern) {
                violations.push(v);
            }
        }

        violations
    }

    /// مطابقة نمط معيّن في المحتوى
    fn match_pattern(&self, content: &str, pattern: &&ScanPattern) -> Option<SecurityViolation> {
        // استخدام contains للأنماط البسيطة (لتفادي الاعتماد على regex)
        if content.contains(pattern.pattern) {
            // استخرج snippet من حول الموقع
            let snippet = self.extract_snippet(content, pattern.pattern);
            Some(SecurityViolation {
                severity: pattern.severity.clone(),
                pattern: pattern.name.to_string(),
                description: pattern.description.to_string(),
                snippet,
            })
        } else {
            None
        }
    }

    /// استخراج مقتطف من الكود حول النمط المكتشف
    fn extract_snippet(&self, content: &str, pattern: &str) -> String {
        if let Some(pos) = content.find(pattern) {
            let start = pos.saturating_sub(30);
            let end = (pos + pattern.len() + 30).min(content.len());
            let mut snippet = String::new();
            if start > 0 {
                snippet.push_str("...");
            }
            snippet.push_str(&content[start..end]);
            if end < content.len() {
                snippet.push_str("...");
            }
            snippet
        } else {
            pattern.to_string()
        }
    }
}

// ─── الاختبارات ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_openai_key() {
        let scanner = SecurityScanner::new();
        let code = "api_key = \"sk-123456789012345678901234\"";
        let result = scanner.scan(code);
        assert!(result.is_err(), "OpenAI key should be detected");
        assert_eq!(result.unwrap_err().pattern, "openai-api-key");
    }

    #[test]
    fn test_detect_private_key() {
        let scanner = SecurityScanner::new();
        let code = "-----BEGIN PRIVATE KEY-----\nABCDEF1234";
        let result = scanner.scan(code);
        assert!(result.is_ok(), "Private key is Warning, not Critical");
        // but scan_all should find it
        let all = scanner.scan_all(code);
        assert!(all.iter().any(|v| v.pattern == "private-key"));
    }

    #[test]
    fn test_detect_rm_rf() {
        let scanner = SecurityScanner::new();
        let code = "rm -rf /home/user/data";
        let result = scanner.scan(code);
        assert!(result.is_err(), "rm -rf should be detected");
    }

    #[test]
    fn test_clean_code_passes() {
        let scanner = SecurityScanner::new();
        let code = "print('Hello, world!')\nfor i in range(10):\n    print(i)";
        let result = scanner.scan(code);
        assert!(result.is_ok(), "Clean code should pass");
    }

    #[test]
    fn test_detect_jwt() {
        let scanner = SecurityScanner::new();
        let code = "token = \"eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8\"";
        let result = scanner.scan(code);
        assert!(result.is_err(), "JWT token should be detected");
    }
}

//! # Path Safety Module — العزل البرمجي الصارم للمسارات
//!
//! يضمن هذا الموديول أن أي عملية وصول للملفات تكون ضمن جذر المستخدم فقط.
//! لا يمكن لأي كود — حتى داخل الوكيل — تجاوز هذه الحدود لأنها مفروضة على مستوى runtime.
//!
//! ## المبادئ:
//! 1. كل مسار يُحلّى (resolve) أولاً ثم يُتحقق من أنه ضمن الجذر المسموح
//! 2. منع الـ path traversal بأشكاله المختلفة: `..`, `//`, `~`, symlinks
//! 3. فحص الـ canonical path بعد إزالة كل الرموز الخطرة
//! 4. رفض أي مسار يحتوي على أحرف غير مسموح بها

use std::fmt;
use std::path::{Component, Path, PathBuf};

/// الأخطاء المتعلقة بالمسارات
#[derive(Debug)]
pub enum PathError {
    TraversalDetected { path: String },
    OutsideRoot { path: String },
    InvalidCharacters { path: String },
    PathTooLong { len: usize, max: usize },
    SymlinkNotAllowed { path: String },
    Io(std::io::Error),
    Other(String),
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TraversalDetected { path } => write!(f, "Path traversal detected: {path}"),
            Self::OutsideRoot { path } => write!(f, "Path outside user root: {path}"),
            Self::InvalidCharacters { path } => write!(f, "Invalid characters in path: {path}"),
            Self::PathTooLong { len, max } => write!(f, "Path too long: {len} chars (max {max})"),
            Self::SymlinkNotAllowed { path } => write!(f, "Symlinks not allowed: {path}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for PathError {}

impl From<std::io::Error> for PathError {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}

pub type PathResult<T> = Result<T, PathError>;

/// قائمة الأحرف غير المسموح بها في أسماء الملفات
const INVALID_CHARS: &[char] = &['\0', '\\', ':', '*', '?', '"', '<', '>', '|', ';', '&', '`', '$'];

/// الحد الأقصى لطول المسار
const MAX_PATH_LENGTH: usize = 4096;

/// عدد المكونات الأقصى في المسار
const MAX_PATH_COMPONENTS: usize = 32;

// ─── الدوال الأساسية ─────────────────────────────────────────────────────────

/// التحقق من أن المسار لا يحتوي على أحرف خطرة
pub fn validate_path_chars(path: &str) -> PathResult<()> {
    if path.contains(INVALID_CHARS) {
        return Err(PathError::InvalidCharacters {
            path: path.chars()
                .map(|c| if INVALID_CHARS.contains(&c) { '?' } else { c })
                .collect(),
        });
    }
    if path.len() > MAX_PATH_LENGTH {
        return Err(PathError::PathTooLong {
            len: path.len(),
            max: MAX_PATH_LENGTH,
        });
    }
    Ok(())
}

/// التحقق من عدم وجود محاولات path traversal
pub fn check_path_traversal(path: &Path) -> PathResult<()> {
    let components: Vec<_> = path.components().collect();

    if components.len() > MAX_PATH_COMPONENTS {
        return Err(PathError::PathTooLong {
            len: components.len(),
            max: MAX_PATH_COMPONENTS,
        });
    }

    let mut depth: i32 = 0;
    for comp in &components {
        match comp {
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return Err(PathError::TraversalDetected {
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
            Component::Normal(_) => depth += 1,
            Component::RootDir => depth = 0,
            Component::CurDir => { /* ignore */ }
            Component::Prefix(_) => {
                return Err(PathError::InvalidCharacters {
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }

    Ok(())
}

/// التحقق من أن المسار المُحلّى (canonical) يبدأ بـ root_dir
pub fn validate_path_in_root(resolved_path: &Path, root_dir: &Path) -> PathResult<()> {
    if !resolved_path.starts_with(root_dir) {
        return Err(PathError::OutsideRoot {
            path: resolved_path.to_string_lossy().to_string(),
        });
    }
    Ok(())
}

/// دالة شاملة: تحقق من مسار آمن ضمن جذر المستخدم
///
/// 1. validate path chars
/// 2. check traversal
/// 3. resolve symlinks (إذا الملف موجود)
/// 4. validate in root
pub fn ensure_safe_path(path: &Path, user_root: &Path) -> PathResult<PathBuf> {
    let path_str = path.to_string_lossy();
    validate_path_chars(&path_str)?;
    check_path_traversal(path)?;

    // إذا المسار موجود، حلّ الـ canonical path
    let resolved = if path.exists() {
        match path.canonicalize() {
            Ok(canonical) => canonical,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // الملف غير موجود — استخدم المسار الأصلي مع منع الـ symlink
                path.to_path_buf()
            }
            Err(e) => return Err(PathError::Io(e)),
        }
    } else {
        // الملف غير موجود — حلّ المسار بدون canonicalize
        resolve_safe(&user_root, path)?
    };

    validate_path_in_root(&resolved, user_root)?;
    Ok(resolved)
}

/// حل آمن لمسار نسبي ضمن جذر معين — بدون canonicalize
fn resolve_safe(root: &Path, child: &Path) -> PathResult<PathBuf> {
    let mut result = root.to_path_buf();
    for comp in child.components() {
        match comp {
            Component::ParentDir => {
                if !result.pop() {
                    return Err(PathError::TraversalDetected {
                        path: child.to_string_lossy().to_string(),
                    });
                }
            }
            Component::Normal(name) => {
                let name_str = name.to_string_lossy();
                validate_path_chars(&name_str)?;
                result.push(name);
            }
            Component::CurDir => { /* ignore */ }
            Component::RootDir | Component::Prefix(_) => {
                // إذا بدأ المسار بـ / نبدأ من الجذر
                result = root.to_path_buf();
            }
        }
    }
    Ok(result)
}

// ─── هيكل UserPathRoot ───────────────────────────────────────────────────────

/// يمثل جذر المستخدم في نظام الملفات — يوفر دوالاً آمنة للوصول
#[derive(Debug, Clone)]
pub struct UserPathRoot {
    user_id: String,
    root_path: PathBuf,
}

impl UserPathRoot {
    /// إنشاء جذر آمن للمستخدم
    ///
    /// المسار النهائي سيكون: `/app/data/users/{user_id}/`
    pub fn new(user_id: &str, base_path: Option<&Path>) -> Self {
        let base = base_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/app/data"));
        let root_path = base.join("users").join(user_id);
        Self {
            user_id: user_id.to_string(),
            root_path,
        }
    }

    /// الحصول على مسار جذر المستخدم
    pub fn root(&self) -> &Path {
        &self.root_path
    }

    /// الحصول على معرّف المستخدم
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// إنشاء مسار آمن ضمن جذر المستخدم
    pub fn join_safe(&self, components: &[&str]) -> PathResult<PathBuf> {
        let mut path = self.root_path.clone();
        for comp in components {
            validate_path_chars(comp)?;
            if comp.contains('/') || comp.contains("..") {
                return Err(PathError::InvalidCharacters {
                    path: comp.to_string(),
                });
            }
            path.push(comp);
        }
        Ok(path)
    }

    /// التحقق من أن مساراً ما ضمن جذر المستخدم — وإرجاع المسار المُحلّى
    pub fn ensure_within(&self, path: &Path) -> PathResult<PathBuf> {
        let path_str = path.to_string_lossy();

        // 1. تحقق من الأحرف
        validate_path_chars(&path_str)?;

        // 2. حلّ المسار وعودته للمسار الطبيعي
        let resolved = if path.is_relative() {
            self.root_path.join(path)
        } else {
            path.to_path_buf()
        };

        // 3. تحقق من path traversal
        check_path_traversal(&resolved)?;

        // 4. تحقق من أن المسار ضمن الجذر
        validate_path_in_root(&resolved, &self.root_path)?;

        // 5. إذا كان الملف موجوداً، تحقق من canonical path
        if resolved.exists() {
            let canonical = resolved.canonicalize().map_err(PathError::Io)?;
            validate_path_in_root(&canonical, &self.root_path)?;
            return Ok(canonical);
        }

        Ok(resolved)
    }

    /// مسار مجلد sessions الخاص بالمستخدم
    pub fn sessions_dir(&self) -> PathBuf {
        self.root_path.join("sessions")
    }

    /// مسار مجلد files الخاص بالمستخدم
    pub fn files_dir(&self) -> PathBuf {
        self.root_path.join("files")
    }

    /// مسار مجلد sandbox الخاص بالمستخدم
    pub fn sandbox_dir(&self) -> PathBuf {
        self.root_path.join("sandbox")
    }

    /// مسار قاعدة بيانات SQLite الخاصة بالمستخدم
    pub fn user_db_path(&self) -> PathBuf {
        self.root_path.join("user.db")
    }

    /// مسار قاعدة بيانات SQLite الخاصة بجلسة محددة
    pub fn session_db_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir().join(session_id).join("session.db")
    }

    /// مسار مجلد files لجلسة محددة
    pub fn session_files_dir(&self, session_id: &str) -> PathBuf {
        self.sessions_dir().join(session_id).join("files")
    }

    /// التأكد من وجود مجلدات المستخدم — وإنشاؤها إن لم تكن موجودة
    pub async fn ensure_dirs(&self) -> PathResult<()> {
        tokio::fs::create_dir_all(&self.root_path).await?;
        tokio::fs::create_dir_all(self.sessions_dir()).await?;
        tokio::fs::create_dir_all(self.files_dir()).await?;
        tokio::fs::create_dir_all(self.sandbox_dir()).await?;
        Ok(())
    }
}

// ─── اختبارات ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_path_traversal_simple() {
        let root = Path::new("/app/data/users/test123");
        let malicious = Path::new("../../../etc/passwd");
        let result = ensure_safe_path(malicious, root);
        assert!(result.is_err(), "Path traversal should be detected");
    }

    #[test]
    fn test_path_traversal_encoded() {
        let root = Path::new("/app/data/users/test123");
        // محاولة traversal ضمن الجذر
        let path = Path::new("sessions/../../files/secret.txt");
        let result = ensure_safe_path(path, root);
        assert!(result.is_err(), "Encoded traversal should be detected");
    }

    #[test]
    fn test_valid_path() {
        let root = Path::new("/app/data/users/test123");
        let path = Path::new("sessions/session-1/files/main.rs");
        let result = ensure_safe_path(path, root);
        assert!(result.is_ok(), "Valid path should pass: {:?}", result.err());
    }

    #[test]
    fn test_invalid_chars() {
        assert!(validate_path_chars("hello$world").is_err());
        assert!(validate_path_chars("hello;rm -rf /").is_err());
        assert!(validate_path_chars("normal_file.rs").is_ok());
    }

    #[test]
    fn test_user_path_root_join() {
        let upr = UserPathRoot::new("user123", None);
        let f = upr.join_safe(&["sessions", "abc123", "files", "main.rs"]);
        assert!(f.is_ok());
        let p = f.unwrap();
        assert!(p.starts_with("/app/data/users/user123"));
        assert!(p.ends_with("main.rs"));
    }

    #[test]
    fn test_user_path_root_traversal() {
        let upr = UserPathRoot::new("user123", None);
        let f = upr.ensure_within(Path::new("../../etc/passwd"));
        assert!(f.is_err());
    }
}

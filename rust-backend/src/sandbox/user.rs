//! # User Layer — عزل المستخدم
//!
//! يخفض صلاحيات العملية إلى nobody (uid=65534, gid=65534)
//! لمنع الوصول إلى ملفات المستخدمين الآخرين.
//!
//! ## لماذا؟
//! - HF Spaces يعمل كـ root في user namespace
//! - بعد fork، يمكننا استدعاء setuid/setgid
//! - حتى لو هرب الكود من الساندبوكس، لن يتمكن من قراءة ملفات الغير
//!
//! ## المرجع
//! - https://huggingface.co/docs/huggingface_hub/en/concepts/sandbox

use crate::sandbox::layer::{SandboxLayer, LayerResult};

/// UID/GID للمستخدم غير المُميز
const NOBODY_UID: u32 = 65534;
const NOBODY_GID: u32 = 65534;

/// طبقة عزل المستخدم
pub struct UserLayer {
    uid: u32,
    gid: u32,
}

impl UserLayer {
    pub fn new() -> Self {
        Self { uid: NOBODY_UID, gid: NOBODY_GID }
    }

    pub fn with_uid(mut self, uid: u32, gid: u32) -> Self {
        self.uid = uid;
        self.gid = gid;
        self
    }
}

impl SandboxLayer for UserLayer {
    fn name(&self) -> &'static str { "user" }

    fn apply_child(&self) -> LayerResult {
        // 1. setgid أولاً
        let ret = unsafe { libc::setgid(self.gid) };
        if ret != 0 {
            return LayerResult::Warn(format!("setgid({}): {} — استمرار كـ root", self.gid,
                std::io::Error::last_os_error()));
        }

        // 2. setuid
        let ret = unsafe { libc::setuid(self.uid) };
        if ret != 0 {
            return LayerResult::Warn(format!("setuid({}): {} — استمرار كـ root", self.uid,
                std::io::Error::last_os_error()));
        }

        tracing::debug!("user: dropped to uid={} gid={}", self.uid, self.gid);
        LayerResult::Ok
    }
}

use alloc::string::String;
use alloc::borrow::ToOwned;
use core::fmt;

/// OS-path-independent path type (no_std). Uses `/` as separator internally.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct VfsPath(pub String);

impl VfsPath {
    pub fn as_str(&self) -> &str { &self.0 }

    pub fn join(&self, segment: &str) -> Self {
        if self.0.ends_with('/') {
            VfsPath(alloc::format!("{}{}", self.0, segment))
        } else {
            VfsPath(alloc::format!("{}/{}", self.0, segment))
        }
    }

    pub fn parent(&self) -> Option<Self> {
        let s = self.0.trim_end_matches('/');
        let idx = s.rfind('/')?;
        Some(VfsPath(s[..idx].to_owned()))
    }

    pub fn file_name(&self) -> Option<&str> {
        let s = self.0.trim_end_matches('/');
        s.rfind('/').map(|i| &s[i + 1..]).or(if s.is_empty() { None } else { Some(s) })
    }
}

impl From<&str> for VfsPath {
    fn from(s: &str) -> Self { VfsPath(s.to_owned()) }
}

impl fmt::Display for VfsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DirEntry {
    pub name:   String,
    pub path:   VfsPath,
    pub is_dir: bool,
}

#[derive(Debug)]
pub enum FsError {
    NotFound(VfsPath),
    PermissionDenied,
    AlreadyExists(VfsPath),
    Io(String),
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsError::NotFound(p)      => write!(f, "not found: {}", p.as_str()),
            FsError::PermissionDenied => write!(f, "permission denied"),
            FsError::AlreadyExists(p) => write!(f, "already exists: {}", p.as_str()),
            FsError::Io(msg)          => write!(f, "I/O error: {msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_adds_separator() {
        let p = VfsPath::from("/foo");
        assert_eq!(p.join("bar").as_str(), "/foo/bar");
    }

    #[test]
    fn join_no_double_slash() {
        let p = VfsPath::from("/foo/");
        assert_eq!(p.join("bar").as_str(), "/foo/bar");
    }

    #[test]
    fn parent_returns_prefix() {
        let p = VfsPath::from("/foo/bar/baz.md");
        assert_eq!(p.parent().unwrap().as_str(), "/foo/bar");
    }

    #[test]
    fn file_name_returns_last_segment() {
        let p = VfsPath::from("/foo/bar/baz.md");
        assert_eq!(p.file_name(), Some("baz.md"));
    }

    #[test]
    fn root_has_no_parent() {
        assert!(VfsPath::from("/").parent().is_none());
    }
}

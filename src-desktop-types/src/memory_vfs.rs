use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::path::{VfsPath, DirEntry, FsError};
use crate::traits::FileSystem;

enum VfsNode {
    File { name: String, content: Vec<u8> },
    Dir  { name: String, children: BTreeMap<String, VfsNode> },
}

pub struct MemoryVfs { root: VfsNode }

impl Default for MemoryVfs {
    fn default() -> Self { Self::new() }
}

impl MemoryVfs {
    pub fn new() -> Self {
        MemoryVfs { root: VfsNode::Dir { name: String::new(), children: BTreeMap::new() } }
    }

    pub fn iter_files(&self) -> impl Iterator<Item = (String, Vec<u8>)> {
        let mut results = Vec::new();
        collect_files(&self.root, &mut String::from("/"), &mut results);
        results.into_iter()
    }
}

fn collect_files(node: &VfsNode, prefix: &mut String, out: &mut Vec<(String, Vec<u8>)>) {
    match node {
        VfsNode::File { name, content } => {
            let path = if prefix.ends_with('/') {
                alloc::format!("{}{}", prefix, name)
            } else {
                alloc::format!("{}/{}", prefix, name)
            };
            out.push((path, content.clone()));
        }
        VfsNode::Dir { name, children } => {
            let prev_len = prefix.len();
            if !name.is_empty() {
                if !prefix.ends_with('/') { prefix.push('/'); }
                prefix.push_str(name);
            }
            for child in children.values() {
                collect_files(child, prefix, out);
            }
            prefix.truncate(prev_len);
        }
    }
}

impl FileSystem for MemoryVfs {
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>, FsError> {
        match navigate(&self.root, path) {
            Some(VfsNode::File { content, .. }) => Ok(content.clone()),
            _ => Err(FsError::NotFound(path.clone())),
        }
    }

    fn write(&mut self, path: &VfsPath, data: &[u8]) -> Result<(), FsError> {
        let parts: Vec<&str> = path.as_str().split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() { return Err(FsError::Io("empty path".into())); }
        let file_name = parts[parts.len() - 1].to_string();
        let dir_parts = &parts[..parts.len() - 1];
        let mut cur = &mut self.root;
        for part in dir_parts {
            match cur {
                VfsNode::Dir { children, .. } => {
                    cur = children.entry(part.to_string()).or_insert_with(|| {
                        VfsNode::Dir { name: part.to_string(), children: BTreeMap::new() }
                    });
                }
                VfsNode::File { .. } => return Err(FsError::Io("parent is a file".into())),
            }
        }
        match cur {
            VfsNode::Dir { children, .. } => {
                children.insert(file_name.clone(), VfsNode::File { name: file_name, content: data.to_vec() });
                Ok(())
            }
            VfsNode::File { .. } => Err(FsError::Io("expected directory".into())),
        }
    }

    fn list_dir(&self, path: &VfsPath) -> Result<Vec<DirEntry>, FsError> {
        match navigate(&self.root, path) {
            Some(VfsNode::Dir { children, .. }) => Ok(children.values().map(|n| match n {
                VfsNode::File { name, .. } => DirEntry { name: name.clone(), path: path.join(name), is_dir: false },
                VfsNode::Dir  { name, .. } => DirEntry { name: name.clone(), path: path.join(name), is_dir: true },
            }).collect()),
            _ => Err(FsError::NotFound(path.clone())),
        }
    }

    fn exists(&self, path: &VfsPath) -> bool { navigate(&self.root, path).is_some() }

    fn create_dir(&mut self, path: &VfsPath) -> Result<(), FsError> {
        let parts: Vec<&str> = path.as_str().split('/').filter(|s| !s.is_empty()).collect();
        let mut cur = &mut self.root;
        for part in &parts {
            match cur {
                VfsNode::Dir { children, .. } => {
                    cur = children.entry(part.to_string()).or_insert_with(|| {
                        VfsNode::Dir { name: part.to_string(), children: BTreeMap::new() }
                    });
                }
                VfsNode::File { .. } => return Err(FsError::Io("parent is a file".into())),
            }
        }
        Ok(())
    }

    fn delete(&mut self, path: &VfsPath) -> Result<(), FsError> {
        let parts: Vec<&str> = path.as_str().split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() { return Err(FsError::Io("cannot delete root".into())); }
        let name = parts[parts.len() - 1];
        let dir_parts = &parts[..parts.len() - 1];
        let mut cur = &mut self.root;
        for part in dir_parts {
            match cur {
                VfsNode::Dir { children, .. } => {
                    cur = children.get_mut(*part).ok_or_else(|| FsError::NotFound(path.clone()))?;
                }
                VfsNode::File { .. } => return Err(FsError::NotFound(path.clone())),
            }
        }
        match cur {
            VfsNode::Dir { children, .. } => {
                children.remove(name).ok_or(FsError::NotFound(path.clone()))?;
                Ok(())
            }
            _ => Err(FsError::NotFound(path.clone())),
        }
    }

    fn rename(&mut self, from: &VfsPath, to: &VfsPath) -> Result<(), FsError> {
        let content = self.read(from)?;
        self.write(to, &content)?;
        self.delete(from)
    }

    fn is_dir(&self, path: &VfsPath) -> bool {
        matches!(navigate(&self.root, path), Some(VfsNode::Dir { .. }))
    }
}

fn navigate<'a>(root: &'a VfsNode, path: &VfsPath) -> Option<&'a VfsNode> {
    let parts: Vec<&str> = path.as_str().split('/').filter(|s| !s.is_empty()).collect();
    let mut cur = root;
    for part in parts {
        match cur {
            VfsNode::Dir { children, .. } => cur = children.get(part)?,
            VfsNode::File { .. } => return None,
        }
    }
    Some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_file() {
        let mut vfs = MemoryVfs::new();
        vfs.write(&VfsPath::from("/foo/bar.md"), b"hello").unwrap();
        assert_eq!(vfs.read(&VfsPath::from("/foo/bar.md")).unwrap(), b"hello");
    }

    #[test]
    fn read_missing_returns_error() {
        let vfs = MemoryVfs::new();
        assert!(matches!(vfs.read(&VfsPath::from("/no/such.md")), Err(FsError::NotFound(_))));
    }

    #[test]
    fn list_dir_returns_children() {
        let mut vfs = MemoryVfs::new();
        vfs.write(&VfsPath::from("/dir/a.md"), b"").unwrap();
        vfs.write(&VfsPath::from("/dir/b.md"), b"").unwrap();
        let mut entries = vfs.list_dir(&VfsPath::from("/dir")).unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "a.md");
    }

    #[test]
    fn delete_removes_file() {
        let mut vfs = MemoryVfs::new();
        vfs.write(&VfsPath::from("/x.md"), b"data").unwrap();
        vfs.delete(&VfsPath::from("/x.md")).unwrap();
        assert!(!vfs.exists(&VfsPath::from("/x.md")));
    }

    #[test]
    fn rename_moves_content() {
        let mut vfs = MemoryVfs::new();
        vfs.write(&VfsPath::from("/old.md"), b"content").unwrap();
        vfs.rename(&VfsPath::from("/old.md"), &VfsPath::from("/new.md")).unwrap();
        assert!(!vfs.exists(&VfsPath::from("/old.md")));
        assert_eq!(vfs.read(&VfsPath::from("/new.md")).unwrap(), b"content");
    }

    #[test]
    fn create_dir_works() {
        let mut vfs = MemoryVfs::new();
        vfs.create_dir(&VfsPath::from("/mydir")).unwrap();
        assert!(vfs.is_dir(&VfsPath::from("/mydir")));
    }

    #[test]
    fn is_dir_distinguishes_file_and_dir() {
        let mut vfs = MemoryVfs::new();
        vfs.write(&VfsPath::from("/dir/file.md"), b"").unwrap();
        assert!(vfs.is_dir(&VfsPath::from("/dir")));
        assert!(!vfs.is_dir(&VfsPath::from("/dir/file.md")));
    }

    #[test]
    fn overwrite_replaces_content() {
        let mut vfs = MemoryVfs::new();
        vfs.write(&VfsPath::from("/a.md"), b"old").unwrap();
        vfs.write(&VfsPath::from("/a.md"), b"new").unwrap();
        assert_eq!(vfs.read(&VfsPath::from("/a.md")).unwrap(), b"new");
    }

    #[test]
    fn delete_nonexistent_returns_not_found() {
        let mut vfs = MemoryVfs::new();
        assert!(matches!(vfs.delete(&VfsPath::from("/ghost.md")), Err(FsError::NotFound(_))));
    }
}

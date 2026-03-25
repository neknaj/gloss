use std::fs;
use src_desktop_types::{FileSystem, VfsPath, DirEntry, FsError};

pub struct NativeFs;

impl FileSystem for NativeFs {
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>, FsError> {
        fs::read(path.as_str()).map_err(|e| map_io_err(e, path))
    }

    fn write(&mut self, path: &VfsPath, data: &[u8]) -> Result<(), FsError> {
        if let Some(parent) = std::path::Path::new(path.as_str()).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| FsError::Io(e.to_string()))?;
            }
        }
        fs::write(path.as_str(), data).map_err(|e| map_io_err(e, path))
    }

    fn list_dir(&self, path: &VfsPath) -> Result<Vec<DirEntry>, FsError> {
        let rd = fs::read_dir(path.as_str()).map_err(|e| map_io_err(e, path))?;
        let mut result = Vec::new();
        for entry in rd {
            let entry = entry.map_err(|e| FsError::Io(e.to_string()))?;
            let name   = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let ep     = entry.path().to_string_lossy().into_owned();
            result.push(DirEntry { name, path: VfsPath::from(ep.as_str()), is_dir });
        }
        Ok(result)
    }

    fn exists(&self, path: &VfsPath) -> bool {
        std::path::Path::new(path.as_str()).exists()
    }

    fn create_dir(&mut self, path: &VfsPath) -> Result<(), FsError> {
        fs::create_dir_all(path.as_str()).map_err(|e| FsError::Io(e.to_string()))
    }

    fn delete(&mut self, path: &VfsPath) -> Result<(), FsError> {
        let p = std::path::Path::new(path.as_str());
        if p.is_dir() {
            fs::remove_dir_all(p).map_err(|e| FsError::Io(e.to_string()))
        } else {
            fs::remove_file(p).map_err(|e| map_io_err(e, path))
        }
    }

    fn rename(&mut self, from: &VfsPath, to: &VfsPath) -> Result<(), FsError> {
        fs::rename(from.as_str(), to.as_str()).map_err(|e| FsError::Io(e.to_string()))
    }

    fn is_dir(&self, path: &VfsPath) -> bool {
        std::path::Path::new(path.as_str()).is_dir()
    }
}

fn map_io_err(e: std::io::Error, path: &VfsPath) -> FsError {
    match e.kind() {
        std::io::ErrorKind::NotFound         => FsError::NotFound(path.clone()),
        std::io::ErrorKind::PermissionDenied => FsError::PermissionDenied,
        std::io::ErrorKind::AlreadyExists    => FsError::AlreadyExists(path.clone()),
        _                                    => FsError::Io(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::NativeFs;
    use src_desktop_types::{FileSystem, VfsPath};

    #[test]
    fn native_fs_write_and_read() {
        let mut fs = NativeFs;
        let path = VfsPath::from(
            std::env::temp_dir().join("gloss_native_test.txt")
                .to_string_lossy().as_ref()
        );
        fs.write(&path, b"hello").unwrap();
        let bytes = fs.read(&path).unwrap();
        assert_eq!(bytes, b"hello");
        fs.delete(&path).unwrap();
    }

    #[test]
    fn native_fs_exists_returns_false_for_missing() {
        let fs = NativeFs;
        assert!(!fs.exists(&VfsPath::from("/nonexistent/gloss_test_xyz.txt")));
    }

    #[test]
    fn native_fs_list_dir_returns_entries() {
        let mut fs = NativeFs;
        let sub = std::env::temp_dir().join("gloss_list_test");
        let sub_path = VfsPath::from(sub.to_string_lossy().as_ref());
        fs.create_dir(&sub_path).unwrap();
        let file_path = VfsPath::from(sub.join("a.txt").to_string_lossy().as_ref());
        fs.write(&file_path, b"x").unwrap();
        let entries = fs.list_dir(&sub_path).unwrap();
        assert!(entries.iter().any(|e| e.name == "a.txt"));
        fs.delete(&sub_path).unwrap();
    }
}

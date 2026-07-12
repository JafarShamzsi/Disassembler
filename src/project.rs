use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const PROJECT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisProject {
    pub schema_version: u32,
    pub binary: ProjectBinary,
    pub user_names: Vec<UserName>,
    pub comments: Vec<Comment>,
    pub bookmarks: Vec<Bookmark>,
    pub functions: Vec<ProjectFunction>,
}

impl AnalysisProject {
    pub fn new(binary: ProjectBinary) -> Self {
        Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            binary,
            user_names: Vec::new(),
            comments: Vec::new(),
            bookmarks: Vec::new(),
            functions: Vec::new(),
        }
    }

    pub fn from_binary(path: impl AsRef<Path>, data: &[u8]) -> Self {
        Self::new(ProjectBinary::from_data(path, data))
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let data = fs::read_to_string(path)?;
        serde_json::from_str(&data).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        fs::write(path, json)
    }

    pub fn set_user_name(&mut self, address: u64, name: impl Into<String>) {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            self.remove_user_name(address);
            return;
        }
        if let Some(existing) = self
            .user_names
            .iter_mut()
            .find(|user_name| user_name.address == address)
        {
            existing.name = name;
        } else {
            self.user_names.push(UserName { address, name });
            self.user_names.sort_by_key(|user_name| user_name.address);
        }
    }

    pub fn set_comment(&mut self, address: u64, text: impl Into<String>) {
        let text = text.into().trim().to_string();
        if text.is_empty() {
            self.remove_comment(address);
            return;
        }
        if let Some(existing) = self
            .comments
            .iter_mut()
            .find(|comment| comment.address == address)
        {
            existing.text = text;
        } else {
            self.comments.push(Comment { address, text });
            self.comments.sort_by_key(|comment| comment.address);
        }
    }

    pub fn toggle_bookmark(&mut self, address: u64, label: Option<String>) {
        if let Some(existing_idx) = self
            .bookmarks
            .iter()
            .position(|bookmark| bookmark.address == address)
        {
            self.bookmarks.remove(existing_idx);
        } else {
            self.bookmarks.push(Bookmark { address, label });
            self.bookmarks.sort_by_key(|bookmark| bookmark.address);
        }
    }

    pub fn remove_user_name(&mut self, address: u64) {
        self.user_names
            .retain(|user_name| user_name.address != address);
    }

    pub fn remove_comment(&mut self, address: u64) {
        self.comments.retain(|comment| comment.address != address);
    }

    pub fn user_name_at(&self, address: u64) -> Option<&UserName> {
        self.user_names
            .iter()
            .find(|user_name| user_name.address == address)
    }

    pub fn comment_at(&self, address: u64) -> Option<&Comment> {
        self.comments
            .iter()
            .find(|comment| comment.address == address)
    }

    pub fn bookmark_at(&self, address: u64) -> Option<&Bookmark> {
        self.bookmarks
            .iter()
            .find(|bookmark| bookmark.address == address)
    }

    pub fn is_bookmarked(&self, address: u64) -> bool {
        self.bookmark_at(address).is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectBinary {
    pub path: PathBuf,
    pub size: u64,
    pub fingerprint: String,
}

impl ProjectBinary {
    pub fn from_data(path: impl AsRef<Path>, data: &[u8]) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            size: data.len() as u64,
            fingerprint: fingerprint_bytes(data),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserName {
    pub address: u64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub address: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub address: u64,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectFunction {
    pub entry: u64,
    pub name: Option<String>,
}

fn fingerprint_bytes(data: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_round_trips_user_analysis() {
        let path = std::env::temp_dir().join(format!(
            "disassembler-project-test-{}.json",
            std::process::id()
        ));
        let mut project = AnalysisProject::from_binary("sample.exe", b"hello binary");
        project.set_user_name(0x1000, "entry");
        project.set_comment(0x1000, "first interesting block");
        project.toggle_bookmark(0x1000, Some("start".to_string()));

        project.save(&path).unwrap();
        let loaded = AnalysisProject::load(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(loaded, project);
        assert_eq!(loaded.schema_version, PROJECT_SCHEMA_VERSION);
        assert_eq!(loaded.user_names[0].name, "entry");
        assert_eq!(loaded.comments[0].text, "first interesting block");
        assert_eq!(loaded.bookmarks[0].label.as_deref(), Some("start"));
    }

    #[test]
    fn project_updates_existing_name_and_comment() {
        let mut project = AnalysisProject::from_binary("sample.exe", b"binary");

        project.set_user_name(0x1000, "old");
        project.set_user_name(0x1000, "new");
        project.set_comment(0x1000, "old comment");
        project.set_comment(0x1000, "new comment");
        project.toggle_bookmark(0x1000, None);
        project.toggle_bookmark(0x1000, None);

        assert_eq!(project.user_names.len(), 1);
        assert_eq!(project.user_names[0].name, "new");
        assert_eq!(project.comments.len(), 1);
        assert_eq!(project.comments[0].text, "new comment");
        assert!(project.bookmarks.is_empty());
    }

    #[test]
    fn empty_name_and_comment_remove_existing_entries() {
        let mut project = AnalysisProject::from_binary("sample.exe", b"binary");

        project.set_user_name(0x1000, "entry");
        project.set_comment(0x1000, "important");
        project.set_user_name(0x1000, " ");
        project.set_comment(0x1000, "");

        assert!(project.user_name_at(0x1000).is_none());
        assert!(project.comment_at(0x1000).is_none());
    }
}

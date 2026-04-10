#![allow(dead_code)]
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Dynamic Markdown skill parser and hot-swapper.
pub struct SkillsEngine {
    skills_dir: PathBuf,
}

impl SkillsEngine {
    pub fn new(skills_dir: impl AsRef<Path>) -> Self {
        Self {
            skills_dir: skills_dir.as_ref().to_path_buf(),
        }
    }

    pub fn list_agent_skills(&self) -> Result<Vec<String>> {
        let mut skills = Vec::new();
        if self.skills_dir.exists() && self.skills_dir.is_dir() {
            for entry in std::fs::read_dir(&self.skills_dir)? {
                let entry = entry?;
                if let Some(ext) = entry.path().extension() {
                    if ext == "md" {
                        if let Some(name) = entry.path().file_stem() {
                            skills.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        Ok(skills)
    }

    pub fn read_skill(&self, skill_name: &str) -> Result<String> {
        let mut file_path = self.skills_dir.join(skill_name);
        file_path.set_extension("md");
        
        let content = std::fs::read_to_string(file_path)?;
        Ok(content)
    }
}

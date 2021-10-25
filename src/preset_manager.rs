use crate::compiler::{DEFAULT_CODE, START_CODE};
use crate::float_id::FloatId;
use anyhow::bail;
use directories::ProjectDirs;
use std::any::Any;
use std::fs::{self, File};
use std::io::Write;
use std::ops::RangeBounds;
use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    version: String,
    aliases: HashMap<String, String>,
}

//toml::from_str(&body).unwrap()

fn create_path_if_needed(p: &Path) -> anyhow::Result<()> {
    if !p.exists() {
        fs::create_dir(p)?;
        dbg!("path created: {:?}", p);
    }
    Ok(())
}

pub struct ProjectPaths {
    config_dir: PathBuf,
    projects_dir: PathBuf,
    application_path: PathBuf,
    base_sarus_path: PathBuf,
    config_file: PathBuf,
}

fn setup_dirs() -> anyhow::Result<ProjectPaths> {
    if let Some(proj_dirs) = ProjectDirs::from("org", "Sarus", "EditorPlugin") {
        let config_dir = proj_dirs.config_dir().to_path_buf();
        let projects_dir = config_dir.join("projects");
        let application_path = config_dir.parent().unwrap().to_path_buf();
        let base_sarus_path = application_path.parent().unwrap().to_path_buf();
        create_path_if_needed(&base_sarus_path)?;
        create_path_if_needed(&application_path)?;
        create_path_if_needed(&config_dir)?;
        create_path_if_needed(&projects_dir)?;
        let example_file = projects_dir.join("example.sarus");
        if !example_file.exists() {
            let mut file = File::create(&example_file)?;
            file.write_all(DEFAULT_CODE.as_bytes())?;
        }
        let config_file = config_dir.join("config.toml");
        if !config_file.exists() {
            let mut aliases = HashMap::new();
            aliases.insert(FloatId::new().to_string(), "example.sarus".to_string());
            let p = Config {
                version: "0.0.1".to_string(),
                aliases,
            };
            let mut file = File::create(&config_file)?;
            file.write_all(toml::to_string(&p)?.as_bytes())?;
        }
        Ok(ProjectPaths {
            config_dir,
            projects_dir,
            application_path,
            base_sarus_path,
            config_file,
        })
    } else {
        anyhow::bail!("could not init ProjectDirs")
    }
}

fn load_config(project_paths: &ProjectPaths) -> anyhow::Result<Config> {
    let config: Config = toml::from_str(
        &fs::read_to_string(project_paths.config_file.clone()).expect("could not load config file"),
    )?;
    Ok(config)
}

fn load_project_files(
    project_paths: &ProjectPaths,
    config: &Config,
) -> anyhow::Result<HashMap<String, String>> {
    let mut files = HashMap::new();

    for (id, path) in &config.aliases {
        if let Ok(code) = fs::read_to_string(project_paths.projects_dir.join(path)) {
            files.insert(id.clone(), code);
        } else {
            anyhow::bail!("could not load file {} {}", id, path)
        }
    }
    Ok(files)
}

pub struct Projects {
    pub project_paths: ProjectPaths,
    pub files: HashMap<String, String>,
    pub config: Config,
}

impl Projects {
    pub fn load() -> anyhow::Result<Self> {
        let project_paths = setup_dirs()?;
        let config = load_config(&project_paths)?;
        let files = load_project_files(&project_paths, &config)?;
        Ok(Projects {
            project_paths,
            files,
            config,
        })
    }

    pub fn reload(&mut self) -> anyhow::Result<()> {
        self.config = load_config(&self.project_paths)?;
        self.files = load_project_files(&self.project_paths, &self.config)?;
        Ok(())
    }

    pub fn new_project(&mut self, file_name: &str) -> anyhow::Result<()> {
        let file_name = if !file_name.ends_with(".sarus") {
            format!("{}.sarus", file_name)
        } else {
            file_name.to_string()
        };
        let new_project_file_path = self.project_paths.projects_dir.join(&file_name);
        if new_project_file_path.exists() {
            anyhow::bail!("file {} already exists", &file_name)
        }
        self.config
            .aliases
            .insert(FloatId::new().to_string(), file_name.to_string());
        let mut file = File::create(&self.project_paths.config_file)?; //TODO does this overwrite?
        file.write_all(toml::to_string(&self.config)?.as_bytes())?;

        let mut file = File::create(&new_project_file_path)?;
        file.write_all(START_CODE.as_bytes())?;
        Ok(())
    }

    pub fn get_code_by_id(&self, id: FloatId) -> anyhow::Result<String> {
        if let Some(code) = self.files.get(&id.to_string()) {
            Ok(code.clone())
        } else {
            anyhow::bail!("code for id {} not found", id)
        }
    }

    pub fn get_code_by_name(&self, name: &str) -> anyhow::Result<String> {
        let id = self.config.aliases.iter().find_map(
            |(key, val)| {
                if val == name {
                    Some(key)
                } else {
                    None
                }
            },
        );
        if let Some(id) = id {
            if let Some(code) = self.files.get(id) {
                Ok(code.clone())
            } else {
                anyhow::bail!("code for id {} not found", id)
            }
        } else {
            anyhow::bail!("id for name {} not found", name)
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_toml() -> anyhow::Result<()> {
        let mut aliases = HashMap::new();
        aliases.insert("0123421".to_string(), "path/to/file".to_string());
        let config = Config {
            version: "0.0.1".to_string(),
            aliases,
        };
        println!("{}", toml::to_string(&config)?);
        Ok(())
    }

    #[test]
    fn test_load_files() -> anyhow::Result<()> {
        let project_paths = setup_dirs()?;
        let config_file = load_config(&project_paths)?;
        let files = load_project_files(&project_paths, &config_file)?;

        assert!(files.len() > 0);
        Ok(())
    }
}

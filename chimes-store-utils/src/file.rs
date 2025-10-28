use anyhow::{anyhow, Result};

use std::{
    fs::{create_dir_all, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

pub fn load_json_file<T>(path: impl AsRef<Path>) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    Ok(serde_json::from_str(&contents)?)
}

pub fn save_json_file<T>(path: impl AsRef<Path>, data: &T) -> Result<()>
where
    T: serde::ser::Serialize + ?Sized,
{
    let path_ = path.as_ref();
    tracing::debug!("Save Config: {}", path_.to_string_lossy());
    if let Some(path_prt) = path_.parent() {
        if let Err(err) = create_dir_all(path_prt) {
            tracing::debug!(
                "could not create dir for {}, error {err}.",
                path_.to_string_lossy()
            );
        }
    }

    let mut file = File::create(path)?;
    let content = serde_json::to_string_pretty(data)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

/**
 * 获取当前目录
 * 当前目录是执执行文件所在目录
 */
pub fn get_current_dir() -> Result<PathBuf, anyhow::Error> {
    match std::env::current_exe() {
        Ok(exefile) => {
            if let Some(path) = exefile.parent() {
                if path.join("assets").exists() {
                    Ok(path.to_path_buf())
                } else {
                    std::env::current_dir().map_err(|e| anyhow!("error to get current dir {e}"))
                }
            } else {
                std::env::current_dir().map_err(|e| anyhow!("error to get current dir {e}"))
            }
        },
        Err(_) => {
            std::env::current_dir().map_err(|e| anyhow!("error to get current dir {e}"))
        }
    }
}
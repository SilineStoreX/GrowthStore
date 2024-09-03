use anyhow::anyhow;
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipWriter};

fn add_file_into_zip(
    zip: &mut ZipWriter<std::fs::File>,
    base_path: impl AsRef<Path>,
    file: &str,
) -> Result<(), anyhow::Error> {
    if file.ends_with('/') {
        zip.add_directory(file.to_owned(), SimpleFileOptions::default())?;
    } else {
        zip.start_file(file.to_owned(), SimpleFileOptions::default())?;
        let content_filepath = base_path.as_ref().join(file);
        let mut cfile = std::fs::File::open(content_filepath)?;
        let mut buf = vec![];
        cfile.read_to_end(&mut buf)?;
        zip.write_all(&buf)?;
        buf.clear();
    }
    Ok(())
}

pub fn create_zip_file(
    filename: impl AsRef<Path>,
    base_path: impl AsRef<Path>,
    files: &[String],
) -> Result<(), anyhow::Error> {
    let file = match std::fs::File::create(filename.as_ref()) {
        Ok(t) => t,
        Err(err) => {
            return Err(anyhow!(err));
        }
    };

    let mut zip = zip::ZipWriter::new(file);

    for file in files {
        add_file_into_zip(&mut zip, base_path.as_ref(), file)?;
    }

    zip.finish()?;

    Ok(())
}

pub fn check_zip_match_archive(
    filename: impl AsRef<Path>,
    dest_path: impl AsRef<Path>,
    force: bool,
) -> Result<bool, anyhow::Error> {
    let file = fs::File::open(filename.as_ref())?;
    let mut zip = zip::ZipArchive::new(file)?;
    let destpath = dest_path.as_ref();

    for i in 0..zip.len() {
        let zipfile = zip.by_index(i)?;
        let path = match zipfile.enclosed_name() {
            Some(path) => path,
            None => continue,
        };

        // archive file should be start models and scripts
        if !(path.starts_with("models/") || path.starts_with("scripts/")) {
            return Ok(false);
        }

        if zipfile.is_file() {
            let tp_dest = destpath.join(path);
            if tp_dest.exists() && !force {
                return Err(anyhow!("Target file exist ({tp_dest:?})"));
            }
        }
    }
    Ok(true)
}

pub fn extract_zip_file(
    filename: impl AsRef<Path>,
    dest_path: impl AsRef<Path>,
    force: bool,
) -> Result<(), anyhow::Error> {
    let file = fs::File::open(filename.as_ref())?;
    let mut zip = zip::ZipArchive::new(file)?;
    let destpath = dest_path.as_ref();

    for i in 0..zip.len() {
        let mut zipfile = zip.by_index(i)?;
        let path = match zipfile.enclosed_name() {
            Some(path) => path,
            None => continue,
        };
        let tp_dest = destpath.join(path);
        if zipfile.is_dir() {
            fs::create_dir_all(tp_dest)?;
        } else if zipfile.is_file() {
            // check the file exists
            if let Some(p) = tp_dest.parent() {
                if !p.exists() {
                    if let Err(err) = fs::create_dir_all(p) {
                        log::warn!("Unable to create dir {}, err is {err}", p.display());
                        continue;
                    }
                }
            }

            if tp_dest.exists() && !force {
                return Err(anyhow!("Target file exist ({tp_dest:?})"));
            }

            let mut outfile = fs::File::create(&tp_dest)?;

            io::copy(&mut zipfile, &mut outfile)?;
        }
    }
    Ok(())
}

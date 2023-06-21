use std::{
    fs::{create_dir_all, read, write},
    io::{Cursor, Read, Seek, Write},
    path::PathBuf,
    process::Command,
};

use anyhow::{anyhow, Result};
use tempfile::{NamedTempFile, TempDir};

use binrw::{binrw, BinReaderExt, BinWrite, NullString};

#[binrw]
#[derive(Debug)]
struct FileEntry {
    len: u32,
    #[brw(pad_size_to(12))]
    name: NullString,
}

pub fn build_fs(
    files: Vec<PathBuf>,
    outfile: PathBuf,
    matching: bool,
    pad: Option<usize>,
    lzari: PathBuf,
) -> Result<()> {
    for file in &files {
        if !file.exists() || !file.is_file() {
            return Err(anyhow!("{} is not a file", file.to_string_lossy()));
        }
    }

    if outfile.exists() && !outfile.is_file() {
        return Err(anyhow!(
            "{} exists and is not a file",
            outfile.to_string_lossy()
        ));
    }

    if let Some(parent) = outfile.parent() {
        if !parent.exists() {
            create_dir_all(parent)?
        }
    }

    let mut data = vec![];
    let mut cursor = Cursor::new(&mut data);

    let tempdir = TempDir::new()?;
    let tempfile = tempdir.into_path().join("comp.bin");
    for file in &files {
        Command::new(&lzari)
            .args(["e", &file.to_string_lossy(), &tempfile.to_string_lossy()])
            .status()?;
        let compressed = read(&tempfile)?;
        let filename = file
            .file_name()
            .expect("Need a filename to add to FS")
            .to_string_lossy()
            .to_string();
        let entry = FileEntry {
            len: compressed.len() as u32 + 0x10,
            name: NullString::from(filename.clone()),
        };
        entry.write_be(&mut cursor)?;

        if matching && filename == "tile1.tg~" {
            cursor.seek(std::io::SeekFrom::Current(-2))?;
            cursor.write_all(&[0x6Cu8, 0x00])?;
        }

        cursor.write_all(&compressed)?;
    }

    if let Some(size) = pad {
        if (cursor.position() as usize) < size {
            cursor.write_all(&vec![0xFF; size - cursor.position() as usize])?;
        }
    }

    write(outfile, data)?;

    Ok(())
}

pub fn extract_fs(infile: PathBuf, outdir: PathBuf, lzari: PathBuf) -> Result<()> {
    if !infile.exists() || !infile.is_file() {
        return Err(anyhow!("{} is not a file", infile.to_string_lossy()));
    }

    if outdir.exists() {
        if !outdir.is_dir() {
            return Err(anyhow!(
                "{} exists and is not a directory",
                outdir.to_string_lossy()
            ));
        }
    } else {
        create_dir_all(&outdir)?
    }

    let data = read(infile)?;
    let mut cursor = Cursor::new(&data);

    loop {
        let file = match cursor.read_be::<FileEntry>() {
            Ok(f) => f,
            Err(_) => break,
        };
        if file.len == 0xFFFFFFFF {
            break;
        }
        let mut compressed = vec![0; file.len as usize - 0x10];
        cursor.read_exact(&mut compressed)?;

        let mut tempfile = NamedTempFile::new()?;
        tempfile.write_all(&compressed)?;
        Command::new(&lzari)
            .args([
                "d",
                &tempfile.path().to_string_lossy(),
                &outdir.join(file.name.to_string()).to_string_lossy(),
            ])
            .status()?;
    }

    Ok(())
}

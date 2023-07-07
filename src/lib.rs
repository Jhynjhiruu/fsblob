use std::{
    fs::{create_dir_all, read, write},
    io::{Cursor, Read, Write},
    path::PathBuf,
    process::Command,
};

use anyhow::{anyhow, Result};
use snailquote::unescape;
use tempfile::{NamedTempFile, TempDir};

use binrw::{binrw, BinReaderExt, BinWrite};

use std::cmp::Ordering;

#[binrw]
#[derive(Debug)]
struct FileEntry {
    len: u32,
    name: [u8; 12],
}

fn copy_from_str(dest: &mut [u8], src: &str) {
    match dest.len().cmp(&src.len()) {
        Ordering::Less => dest.copy_from_slice(&src.as_bytes()[..dest.len()]),
        Ordering::Equal => dest.copy_from_slice(src.as_bytes()),
        Ordering::Greater => dest[..src.len()].copy_from_slice(src.as_bytes()),
    }
}

pub fn build_fs(
    files: Vec<String>,
    outfile: PathBuf,
    pad: Option<usize>,
    lzari: PathBuf,
) -> Result<()> {
    let files = files
        .iter()
        .map(|f| {
            let tup = f.split_once('@').map_or(
                (
                    f.to_owned(),
                    PathBuf::from(f)
                        .file_name()
                        .expect("Should have a filename")
                        .to_string_lossy()
                        .into_owned(),
                ),
                |e| {
                    (
                        e.0.to_owned(),
                        unescape(e.1).expect("Needs valid filename to add to FS"),
                    )
                },
            );
            (PathBuf::from(tup.0), tup.1)
        })
        .collect::<Vec<_>>();

    for (file, _) in &files {
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
    for (file, filename) in &files {
        Command::new(&lzari)
            .args(["e", &file.to_string_lossy(), &tempfile.to_string_lossy()])
            .status()?;
        let compressed = read(&tempfile)?;

        let mut name = [0; 12];
        copy_from_str(&mut name, filename);

        let entry = FileEntry {
            len: compressed.len() as u32 + 0x10,
            name,
        };
        entry.write_be(&mut cursor)?;

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
                &outdir
                    .join(
                        String::from_utf8(file.name.to_vec())
                            .expect("Filename must be valid")
                            .split('\0')
                            .next()
                            .expect(".split should always return at least one segment"),
                    )
                    .to_string_lossy(),
            ])
            .status()?;
    }

    Ok(())
}

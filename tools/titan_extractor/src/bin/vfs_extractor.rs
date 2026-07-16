use std::{fs, path::PathBuf};

use rose_file_readers::{RoseFileReader, VfsFile, VfsIndex, VfsPath, VirtualFilesystemDevice};

/// Extracts every file out of a classic irose-style data.idx + *.VFS archive
/// set (used by RUFF/vanilla iRose/SevenHearts) into a loose directory tree.
/// Unlike TitanROSE's hash-only index, this format stores real file names in
/// the index itself, so filenames are read directly from it (no external
/// "known names" list needed).
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "Usage: {} <data.idx> <output-dir>",
            args.first().map(String::as_str).unwrap_or("vfs_extractor")
        );
        std::process::exit(1);
    }

    let idx_path = PathBuf::from(&args[1]);
    let output_dir = PathBuf::from(&args[2]);

    println!("Reading file names from {}...", idx_path.display());
    let filenames = read_filenames(&idx_path)?;
    println!("Found {} file names.", filenames.len());

    println!("Loading VFS index...");
    let vfs = VfsIndex::load(&idx_path)?;

    fs::create_dir_all(&output_dir)?;

    let mut extracted = 0usize;
    let mut failed = 0usize;

    for filename in &filenames {
        let vfs_path: VfsPath = filename.as_str().into();

        let file = match vfs.open_file(&vfs_path) {
            Ok(file) => file,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        let relative_path = filename.replace('\\', "/");
        let out_path = output_dir.join(relative_path.trim_start_matches('/'));
        if let Some(parent) = out_path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                eprintln!("Skipping {filename}: failed to create dir: {error}");
                failed += 1;
                continue;
            }
        }

        let bytes: &[u8] = match &file {
            VfsFile::Buffer(buf) => buf.as_slice(),
            VfsFile::View(view) => view,
        };
        if let Err(error) = fs::write(&out_path, bytes) {
            eprintln!("Skipping {filename}: failed to write: {error}");
            failed += 1;
            continue;
        }
        extracted += 1;

        if extracted % 1000 == 0 {
            println!("  extracted {extracted}...");
        }
    }

    println!(
        "Done. Extracted {extracted} files ({failed} failed) to {}.",
        output_dir.display()
    );

    Ok(())
}

/// Re-parses data.idx to recover the real file names (VfsIndex itself keeps
/// them private after loading), mirroring rose-file-readers' own VfsIndex::load.
fn read_filenames(index_path: &std::path::Path) -> anyhow::Result<Vec<String>> {
    let data = fs::read(index_path)?;
    let mut reader = RoseFileReader::from(&data);
    let mut filenames = Vec::new();

    let _base_version = reader.read_u32()?;
    let _current_version = reader.read_u32()?;
    let num_vfs = reader.read_u32()? as usize;

    for _ in 0..num_vfs {
        let (vfs_filename, _, _) =
            encoding_rs::EUC_KR.decode(reader.read_u16_length_bytes()?.split_last().unwrap().1);
        let offset = reader.read_u32()? as u64;
        let next_vfs_position = reader.position();
        reader.set_position(offset);

        let num_files = reader.read_u32()? as usize;
        let _ = reader.read_u32()?;
        let _ = reader.read_u32()?;

        if vfs_filename.to_uppercase() == "ROOT.VFS" {
            reader.set_position(next_vfs_position);
            continue;
        }

        for _ in 0..num_files {
            let (filename, _, _) = encoding_rs::EUC_KR
                .decode(reader.read_u16_length_bytes()?.split_last().unwrap().1);
            let _offset = reader.read_u32()?;
            let _size = reader.read_u32()?;
            let _block_size = reader.read_u32()?;
            let is_deleted = reader.read_u8()?;
            let _is_compressed = reader.read_u8()?;
            let _is_encrypted = reader.read_u8()?;
            let _version = reader.read_u32()?;
            let _crc = reader.read_u32()?;

            if is_deleted == 0 {
                filenames.push(
                    VfsPath::normalise_path(&filename)
                        .to_string_lossy()
                        .to_string(),
                );
            }
        }

        reader.set_position(next_vfs_position);
    }

    Ok(filenames)
}

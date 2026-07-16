use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use rose_file_readers::{RoseFileReader, TitanVfsIndex, VfsFile, VfsPath, VirtualFilesystemDevice};

/// Reads the file name list out of a vanilla irose data.idx (which - unlike
/// TitanROSE's hash-only index - stores real file names), returning
/// forward-slash, uppercase, VFS-normalised paths.
fn read_vanilla_filenames(index_path: &Path) -> anyhow::Result<Vec<String>> {
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

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "Usage: {} <vanilla-data.idx> <titan-dir-with-data.idx+data.trf> <output-dir>",
            args.first().map(String::as_str).unwrap_or("titan_extractor")
        );
        std::process::exit(1);
    }

    let vanilla_idx_path = PathBuf::from(&args[1]);
    let titan_dir = PathBuf::from(&args[2]);
    let output_dir = PathBuf::from(&args[3]);

    println!("Reading known file names from {}...", vanilla_idx_path.display());
    let mut filenames = read_vanilla_filenames(&vanilla_idx_path)?;
    filenames.sort();
    filenames.dedup();
    println!("Found {} known file names.", filenames.len());

    let titan_idx_path = titan_dir.join("data.idx");
    let titan_trf_path = titan_dir.join("data.trf");
    println!(
        "Loading TitanROSE VFS from {} / {}...",
        titan_idx_path.display(),
        titan_trf_path.display()
    );
    let titan_vfs = TitanVfsIndex::load(&titan_idx_path, &titan_trf_path)?;
    println!("Loaded, version {}.", titan_vfs.version);

    fs::create_dir_all(&output_dir)?;

    let mut extracted = 0usize;
    let mut missing: HashSet<String> = HashSet::new();

    for filename in &filenames {
        let vfs_path: VfsPath = filename.as_str().into();

        let file = match titan_vfs.open_file(&vfs_path) {
            Ok(file) => file,
            Err(_) => {
                missing.insert(filename.clone());
                continue;
            }
        };

        let relative_path = filename.replace('\\', "/");
        let out_path = output_dir.join(relative_path.trim_start_matches('/'));
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let bytes: &[u8] = match &file {
            VfsFile::Buffer(buf) => buf.as_slice(),
            VfsFile::View(view) => view,
        };
        fs::write(&out_path, bytes)?;
        extracted += 1;

        if extracted % 500 == 0 {
            println!("  extracted {extracted}...");
        }
    }

    println!(
        "Done. Extracted {extracted} files ({} known paths not present in this TitanROSE archive) to {}.",
        missing.len(),
        output_dir.display()
    );

    Ok(())
}

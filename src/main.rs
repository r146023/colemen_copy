use std::collections::HashSet;
use std::env;
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write, Seek};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::thread;
use rand::{Rng, thread_rng};

/// ColemenCopy - A cross-platform alternative to Robocopy
///
/// Supported options:
/// /S         - Copy subdirectories, but not empty ones
/// /E         - Copy subdirectories, including empty ones
/// /Z         - Copy files in restartable mode (slower but more robust)
/// /B         - Copy files in Backup mode (overrides file/folder permissions)
/// /PURGE     - Delete destination files/folders that no longer exist in source
/// /MIR       - Mirror directory tree (like /PURGE plus all subdirectories)
/// /MOV       - Move files (delete from source after copying)
/// /MOVE      - Move files and directories (delete from source after copying)
/// /A+:[RASHCNETO] - Add specified attributes to copied files
/// /A-:[RASHCNETO] - Remove specified attributes from copied files
/// /MT[:n]    - Multithreaded copying with n threads (default is 8)
/// /R:n       - Number of retries on failed copies (default is 1 million)
/// /W:n       - Wait time between retries in seconds (default is 30)
/// /LOG:file  - Output log to file
/// /L         - List only - don't copy, timestamp or delete any files
/// /NP        - No progress - don't display % copied
/// /NFL       - No file list - don't log file names
/// /EMPTY     - Create empty (zero-byte) copies of files

struct CopyOptions {
    recursive: bool,
    include_empty: bool,
    restartable: bool,
    backup_mode: bool,
    purge: bool,
    mirror: bool,
    move_files: bool,
    move_dirs: bool,
    attributes_add: String,
    attributes_remove: String,
    threads: usize,
    retries: usize,
    wait_time: u64,
    log_file: Option<String>,
    list_only: bool,
    show_progress: bool,
    log_file_names: bool,
    empty_files: bool,  // New option for creating empty files
    child_only: bool,  // New option for processing only direct child folders
    shred_files: bool,  // New option for secure file deletion
}

impl Default for CopyOptions {
    fn default() -> Self {
        CopyOptions {
            recursive: false,
            include_empty: false,
            restartable: false,
            backup_mode: false,
            purge: false,
            mirror: false,
            move_files: false,
            move_dirs: false,
            attributes_add: String::new(),
            attributes_remove: String::new(),
            threads: 8,
            retries: 1_000_000,
            wait_time: 30,
            log_file: None,
            list_only: false,
            show_progress: true,
            log_file_names: true,
            empty_files: false,  // Default to false
            child_only: false,  // Default to false
            shred_files: false,  // Default to false
        }
    }
}

struct Statistics {
    dirs_created: usize,
    files_copied: usize,
    bytes_copied: u64,
    dirs_skipped: usize,
    files_skipped: usize,
    files_failed: usize,
    dirs_removed: usize,
    files_removed: usize,
}

impl Default for Statistics {
    fn default() -> Self {
        Statistics {
            dirs_created: 0,
            files_copied: 0,
            bytes_copied: 0,
            dirs_skipped: 0,
            files_skipped: 0,
            files_failed: 0,
            dirs_removed: 0,
            files_removed: 0,
        }
    }
}

fn main() -> io::Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        print_usage(&args[0]);
        return Ok(());
    }

    let source_dir = &args[1];
    let dest_dir = &args[2];

    // Check if source directory exists
    let source_path = Path::new(source_dir);
    if !source_path.exists() {
        eprintln!("ERROR: Source directory does not exist: {}", source_dir);
        return Ok(());
    }

    // Extract file pattern if specified (3rd argument)
    let file_pattern = if args.len() > 3 && !args[3].starts_with('/') {
        Some(args[3].clone())
    } else {
        None
    };

    // Parse options
    let mut options = CopyOptions::default();

    for arg in args.iter().skip(3 + if file_pattern.is_some() { 1 } else { 0 }) {
        match arg.to_uppercase().as_str() {
            "/S" => options.recursive = true,
            "/E" => {
                options.recursive = true;
                options.include_empty = true;
            },
            "/Z" => options.restartable = true,
            "/B" => options.backup_mode = true,
            "/PURGE" => options.purge = true,
            "/MIR" => {
                options.purge = true;
                options.recursive = true;
                options.include_empty = true;
            },
            "/MOV" => options.move_files = true,
            "/MOVE" => {
                options.move_files = true;
                options.move_dirs = true;
            },
            "/L" => options.list_only = true,
            "/NP" => options.show_progress = false,
            "/NFL" => options.log_file_names = false,
            "/EMPTY" => options.empty_files = true,
            "/CHILDONLY" => options.child_only = true,
            "/SHRED" => options.shred_files = true,
            _ => {
                if arg.starts_with("/A+:") {
                    options.attributes_add = arg[4..].to_string();
                } else if arg.starts_with("/A-:") {
                    options.attributes_remove = arg[4..].to_string();
                } else if arg.starts_with("/MT") {
                    let threads = if arg.len() > 4 && arg.chars().nth(3) == Some(':') {
                        arg[4..].parse::<usize>().unwrap_or(8)
                    } else {
                        8
                    };
                    options.threads = threads;
                } else if arg.starts_with("/R:") {
                    let retries = arg[3..].parse::<usize>().unwrap_or(1_000_000);
                    options.retries = retries;
                } else if arg.starts_with("/W:") {
                    let wait = arg[3..].parse::<u64>().unwrap_or(30);
                    options.wait_time = wait;
                } else if arg.starts_with("/LOG:") {
                    options.log_file = Some(arg[5..].to_string());
                }
            }
        }
    }

    // Initialize a log file if specified
    let mut log_file = if let Some(log_path) = &options.log_file {
        Some(File::create(log_path)?)
    } else {
        None
    };

    // Log start message
    let start_time = SystemTime::now();
    let start_msg = format!(
        "-------------------------------------------------------------------------------\n\
         ColemenCopy - Started: {}\n\
         Source: {}\n\
         Destination: {}\n\
         {}\n\
         Options: {}\n\
         -------------------------------------------------------------------------------\n",
        format_time(start_time),
        source_dir,
        dest_dir,
        if let Some(pattern) = &file_pattern {
            format!("Pattern: {}", pattern)
        } else {
            "Pattern: *.*".to_string()
        },
        format_options(&options)
    );

    println!("{}", start_msg);
    if let Some(log) = &mut log_file {
        log.write_all(start_msg.as_bytes())?;
    }

    // Create destination directory if it doesn't exist
    let dest_path = Path::new(dest_dir);
    if !dest_path.exists() {
        if !options.list_only {
            log_message(&mut log_file, &format!("Creating destination directory: {}", dest_dir));
            fs::create_dir_all(dest_path)?;
        } else {
            log_message(&mut log_file, &format!("Would create destination directory: {}", dest_dir));
        }
    }

    // Perform the copy operation
    let mut stats = Statistics::default();

    // Handle child-only mode
    if options.child_only && source_path.is_dir() {
        // Process each child directory individually
        if let Ok(entries) = fs::read_dir(source_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let child_path = entry.path();
                    if child_path.is_dir() {
                        let child_name = child_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let child_dest = dest_path.join(&child_name);

                        // Log the child directory processing
                        log_message(
                            &mut log_file,
                            &format!("\nProcessing child directory: {}", child_name)
                        );

                        // Process this child directory
                        copy_directory(
                            &child_path,
                            &child_dest,
                            &file_pattern,
                            &options,
                            &mut log_file,
                            &mut stats,
                        )?;
                    }
                }
            }
        }
    } else {
        // Regular mode - process the entire source directory
        copy_directory(
            source_path,
            dest_path,
            &file_pattern,
            &options,
            &mut log_file,
            &mut stats,
        )?;
    }

    // Log completion message
    let end_time = SystemTime::now();
    let elapsed = end_time.duration_since(start_time).unwrap_or(Duration::from_secs(0));

    let summary = format!(
        "-------------------------------------------------------------------------------\n\
         ColemenCopy - Finished: {}\n\
         Source: {}\n\
         Destination: {}\n\n\
         Statistics:\n\
             Directories: {}\n\
             Files: {}\n\
             Bytes: {}\n\
             Directories skipped: {}\n\
             Files skipped: {}\n\
             Files failed: {}\n\
             Directories removed: {}\n\
             Files removed: {}\n\n\
         Elapsed time: {} seconds\n\
         -------------------------------------------------------------------------------\n",
        format_time(end_time),
        source_dir,
        dest_dir,
        stats.dirs_created,
        stats.files_copied,
        stats.bytes_copied,
        stats.dirs_skipped,
        stats.files_skipped,
        stats.files_failed,
        stats.dirs_removed,
        stats.files_removed,
        elapsed.as_secs()
    );

    println!("{}", summary);
    if let Some(log) = &mut log_file {
        log.write_all(summary.as_bytes())?;
    }

    Ok(())
}

fn print_usage(program_name: &str) {
    println!("Usage: {} <source> <destination> [<file_pattern>] [options]", program_name);
    println!("Options:");
    println!("  /S         - Copy subdirectories, but not empty ones");
    println!("  /E         - Copy subdirectories, including empty ones");
    println!("  /Z         - Copy files in restartable mode (slower but more robust)");
    println!("  /B         - Copy files in Backup mode (overrides permissions)");
    println!("  /PURGE     - Delete destination files/folders that no longer exist in source");
    println!("  /MIR       - Mirror directory tree (like /PURGE plus all subdirectories)");
    println!("  /MOV       - Move files (delete from source after copying)");
    println!("  /MOVE      - Move files and directories (delete from source after copying)");
    println!("  /A+:[RASHCNETO] - Add specified attributes to copied files");
    println!("  /A-:[RASHCNETO] - Remove specified attributes from copied files");
    println!("  /MT[:n]    - Multithreaded copying with n threads (default is 8)");
    println!("  /R:n       - Number of retries on failed copies (default is 1 million)");
    println!("  /W:n       - Wait time between retries in seconds (default is 30)");
    println!("  /LOG:file  - Output log to file");
    println!("  /L         - List only - don't copy, timestamp or delete any files");
    println!("  /NP        - No progress - don't display % copied");
    println!("  /NFL       - No file list - don't log file names");
    println!("  /EMPTY     - Create empty (zero-byte) copies of files");
    println!("  /CHILDONLY - Process only direct child folders of source path");
    println!("  /SHRED     - Securely overwrite files before deletion");
}

fn format_time(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0));
    let secs = duration.as_secs();

    let (hour, remainder) = (secs / 3600, secs % 3600);
    let (min, sec) = (remainder / 60, remainder % 60);

    format!("{:02}:{:02}:{:02}", hour % 24, min, sec)
}

fn format_options(options: &CopyOptions) -> String {
    let mut result = Vec::new();

    if options.recursive {
        if options.include_empty {
            result.push("/E".to_string());
        } else {
            result.push("/S".to_string());
        }
    }

    if options.restartable {
        result.push("/Z".to_string());
    }

    if options.backup_mode {
        result.push("/B".to_string());
    }

    if options.mirror {
        result.push("/MIR".to_string());
    } else if options.purge {
        result.push("/PURGE".to_string());
    }

    if options.move_dirs {
        result.push("/MOVE".to_string());
    } else if options.move_files {
        result.push("/MOV".to_string());
    }

    if !options.attributes_add.is_empty() {
        result.push(format!("/A+:{}", options.attributes_add));
    }

    if !options.attributes_remove.is_empty() {
        result.push(format!("/A-:{}", options.attributes_remove));
    }

    if options.threads != 8 {
        result.push(format!("/MT:{}", options.threads));
    }

    if options.retries != 1_000_000 {
        result.push(format!("/R:{}", options.retries));
    }

    if options.wait_time != 30 {
        result.push(format!("/W:{}", options.wait_time));
    }

    if options.list_only {
        result.push("/L".to_string());
    }

    if !options.show_progress {
        result.push("/NP".to_string());
    }

    if !options.log_file_names {
        result.push("/NFL".to_string());
    }

    if options.empty_files {
        result.push("/EMPTY".to_string());
    }

    if options.child_only {
        result.push("/CHILDONLY".to_string());
    }

    if options.shred_files {
        result.push("/SHRED".to_string());
    }

    result.join(" ")
}

fn log_message(log_file: &mut Option<File>, message: &str) {
    println!("{}", message);
    if let Some(log) = log_file {
        let _ = writeln!(log, "{}", message);
    }
}

fn matches_pattern(entry_name: &str, pattern: &Option<String>) -> bool {
    if let Some(pattern_str) = pattern {
        // Very simple pattern matching - supports only * wildcard
        // For a more robust solution, use a proper glob crate
        if pattern_str == "*" || pattern_str == "*.*" {
            return true;
        }

        let file_name = Path::new(entry_name).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if pattern_str.starts_with('*') && pattern_str.ends_with('*') {
            // *contains*
            let substr = &pattern_str[1..pattern_str.len() - 1];
            file_name.contains(substr)
        } else if pattern_str.starts_with('*') {
            // *ends_with
            let suffix = &pattern_str[1..];
            file_name.ends_with(suffix)
        } else if pattern_str.ends_with('*') {
            // starts_with*
            let prefix = &pattern_str[..pattern_str.len() - 1];
            file_name.starts_with(prefix)
        } else {
            // exact match
            file_name == pattern_str
        }
    } else {
        true
    }
}

fn should_copy_file(src_meta: &Metadata, dst_meta: Option<&Metadata>) -> bool {
    // If destination doesn't exist, copy
    if dst_meta.is_none() {
        return true;
    }

    let dst_meta = dst_meta.unwrap();

    // If source is newer, copy
    let src_modified = src_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let dst_modified = dst_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

    if src_modified > dst_modified {
        return true;
    }

    // If same timestamp but different size, copy
    if src_modified == dst_modified && src_meta.len() != dst_meta.len() {
        return true;
    }

    false
}

fn copy_file(
    src_path: &Path,
    dst_path: &Path,
    options: &CopyOptions,
    log_file: &mut Option<File>,
    stats: &mut Statistics,
) -> io::Result<()> {
    let src_meta = fs::metadata(src_path)?;
    let dst_meta = fs::metadata(dst_path).ok();

    if !should_copy_file(&src_meta, dst_meta.as_ref()) {
        if options.log_file_names {
            log_message(log_file, &format!("Skipping identical file: {}", dst_path.display()));
        }
        stats.files_skipped += 1;
        return Ok(());
    }

    if options.list_only {
        log_message(log_file, &format!("Would copy file: {} -> {}", src_path.display(), dst_path.display()));
        stats.files_copied += 1;
        stats.bytes_copied += src_meta.len();
        return Ok(());
    }

    if options.log_file_names {
        log_message(log_file, &format!("Copying file: {} -> {}", src_path.display(), dst_path.display()));
    }

    let mut retry_count = 0;
    loop {
        match copy_file_with_progress(src_path, dst_path, src_meta.len(), options) {
            Ok(_) => {
                // Preserve file modification time
                if let Ok(src_time) = src_meta.modified() {
                    let _ = filetime::set_file_mtime(dst_path, filetime::FileTime::from_system_time(src_time));
                }

                // Set/unset attributes if specified
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;

                    if !options.attributes_add.is_empty() || !options.attributes_remove.is_empty() {
                        if let Ok(metadata) = fs::metadata(dst_path) {
                            let mut attributes = metadata.file_attributes();

                            // Add attributes
                            for c in options.attributes_add.chars() {
                                match c {
                                    'R' => attributes |= 0x00000001, // FILE_ATTRIBUTE_READONLY
                                    'A' => attributes |= 0x00000020, // FILE_ATTRIBUTE_ARCHIVE
                                    'S' => attributes |= 0x00000004, // FILE_ATTRIBUTE_SYSTEM
                                    'H' => attributes |= 0x00000002, // FILE_ATTRIBUTE_HIDDEN
                                    'C' => attributes |= 0x00000800, // FILE_ATTRIBUTE_COMPRESSED
                                    'N' => attributes |= 0x00000080, // FILE_ATTRIBUTE_NORMAL
                                    _ => {}
                                }
                            }

                            // Remove attributes
                            for c in options.attributes_remove.chars() {
                                match c {
                                    'R' => attributes &= !0x00000001,
                                    'A' => attributes &= !0x00000020,
                                    'S' => attributes &= !0x00000004,
                                    'H' => attributes &= !0x00000002,
                                    'C' => attributes &= !0x00000800,
                                    'N' => attributes &= !0x00000080,
                                    _ => {}
                                }
                            }

                            // Apply attributes
                            let _ = std::process::Command::new("attrib")
                                .arg(format!("+{}", attributes))
                                .arg(dst_path.to_string_lossy().to_string())
                                .output();
                        }
                    }
                }

                // Move (delete source) if requested
                if options.move_files {
                    if options.shred_files {
                        securely_delete_file(src_path, log_file)?;
                    } else {
                        let _ = fs::remove_file(src_path);
                    }
                }

                stats.files_copied += 1;
                stats.bytes_copied += src_meta.len();
                break;
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= options.retries {
                    log_message(log_file, &format!("Failed to copy after {} retries: {} -> {}, Error: {}",
                        options.retries, src_path.display(), dst_path.display(), e));
                    stats.files_failed += 1;
                    return Err(e);
                }

                log_message(log_file, &format!("Retry {} of {}: {} -> {}, Error: {}",
                    retry_count, options.retries, src_path.display(), dst_path.display(), e));

                thread::sleep(Duration::from_secs(options.wait_time));
            }
        }
    }

    Ok(())
}

fn copy_file_with_progress(
    src_path: &Path,
    dst_path: &Path,
    total_size: u64,
    options: &CopyOptions
) -> io::Result<()> {
    // If empty_files option is enabled, just create an empty file
    if options.empty_files {
        let mut dst_file = File::create(dst_path)?;
        dst_file.flush()?;
        return Ok(());
    }

    // Rest of the existing function for normal copying
    const BUFFER_SIZE: usize = 64 * 1024; // 64 KB buffer

    let mut src_file = File::open(src_path)?;
    let mut dst_file = File::create(dst_path)?;

    let mut buffer = [0; BUFFER_SIZE];
    let mut bytes_copied: u64 = 0;
    let mut last_progress = 0;

    loop {
        let bytes_read = src_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        dst_file.write_all(&buffer[..bytes_read])?;

        // If restartable mode is enabled, flush after each write
        if options.restartable {
            dst_file.flush()?;
        }

        bytes_copied += bytes_read as u64;

        // Show progress
        if options.show_progress && total_size > 0 {
            let progress = ((bytes_copied * 100) / total_size) as usize;
            if progress > last_progress {
                print!("\rCopying: {}% complete", progress);
                io::stdout().flush()?;
                last_progress = progress;
            }
        }
    }

    if options.show_progress && total_size > 0 {
        println!("\rCopying: 100% complete");
    }

    dst_file.flush()?;

    Ok(())
}

fn copy_directory(
    src_dir: &Path,
    dst_dir: &Path,
    file_pattern: &Option<String>,
    options: &CopyOptions,
    log_file: &mut Option<File>,
    stats: &mut Statistics,
) -> io::Result<()> {
    // Ensure the destination directory exists
    if !dst_dir.exists() {
        if !options.list_only {
            log_message(log_file, &format!("Creating directory: {}", dst_dir.display()));
            fs::create_dir_all(dst_dir)?;
            stats.dirs_created += 1;
        } else {
            log_message(log_file, &format!("Would create directory: {}", dst_dir.display()));
            stats.dirs_created += 1;
        }
    }

    // Collect the source files and directories
    let mut src_entries = HashSet::new();
    let entries = fs::read_dir(src_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        if path.is_file() {
            if matches_pattern(&file_name, file_pattern) {
                src_entries.insert(file_name.clone());

                let dst_path = dst_dir.join(&file_name);
                copy_file(&path, &dst_path, options, log_file, stats)?;
            }
        } else if path.is_dir() && options.recursive {
            src_entries.insert(file_name.clone());

            let dst_subdir = dst_dir.join(&file_name);

            // Skip empty directories if not including them
            if !options.include_empty {
                let is_empty = path.read_dir()?.next().is_none();
                if is_empty {
                    if options.log_file_names {
                        log_message(log_file, &format!("Skipping empty directory: {}", path.display()));
                    }
                    stats.dirs_skipped += 1;
                    continue;
                }
            }

            copy_directory(&path, &dst_subdir, file_pattern, options, log_file, stats)?;

            // Move (delete source dir) if requested
            if options.move_dirs && !options.list_only {
                let is_empty = path.read_dir()?.next().is_none();
                if is_empty {
                    let _ = fs::remove_dir(&path);
                }
            }
        }
    }

    // Purge files/directories in destination that don't exist in source
    if (options.purge || options.mirror) && !options.list_only {
        if let Ok(entries) = fs::read_dir(dst_dir) {
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy().to_string();

                if !src_entries.contains(&file_name) {
                    if path.is_file() {
                        if options.shred_files {
                            log_message(log_file, &format!("Securely removing file: {}", path.display()));
                            securely_delete_file(&path, log_file)?;
                        } else {
                            log_message(log_file, &format!("Removing file: {}", path.display()));
                            fs::remove_file(&path)?;
                        }
                        stats.files_removed += 1;
                    } else if path.is_dir() {
                        // For directories, recursively handle if shredding is enabled
                        if options.shred_files {
                            log_message(log_file, &format!("Securely removing directory: {}", path.display()));
                            secure_remove_dir_all(&path, log_file)?;
                        } else {
                            log_message(log_file, &format!("Removing directory: {}", path.display()));
                            fs::remove_dir_all(&path)?;
                        }
                        stats.dirs_removed += 1;
                    }
                }
            }
        }
    }

    Ok(())
}

fn securely_delete_file(path: &Path, log_file: &mut Option<File>) -> io::Result<()> {
    // Get the file size
    let metadata = fs::metadata(path)?;
    let file_size = metadata.len();

    // Open the file for writing
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(path)?;

    // Buffer for overwriting
    const BUFFER_SIZE: usize = 64 * 1024; // 64 KB

    // Multiple overwrite passes with different patterns
    let patterns = [
        0xFF, // All ones
        0x00, // All zeros
        0xAA, // 10101010
        0x55, // 01010101
        0xF0, // 11110000
        0x0F, // 00001111
    ];

    let mut buffer = vec![0; BUFFER_SIZE];

    for &pattern in &patterns {
        // Fill buffer with the current pattern
        for i in 0..BUFFER_SIZE {
            buffer[i] = pattern;
        }

        // Seek to beginning of file
        file.seek(io::SeekFrom::Start(0))?;

        // Write the pattern over the entire file
        let mut remaining = file_size;
        while remaining > 0 {
            let to_write = std::cmp::min(remaining, BUFFER_SIZE as u64) as usize;
            file.write_all(&buffer[..to_write])?;
            remaining -= to_write as u64;
        }

        // Flush to ensure data is written
        file.flush()?;
    }

    // Final pass with random data
    let mut rng = thread_rng();
    for i in 0..BUFFER_SIZE {
        buffer[i] = rng.gen_range(0..=255);
    }

    file.seek(io::SeekFrom::Start(0))?;

    let mut remaining = file_size;
    while remaining > 0 {
        let to_write = std::cmp::min(remaining, BUFFER_SIZE as u64) as usize;
        file.write_all(&buffer[..to_write])?;
        remaining -= to_write as u64;
    }

    file.flush()?;

    // Close the file
    drop(file);

    // Now delete the file
    fs::remove_file(path)?;

    log_message(log_file, &format!("Securely deleted file: {}", path.display()));

    Ok(())
}

fn secure_remove_dir_all(dir: &Path, log_file: &mut Option<File>) -> io::Result<()> {
    // First, recursively shred all files in subdirectories
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                secure_remove_dir_all(&path, log_file)?;
            } else {
                securely_delete_file(&path, log_file)?;
            }
        }

        // Remove the now-empty directory
        fs::remove_dir(dir)?;
        log_message(log_file, &format!("Removed directory after secure file deletion: {}", dir.display()));
    }

    Ok(())
}
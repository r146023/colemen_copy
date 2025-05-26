# Colemen_copy: A Cross-Platform Alternative to Robocopy

Colemen_copy is a robust file copying utility written in Rust that provides similar functionality to Microsoft's Robocopy (Robust File Copy) but works across multiple platforms including Linux, macOS, and Windows.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Cross-Platform Compatibility**: Works on Linux, macOS, and Windows
- **Directory Mirroring**: Complete directory tree synchronization
- **Robust Copying**: Automatic retries for failed operations
- **File Filtering**: Include/exclude files based on patterns
- **Progress Tracking**: Real-time copying progress information
- **Detailed Logging**: Comprehensive logging of all operations
- **Move Operations**: Support for moving files instead of just copying
- **Attribute Preservation**: Maintains file timestamps and attributes
- **Empty Files Mode**: Create zero-byte copies to recreate directory structures efficiently
- **Child Directory Mode**: Process only direct child folders of source path
- **Secure Deletion**: Securely overwrite files before deletion to prevent data recovery

## Installation

### From Source

1. Ensure you have Rust and Cargo installed:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/colemen_copy.git
   cd colemen_copy
   ```

3. Build the project:
   ```bash
   cargo build --release
   ```

4. The executable will be available at `target/release/colemen_copy`

## Usage

```
colemen_copy <source> <destination> [file_pattern] [options]
```

### Basic Examples

```bash
# Copy all files from source to destination
./colemen_copy /path/to/source /path/to/destination

# Copy all .jpg files
./colemen_copy /path/to/source /path/to/destination *.jpg

# Mirror a directory tree, including empty directories
./colemen_copy /path/to/source /path/to/destination /MIR

# Copy with progress display, retries, and logging
./colemen_copy /path/to/source /path/to/destination /Z /R:5 /W:10 /LOG:copy_log.txt

# Create empty (zero-byte) copies of all files
./colemen_copy /path/to/source /path/to/destination /EMPTY /E

# Process only direct child folders of the source directory
./colemen_copy /path/to/source /path/to/destination /CHILDONLY

# Securely delete files in destination that don't exist in source
./colemen_copy /path/to/source /path/to/destination /PURGE /SHRED
```

## Command Line Options

| Option | Description |
|--------|-------------|
| `/S` | Copy subdirectories, but not empty ones |
| `/E` | Copy subdirectories, including empty ones |
| `/Z` | Copy files in restartable mode (slower but more robust) |
| `/B` | Copy files in Backup mode (overrides file/folder permissions) |
| `/PURGE` | Delete destination files/folders that no longer exist in source |
| `/MIR` | Mirror directory tree (like `/PURGE` plus all subdirectories) |
| `/MOV` | Move files (delete from source after copying) |
| `/MOVE` | Move files and directories (delete from source after copying) |
| `/A+:[RASHCNETO]` | Add specified attributes to copied files |
| `/A-:[RASHCNETO]` | Remove specified attributes from copied files |
| `/MT[:n]` | Multithreaded copying with n threads (default is 8) |
| `/R:n` | Number of retries on failed copies (default is 1 million) |
| `/W:n` | Wait time between retries in seconds (default is 30) |
| `/LOG:file` | Output log to file |
| `/L` | List only - don't copy, timestamp or delete any files |
| `/NP` | No progress - don't display % copied |
| `/NFL` | No file list - don't log file names |
| `/EMPTY` | Create empty (zero-byte) copies of files |
| `/CHILDONLY` | Process only direct child folders of source path |
| `/SHRED` | Securely overwrite files before deletion |

## File Pattern Syntax

Colemen_copy supports simple wildcard patterns:

- `*` - Matches any number of any characters
- `*word*` - Matches any string containing "word"
- `word*` - Matches any string starting with "word"
- `*word` - Matches any string ending with "word"
- `*.ext` - Matches any file with the extension ".ext"

## Understanding the Output

When Colemen_copy runs, it provides statistics in the following format:

```
-------------------------------------------------------------------------------
Colemen_copy - Finished: HH:MM:SS
Source: /path/to/source
Destination: /path/to/destination

Statistics:
    Directories: X
    Files: Y
    Bytes: Z
    Directories skipped: A
    Files skipped: B
    Files failed: C
    Directories removed: D
    Files removed: E

Elapsed time: N seconds
-------------------------------------------------------------------------------
```

## Advanced Usage

### Mirroring a Directory Tree

To make the destination exactly match the source (including deleting files in destination that don't exist in source):

```bash
./colemen_copy /path/to/source /path/to/destination /MIR
```

---

### Multithreaded Copying

For faster operations on multi-core systems:

```bash
./colemen_copy /path/to/source /path/to/destination /MT:16
```

---

### Retry Logic for Network Shares

When copying across unreliable networks:

```bash
./colemen_copy /path/to/source /path/to/destination /Z /R:100 /W:30
```

---

### Moving Files

To move files instead of copying them:

```bash
./colemen_copy /path/to/source /path/to/destination /MOV
```

---


### Copying Specific File Types

To copy only specific file types:

```bash
./colemen_copy /path/to/source /path/to/destination *.jpg *.png *.gif /S
```

---


### Secure File Deletion

When removing files (either with `/PURGE`, `/MIR`, or when moving files with `/MOV` or `/MOVE`), you can ensure the files are securely deleted to prevent data recovery:

```bash
./colemen_copy /path/to/source /path/to/destination /MIR /SHRED
```

> Note that secure deletion significantly increases the time required for operations that delete files, as each file must be overwritten multiple times before deletion.



---

### Creating Directory Structure with Empty Files

To recreate a directory structure without copying the actual file contents (useful for testing or planning):

```bash
./colemen_copy /path/to/source /path/to/destination /EMPTY /E
```

This creates all the same directories and files, but the files are empty (zero bytes), saving disk space while preserving the structure.


## Use Cases for Empty Files Mode

To recreate a directory structure without copying the actual file contents (useful for testing or planning):

```bash
./colemen_copy /path/to/source /path/to/destination /EMPTY /E
```

The `/EMPTY` option is particularly useful for:

1. **Testing directory structures**: Verify paths and permissions without transferring large amounts of data
2. **Planning migrations**: Set up the target structure to analyze space requirements and permissions
3. **Template creation**: Create skeleton file structures that will be populated later
4. **Backup preparation**: Create a directory structure quickly before prioritizing which files to back up
5. **Low bandwidth environments**: When you need to recreate a file structure remotely but have limited bandwidth


---

### Processing Only Direct Child Folders

To process only the direct child folders of the source path:

```bash
./colemen_copy /path/to/source /path/to/destination /CHILDONLY
```

This option is useful when:
1. Organizing collections of folders that need individual processing
2. Working with media libraries where each subfolder is a separate project or album
3. Batch processing a set of distinct folders without recursing into their hierarchies
4. Handling first-level categorized data that needs to be copied separately


### Example
Lets say this is your source directory:
```
Source/
├── ProjectA/
│   ├── docs/
│   │   └── manual.pdf
│   ├── src/
│   │   ├── main.c
│   │   └── helper.c
│   └── README.md
├── ProjectB/
│   ├── images/
│   │   ├── logo.png
│   ├── website/
│   │   └── index.html
│   └── README.md
├── ProjectC/
│   ├── data/
│   │   └── sample.csv
│   └── scripts/
│        └── analyze.py
└── ThisFileIsIgnored.md
```

This is the destination:
```
Destination/
├── ProjectA/
│   ├── docs/
│   │   └── manual.pdf
│   ├── src/
│   │   ├── main.c
│   │   └── helper.c
│   └── README.md
├── EXTRA_THING/
├── ProjectB/
│   ├── images/
│   │   ├── logo.png
│   │   └── banner.jpg
│   ├── website/
│   │   └── index.html
│   └── README.md
```
When you run the command:
```bash
./colemen_copy /path/to/Source /path/to/Destination /MIR /CHILDONLY
```
It will go through ProjectA,ProjectB and ProjectC to synchronize everything within those directories.
It will completely ignore `ThisFileIsIgnored.md` and it will **NOT** delete or modify the `EXTRA_THING` directory in the destination.

The `banner.jpg` in ProjectB will be deleted because it does not exist in the source.

This is very useful for synchronizing a folder where each child needs to be handled individually.




---

## Comparison with Other Tools

### Colemen_copy vs Robocopy

- **Advantages**: Cross-platform compatibility, Rust safety guarantees
- **Limitations**: Some advanced Robocopy features may not be implemented

### Colemen_copy vs rsync

- **Advantages**: Windows support, Robocopy-like syntax for Windows users
- **Limitations**: Not as optimized for network transfers as rsync

## Error Handling

Colemen_copy includes comprehensive error handling and will:

1. Retry failed operations according to specified retry parameters
2. Log all errors encountered during copying
3. Provide a summary of successful and failed operations
4. Return appropriate exit codes for scripting

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Inspired by Microsoft's Robocopy utility
- Built with Rust for performance and safety
- Thanks to all contributors who have helped improve this tool

---

*Colemen_copy is not affiliated with Microsoft or Robocopy. It is an independent implementation providing similar functionality.*


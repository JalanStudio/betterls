// imports
use std::{fs, path::{Path, PathBuf}};
use chrono::{DateTime, Utc};  // date/time parsing & formatting
use clap::Parser;  // terminal argument parser
use owo_colors::OwoColorize;  // colored text
use serde::Serialize;  // structs -> json
use strum::Display;  // format Enum variants as  strings easily
use tabled::{Table, Tabled, settings::Style};  // print ASCII tables in terminals

// Structures
// #[derive(...)]: Debug(for printing with {:?}); Display(for printing with {} (from strum)); Serialize(for converting to JSON)
#[derive(Debug, Display, Serialize)]
enum FileType { File, Directory}

#[derive(Debug, Tabled, Serialize)] 
struct FileMetadata {
    // The #[tabled(rename = "...")] attribute changes the column header name in the output table.
    #[tabled(rename="Name")] name: String,
    #[tabled(rename="Type")] ftype: FileType,
    #[tabled(rename="Size")] size: String,
    #[tabled(rename="Last Modified")] modified: String
}

#[derive(Debug, Parser)]
#[command(version, about="better ls", long_about="better version of the commonly used command `ls`")]
struct Cli {
    path: Option<PathBuf>,
    #[arg(short, long)] json: bool
}

fn main() {
    let cli = Cli::parse();  // Parses args passed in terminal
    let path = cli.path.unwrap_or(PathBuf::from("."));
    if let Ok(does_exist) = fs::exists(&path) {  // checks if path exists; fs::exists is the newer api
        if does_exist {
            let files = fetchfiles(&path);  // gets the list of 'FileMetadata' objects
            if files.is_empty() { println!("{}", "The folder is empty".red()); } else {
                let mut f_table = Table::new(files);  // Create a new Table from the vector of files.
                f_table.with(Style::rounded());  // Apply a rounded visual style to the table borders.
                if cli.json {
                    // Serialize the data again to JSON string and print it.
                    // We call fetchfiles(&path) again here, which is slightly inefficient (fetching twice), but safe.
                    println!("{}", serde_json::to_string(&fetchfiles(&path)).unwrap_or("Can't parse json".to_string()));
                } else { println!("{}", f_table); }
            }
        } else { println!("{}", "Path does not exist.".red()); }
    } else { println!("{}", "Error reading directory.".red()); }  // If the operating system failed to check the directory
}

fn fetchfiles(path: &Path) -> Vec<FileMetadata> {
    let mut data = Vec::default(); // Initialize empty vector
    if let Ok(content) = fs::read_dir(path) {
        for entry in content {  // Loop through every entry in the directory.
            if let Ok(file) = entry {  // Check if the entry is valid (not a corrupted link or read error).
                if let Ok(meta) = fs::metadata(&file.path()) {  // Get the metadata (size, permissions, etc.) for the specific file.
                    data.push(FileMetadata {  // Create our custom struct and push it into the vector.
                        name: file.file_name().into_string().unwrap_or("???".into()),
                        ftype: if meta.is_dir() { FileType::Directory } else { FileType::File },
                        size: if meta.is_dir() {
                            // If it's a directory, we must check if it's empty or calculate recursive size.
                            match is_dir_empty(&file.path()) {
                                Ok(true) => "0B".to_string(), // Empty dir
                                // If not empty, call recursive function `dir_size` and format the bytes.
                                Ok(false) => convert_binary_units(dir_size(&file.path())),
                                Err(_) => "0B".to_string() // Error reading dir
                            }
                        } else { convert_binary_units(meta.len()) },  // normal file

                        modified: if let Ok(modif) = meta.modified() {
                            let date: DateTime<Utc> = modif.into(); format!("{}", date.format("%a %e %b %y"))
                        } else { String::default() },
                    });
                }
            }
        }
    }
    data // Return
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {  // 'flatten' removes Err results, giving us only valid entries.
            let p = entry.path();
            if let Ok(meta) = fs::symlink_metadata(&p) {  // Use symlink_metadata so we don't follow symlinks (preventing infinite loops)
                if meta.is_dir() {
                    total += dir_size(&p);  // RECURSION: If this entry is a directory, call this function again on it.
                } else {
                    total += meta.len();  // If it's a file, add its size to the total.
                }
            }
        }
    }
    total
}

// checks if a directory has any children.
// It reads the directory and fetchs the 'next' item; If 'next()' is None, directory is empty
fn is_dir_empty(path: &Path) -> std::io::Result<bool> { Ok(fs::read_dir(path)?.next().is_none()) }

// converts raw bytes (u64) into readable strings (KB, MB, GB).
fn convert_binary_units(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    match size {
        b if b >= GB => format!("{:.2}GB", b as f64 / GB as f64), // Divide by GB and format to 2 decimal places
        b if b >= MB => format!("{:.2}MB", b as f64 / MB as f64),
        b if b >= KB => format!("{:.2}KB", b as f64 / KB as f64),
        b => format!("{b}B")
    }
}


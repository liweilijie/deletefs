use clap::{Parser, Subcommand};
use fs_extra::file::CopyOptions;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use walkdir::{DirEntry, WalkDir};
use threadpool::ThreadPool;
use crossbeam_channel::unbounded;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short = 'p', long)]
    path: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// delete sample file
    Del,
    /// trim name
    Trim { vchar: Option<String> },
}

fn main() {
    let args = Args::parse();

    println!("args:{:#?}", args);
    println!("Hello {}!", args.path);

    let trash_path = Path::new(&args.path).join("trash");
    // create trash dire
    if !trash_path.exists() {
        println!("not exist {} and create it.", trash_path.display());
        fs_extra::dir::create(&trash_path, false).unwrap();
    } else {
        println!("{} is exist.", trash_path.display());
    }

    let default_trims: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![
        String::from(r#"海量资源尽在：666java.com【海量资源： www.666java.com】"#),
        String::from(r#"【更多资源访问：  666java.com】"#),
        String::from(r#"【666资源站：666 java.com】"#),
        String::from(r#"【海量资源：666java.com】"#),
        String::from(r#"【海量一手：666java .com】"#),
        String::from(r#"【海量一手：666java.com】"#),
        String::from(r#"【666资源站：666java.com】"#),
        String::from(r#"海量资源尽在：666java.com"#),
        String::from(r#"海量资源：666java.com"#),
        String::from(r#"更多资源： www.666java.com"#),
        String::from(r#"【IT视频学习网-www.itspxx.com】"#),
    ]));

    let pool = ThreadPool::new(num_cpus::get());
    let (sender, receiver) = unbounded();
    let total_files= Arc::new(AtomicU64::new(0));
    let total_rename_files = Arc::new(AtomicU64::new(0));
    let start = Instant::now();


    match &args.command {
        Commands::Del => {
            let options = CopyOptions::new(); //Initialize default values for CopyOptions

            for entry in WalkDir::new(&args.path) {
                if entry.is_err() {
                    continue;
                }

                let entry = entry.unwrap();

                if entry.path_is_symlink() {
                    continue;
                }

                if is_hidden(&entry) {
                    continue;
                }

                if entry.path().display().to_string().contains("/trash") {
                    continue;
                }

                if Path::new(entry.path()).is_dir() {
                    continue;
                }

                if is_delete_file(&entry) {
                    // println!("delete: {}", entry.path().display());
                    let path = trash_path.clone().join(entry.file_name());
                    println!("from:{}\n\t to:{}", entry.path().display(), path.display());
                    fs_extra::file::move_file(entry.path(), path, &options).expect("error move");
                }
            }
        }

        Commands::Trim { vchar } => {
            if vchar.is_some() {
                let vchar = vchar.as_ref().unwrap().clone();
                default_trims.lock().unwrap().insert(0, vchar.clone());
                // (*default_trims).insert(0, vchar.clone());
                println!("vchar: {}, default_trims:{:?}", vchar, default_trims);
            }
            // 轮询目录
            for entry in WalkDir::new(&args.path) {
                if entry.is_err() {
                    continue
                }

                let entry = entry.unwrap();

                let entry = entry.clone();
                let sender = sender.clone();
                let default_trims = Arc::clone(&default_trims);
                let total_files = Arc::clone(&total_files);
                let total_rename_files = Arc::clone(&total_rename_files);
                pool.execute(move || {
                    if entry.path_is_symlink() {
                        return;
                    }

                    if is_hidden(&entry) {
                        return;
                    }

                    if entry.path().display().to_string().contains("/trash") {
                        return;
                    }

                    if Path::new(entry.path()).is_dir() {
                        return;
                    }

                    total_files.fetch_add(1, Ordering::Relaxed);
                    for v in &*default_trims.lock().unwrap() {
                        if rename_file(&entry, v) {
                            sender.send(entry.path().display().to_string()).expect("could not send data.");
                            total_rename_files.fetch_add(1, Ordering::Relaxed);
                            break;
                        }
                    }
                });
            }
        }
    }

    drop(sender);

    for t in receiver.iter() {
        println!("rename:{}", t);
    }

    println!("use:{} threads, total:{}, rename:{}, elapsed:{:?}", num_cpus::get(), total_files.load(Ordering::Relaxed), total_rename_files.load(Ordering::Relaxed), start.elapsed());
}

fn rename_file(entry: &DirEntry, vchar: &str) -> bool {
    return if entry.file_name().to_string_lossy().contains(&vchar) {
        let to = Path::new(entry.path().parent().unwrap()).join(
            entry
                .file_name()
                .to_string_lossy()
                .replace(&vchar, "")
                .trim(),
        );
        match fs::rename(entry.path(), to) {
            Err(e) => println!(
                "rename: {} error:{:?}",
                entry.file_name().to_string_lossy(),
                e
            ),
            Ok(_) => {}
        }
        true
    } else {
        false
    };
}

fn is_delete_file(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.ends_with(r#"(1).mp4"#))
        .unwrap_or(false)
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::println;

    // cargo test -- --nocapture -- this_test_rename_will_pass
    #[test]
    fn this_test_rename_will_pass() {
        let from = String::from("[2.4]--2-4基于Phoenix的RBAC权限模型【海量资源：666java.com】.mp4");
        let vchar = "666java";
        let to = from.replace(vchar, "");
        println!("from:{}, vchar:{}, to:{}", from, vchar, to);
    }
}

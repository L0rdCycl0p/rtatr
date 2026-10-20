use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use crate::error::{Result, RtatrError};
use crate::huid::Huid;
use crate::store::Store;
use crate::task::{Task, NEW_TASK_PRIORITY};

const TAG: &str = "TODO:";

pub fn run(store: &Store, files: &[PathBuf], id_suffix: Option<&str>) -> Result<()> {
    if files.is_empty() {
        return Err(RtatrError::Parse("no files were provided".into()));
    }
    for file in files {
        scan_file(store, file, id_suffix)?;
    }
    Ok(())
}

enum PromptResult {
    Yes,
    No,
    Quit,
    YesAll,
    NoAll,
}

fn scan_file(store: &Store, file: &Path, id_suffix: Option<&str>) -> Result<()> {
    let content = std::fs::read_to_string(file).map_err(|e| RtatrError::Io {
        path: file.to_path_buf(),
        source: e,
    })?;
    let mut lines: Vec<String> = content.split('\n').map(String::from).collect();
    let mut yes_entire_file = false;
    let mut skip_entire_file = false;

    let mut i = 0;
    while i < lines.len() {
        if skip_entire_file {
            break;
        }
        let line = lines[i].clone();
        let Some(pos) = line.find(TAG) else {
            i += 1;
            continue;
        };
        let prefix = &line[..pos];
        let suffix = &line[pos + TAG.len()..];
        let title = suffix.trim();
        let line_number = i + 1;

        println!("{}:{}: Found a TODO: {}", file.display(), line_number, title);
        let start = i.saturating_sub(2);
        let end = (i + 2).min(lines.len().saturating_sub(1));
        for (j, line) in lines.iter().enumerate().take(end+1).skip(start) {
            if j == i {
                println!("{prefix}TODO:{suffix}");
            } else {
                println!("{}", line);
            }
        }

        let create = if yes_entire_file {
            true
        } else {
            match prompt_create()? {
                PromptResult::Yes => true,
                PromptResult::No => {
                    i += 1;
                    continue;
                }
                PromptResult::Quit => {
                    write_back(file, &lines)?;
                    return Ok(());
                }
                PromptResult::YesAll => {
                    yes_entire_file = true;
                    true
                }
                PromptResult::NoAll => {
                    skip_entire_file = true;
                    continue;
                }
            }
        };
        if !create {
            i += 1;
            continue;
        }

        let huid = match id_suffix {
            Some(s) => Huid::now_with_suffix(Some(s.to_string())),
            None => store.next_huid()?,
        };
        let mut task = Task::new(
            huid.clone(),
            title.to_string(),
            NEW_TASK_PRIORITY,
            Vec::new(),
        );
        store.save(&mut task)?;
        lines[i] = format!("{prefix}TASK({huid}):{suffix}");
        write_back(file, &lines)?;
        i += 1;
    }
    Ok(())
}

fn write_back(file: &Path, lines: &[String]) -> Result<()> {
    let out = lines.join("\n");
    std::fs::write(file, out).map_err(|e| RtatrError::Io {
        path: file.to_path_buf(),
        source: e,
    })?;
    Ok(())
}

fn prompt_create() -> Result<PromptResult> {
    let opts = [
        ('y', "yes"),
        ('n', "no"),
        ('q', "quit"),
        ('a', "yes until the end of file"),
        ('c', "no until the end of file"),
    ];
    loop {
        print!("Create a task from it? [");
        for (c, _) in &opts {
            print!("{c},");
        }
        print!("?] ");
        std::io::stdout().flush().ok();
        let mut buf = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut buf)
            .map_err(|e| RtatrError::Io {
                path: PathBuf::from("<stdin>"),
                source: e,
            })?;
        let resp = buf.trim().to_lowercase();
        if resp == "?" {
            for (c, d) in &opts {
                println!("{c} - {d}");
            }
            continue;
        }
        match resp.as_str() {
            "y" => return Ok(PromptResult::Yes),
            "n" => return Ok(PromptResult::No),
            "q" => return Ok(PromptResult::Quit),
            "a" => return Ok(PromptResult::YesAll),
            "c" => return Ok(PromptResult::NoAll),
            _ => continue,
        }
    }
}
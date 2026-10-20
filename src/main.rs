use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use rtatr::error::{Result, RtatrError};
use rtatr::huid::Huid;
use rtatr::path::Paths;
use rtatr::query;
use rtatr::split_tags;
use rtatr::store::Store;
use rtatr::task::{Task, NEW_TASK_PRIORITY};

#[derive(Parser)]
#[command(name = "rtatr", version, about = "Rust Task Tracker")]
struct Cli {
    /// Root directory for the task store
    #[arg(long, env = "RTATR_ROOT", default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create the tasks/ directory in the current directory
    Init,

    /// List tasks matching a TQL query (open tasks only unless --closed)
    Ls {
        /// Query (e.g. `:work and priority lt 100`); empty = all
        query: Option<String>,
        /// List closed tasks
        #[arg(short, long)]
        closed: bool,
        /// Sort ascending (default: descending by priority)
        #[arg(short, long)]
        ascending: bool,
        /// Sort by HUID instead of priority
        #[arg(long = "id")]
        by_id: bool,
    },

    /// Create a new task
    #[command(alias = "add")]
    New {
        /// Tags for the task (comma and/or whitespace separated)
        #[arg(short, long)]
        tag: Vec<String>,
        /// Priority of the new task
        #[arg(short, long, default_value_t = NEW_TASK_PRIORITY)]
        priority: i32,
        /// Optional HUID suffix (e.g. `-s rexim` → `20240101-120000-rexim`)
        #[arg(short = 's', long = "suffix")]
        id_suffix: Option<String>,
        /// Task title (positional args are joined with spaces)
        title: Vec<String>,
    },

    /// Find a task by HUID (regardless of open/closed)
    Find {
        huid: String,
        /// Print only the path to TASK.md
        #[arg(long)]
        path_only: bool,
    },

    /// Print the HUID of the current task folder
    Id,

    /// Print a summary of tasks grouped by tag
    Summary {
        /// Summarise closed tasks instead of open ones
        #[arg(short, long)]
        closed: bool,
    },

    /// Update a task's tags
    Update {
        huid: String,
        #[arg(long)]
        add_tag: Vec<String>,
        #[arg(long)]
        remove_tag: Vec<String>,
    },

    /// Mark a task closed
    Done { huid: String },

    /// Delete a task
    Rm { huid: String },

    /// Print a task
    Show { huid: String },

    /// Print version
    Version,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let store = Store::new(Paths::from(cli.root));

    match cli.command {
        Command::Init => {
            store
                .paths()
                .ensure_tasks_dir()
                .map_err(|e| RtatrError::Io {
                    path: store.paths().tasks_dir(),
                    source: e,
                })?;
        }

        Command::Ls {
            query: q,
            closed,
            ascending,
            by_id,
        } => {
            let expr = query::parse(q.as_deref().unwrap_or("")).map_err(RtatrError::Query)?;
            let mut tasks: Vec<Task> = store
                .query(&expr)?
                .into_iter()
                .filter(|t| (t.status == "CLOSED") == closed)
                .collect();

            if by_id {
                tasks.sort_by(|a, b| b.huid.cmp(&a.huid));
            } else {
                tasks.sort_by_key(|a| std::cmp::Reverse(a.priority));
            }
            if ascending {
                tasks.reverse();
            }

            if tasks.is_empty() {
                println!("No tasks were found");
            } else {
                for task in &tasks {
                    println!(
                        "{}  {:<7}  {:>3}  {:<30}  {}",
                        task.huid,
                        task.status,
                        task.priority,
                        task.tags.join(","),
                        task.title,
                    );
                }
            }
        }

        Command::New {
            tag,
            priority,
            id_suffix,
            title,
        } => {
            let title = if title.is_empty() {
                "New Task".to_string()
            } else {
                title.join(" ")
            };
            let tags: Vec<String> = tag.iter().flat_map(|t| split_tags(t)).collect();

            let huid = match id_suffix {
                Some(s) => {
                    for c in s.chars() {
                        if !(c.is_ascii_alphanumeric() || c == '-') {
                            return Err(RtatrError::Parse(format!(
                                "invalid suffix character: {c:?}"
                            )));
                        }
                    }
                    let mut h = Huid::now_with_suffix(Some(s.clone()));
                    if store.paths().task_dir(&h).exists() {
                        // Upstream: sleep 1s and retry once.
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        h = Huid::now_with_suffix(Some(s));
                        if store.paths().task_dir(&h).exists() {
                            return Err(RtatrError::Parse(format!("HUID {h} already exists")));
                        }
                    }
                    h
                }
                None => store.next_huid()?,
            };

            let mut task = Task::new(huid.clone(), title, priority, tags);
            store.save(&mut task)?;
            println!("{huid}");
        }

        Command::Find { huid, path_only } => {
            let h: Huid = huid.parse().map_err(RtatrError::InvalidHuid)?;
            let task = store.get(&h)?;
            if path_only {
                println!("{}", task.path.display());
            } else {
                print_task_report(&task);
            }
        }

        Command::Id => match store.paths().current_task_huid() {
            Some(h) => print!("{h}"),
            None => {
                return Err(RtatrError::Parse(
                    "you are not inside any task folder".into(),
                ))
            }
        },

        Command::Summary { closed } => {
            let descriptions = read_tag_descriptions(store.paths());
            let mut total = 0usize;
            let mut untagged = 0usize;
            let mut tag_counts: HashMap<String, usize> = HashMap::new();

            for t in store.all()? {
                if (t.status == "CLOSED") != closed {
                    continue;
                }
                total += 1;
                if t.tags.is_empty() {
                    untagged += 1;
                }
                for tag in &t.tags {
                    *tag_counts.entry(tag.clone()).or_insert(0) += 1;
                }
            }

            let mut sorted: Vec<(String, usize)> = tag_counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let max_width = sorted.iter().map(|(t, _)| t.len()).max().unwrap_or(0);

            println!("STATUS:   {}", if closed { "CLOSED" } else { "OPEN" });
            println!("TOTAL:    {total}");
            if untagged > 0 {
                println!("UNTAGGED: {untagged}");
            }
            if !sorted.is_empty() {
                println!("TAGGED:");
                for (tag, count) in &sorted {
                    match descriptions.get(tag) {
                        Some(desc) => println!(
                            "    {:>width$} => {:3} - {}",
                            tag,
                            count,
                            desc,
                            width = max_width
                        ),
                        None => println!("    {:>width$} => {:3}", tag, count, width = max_width),
                    }
                }
            }
        }

        Command::Update {
            huid,
            add_tag,
            remove_tag,
        } => {
            let h: Huid = huid.parse().map_err(RtatrError::InvalidHuid)?;
            let mut task = store.get(&h)?;
            for t in add_tag {
                for tag in split_tags(&t) {
                    if !task.tags.contains(&tag) {
                        task.tags.push(tag);
                    }
                }
            }
            for t in remove_tag {
                for tag in split_tags(&t) {
                    task.tags.retain(|x| x != &tag);
                }
            }
            store.save(&mut task)?;
        }

        Command::Done { huid } => {
            let h: Huid = huid.parse().map_err(RtatrError::InvalidHuid)?;
            let mut task = store.get(&h)?;
            task.status = "CLOSED".into();
            store.save(&mut task)?;
        }

        Command::Rm { huid } => {
            let h: Huid = huid.parse().map_err(RtatrError::InvalidHuid)?;
            store.delete(&h)?;
        }

        Command::Show { huid } => {
            let h: Huid = huid.parse().map_err(RtatrError::InvalidHuid)?;
            let task = store.get(&h)?;
            println!("HUID:     {}", task.huid);
            println!("Title:    {}", task.title);
            println!("Status:   {}", task.status);
            println!("Priority: {}", task.priority);
            println!("Tags:     {}", task.tags.join(","));
            println!();
            print!("{}", task.body);
        }

        Command::Version => {
            println!("rtatr {}", env!("CARGO_PKG_VERSION"));
        }
    }
    Ok(())
}

/// Mirror of upstream `print_task_report` (emacs compilation-mode format).
fn print_task_report(task: &Task) {
    let tags = if task.tags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", task.tags.join(","))
    };
    println!(
        "{}:1: {} [PRIORITY: {:3}]{} {}",
        task.path.display(),
        task.status,
        task.priority,
        tags,
        task.title,
    );
}

fn read_tag_descriptions(paths: &Paths) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let path = paths.tasks_dir().join("tags");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return map;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, |c: char| c.is_whitespace() || c == ',');
        let tag = parts.next().unwrap_or("").to_string();
        if tag.is_empty() {
            continue;
        }
        let desc = parts
            .next()
            .unwrap_or("")
            .trim_start_matches(|c: char| c.is_whitespace() || c == ',')
            .trim_end()
            .to_string();
        map.entry(tag).or_insert(desc);
    }
    map
}

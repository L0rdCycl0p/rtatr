use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::engine::ArgValueCompleter;
use clap_complete::Shell;

use rtatr::error::{Result, RtatrError};
use rtatr::huid::Huid;
use rtatr::path::Paths;
use rtatr::query;
use rtatr::split_tags;
use rtatr::store::Store;
use rtatr::task::{Task, NEW_TASK_PRIORITY};

#[derive(Parser)]
#[command(name = "rtatr", version, about = "Rust Task Tracker")]
#[command(disable_help_subcommand = true)]
struct Cli {
    /// Project root containing `tasks/`. Falls back to `RTATR_ROOT`,
    /// then upward search from CWD, then `.`.
    #[arg(long, env = "RTATR_ROOT", value_hint = clap::ValueHint::DirPath)]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create the tasks/ directory
    Init,

    /// List and update tasks matching a TQL query
    Ls {
        /// Query (e.g. `:work and priority lt 100`); empty = all
        #[arg(add = ArgValueCompleter::new(rtatr::complete::complete_query))]
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
        /// Apply update operations. Comma- or space-separated list of
        /// `open`, `close`, `+tag`, `-tag`. Repeatable.
        #[arg(long, add = ArgValueCompleter::new(rtatr::complete::complete_update_ops))]
        update: Vec<String>,
        /// Print parsed query tokens and AST without listing tasks
        #[arg(long)]
        debug: bool,
    },

    /// Create a new task
    #[command(alias = "add")]
    New {
        #[arg(short, long, add = ArgValueCompleter::new(rtatr::complete::complete_tag_list))]
        tag: Vec<String>,
        #[arg(short, long, default_value_t = NEW_TASK_PRIORITY)]
        priority: i32,
        #[arg(short = 's', long = "suffix")]
        id_suffix: Option<String>,
        title: Vec<String>,
    },

    /// Create tasks from all the TODOs in the given files (interactive)
    Todo {
        #[arg(short = 's', long = "suffix")]
        id_suffix: Option<String>,
        #[arg(value_hint = clap::ValueHint::FilePath,
              add = ArgValueCompleter::new(rtatr::complete::complete_files))]
        files: Vec<PathBuf>,
    },

    /// Find a task by HUID
    Find {
        #[arg(add = ArgValueCompleter::new(rtatr::complete::complete_huids))]
        huid: String,
        #[arg(long)]
        path_only: bool,
    },

    /// Print the HUID of the current task folder
    Id,

    /// Grep the repo for references to a task HUID
    Ref {
        #[arg(add = ArgValueCompleter::new(rtatr::complete::complete_huids))]
        huid: Option<String>,
    },

    /// Generate a Graphviz graph of task cross-references
    Graph,

    /// Print a summary of tasks grouped by tag
    Summary {
        #[arg(short, long)]
        closed: bool,
    },

    /// Update a single task's tags (rtatr extension)
    Update {
        #[arg(add = ArgValueCompleter::new(rtatr::complete::complete_huids))]
        huid: String,
        #[arg(long, add = ArgValueCompleter::new(rtatr::complete::complete_tag_list))]
        add_tag: Vec<String>,
        #[arg(long, add = ArgValueCompleter::new(rtatr::complete::complete_tag_list))]
        remove_tag: Vec<String>,
    },

    /// Mark a task closed (rtatr extension)
    Done {
        #[arg(add = ArgValueCompleter::new(rtatr::complete::complete_huids))]
        huid: String,
    },

    /// Delete a task (rtatr extension)
    Rm {
        #[arg(add = ArgValueCompleter::new(rtatr::complete::complete_huids))]
        huid: String,
    },

    /// Print a task (rtatr extension)
    Show {
        #[arg(add = ArgValueCompleter::new(rtatr::complete::complete_huids))]
        huid: String,
    },

    /// DEPRECATED: use `rtatr ls -update +tag <query>` instead
    Tag {
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
        #[arg(short = 'c', long)]
        closed: bool,
        query: Vec<String>,
    },

    /// DEPRECATED: use `rtatr ls -update -tag <query>` instead
    Untag {
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
        #[arg(short = 'c', long)]
        closed: bool,
        query: Vec<String>,
    },

    /// Print help for the program or a specific command
    Help {
        #[arg(add = ArgValueCompleter::new(rtatr::complete::complete_subcommands))]
        command: Option<String>,
    },
    /// Generate a static shell completion script
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Print version
    Version,
}

fn main() {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();

    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
fn preprocess_args() -> Vec<String> {
    const LONG_FLAGS: &[&str] = &["help", "debug", "update", "path-only", "id"];

    let mut out = Vec::new();
    let mut after_dash_dash = false;
    for arg in std::env::args() {
        if after_dash_dash {
            out.push(arg);
            continue;
        }
        if arg == "--" {
            after_dash_dash = true;
            out.push(arg);
            continue;
        }
        if let Some(stripped) = arg.strip_prefix('-') {
            if !arg.starts_with("--") && LONG_FLAGS.contains(&stripped) {
                out.push(format!("--{stripped}"));
                continue;
            }
        }
        out.push(arg);
    }
    out
}
fn run() -> Result<()> {
    let cli = Cli::parse_from(preprocess_args());
    let paths = match cli.root {
        Some(r) => Paths::from(r),
        None => Paths::discover().unwrap_or_else(|| Paths::from(PathBuf::from("."))),
    };
    let store = Store::new(paths);

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
            update,
            debug,
        } => {
            let raw = q.as_deref().unwrap_or("");
            let q_src = if raw.trim().is_empty() { "any" } else { raw };
            let q_is_empty = raw.trim().is_empty();

            let parsed_q = query::parse(q_src).map_err(RtatrError::Query)?;
            let update_src = update.join(" ");
            let parsed_u = rtatr::parse_update(&update_src).map_err(RtatrError::Query)?;

            if debug {
                println!("QUERY TOKENS:");
                for tok in query::tokenize_for_debug(q_src).map_err(RtatrError::Query)? {
                    println!("    {tok}");
                }
                println!();
                println!("QUERY OPS:");
                query::print_qop(&parsed_q, "    ");
                if !parsed_u.is_empty() {
                    println!();
                    println!("UPDATE TOKENS:");
                    for tok in update_src
                        .split(|c: char| c.is_whitespace() || c == ',')
                        .filter(|s| !s.is_empty())
                    {
                        println!("    {tok}");
                    }
                    println!();
                    println!("UPDATE OPS:");
                    for op in &parsed_u {
                        print!("    ");
                        rtatr::update::print_uop(op);
                    }
                }
                return Ok(());
            }

            if !parsed_u.is_empty() && q_is_empty {
                return Err(RtatrError::Parse(
                    "-update with empty query is not allowed; use \"any\" query".into(),
                ));
            }

            let mut tasks: Vec<Task> = store
                .query(&parsed_q)?
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
                return Ok(());
            }

            let mut updated = 0usize;
            for task in &mut tasks {
                let was_updated = !parsed_u.is_empty() && rtatr::apply_update(task, &parsed_u);
                if was_updated {
                    store.save(task)?;
                    updated += 1;
                }
                let marker = was_updated.then_some(update_src.as_str());
                print_task_report(task, marker);
            }
            if updated > 0 {
                println!("{updated} tasks were updated");
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

        Command::Todo { id_suffix, files } => {
            rtatr::todo::run(&store, &files, id_suffix.as_deref())?;
        }

        Command::Find { huid, path_only } => {
            let h: Huid = huid.parse().map_err(RtatrError::InvalidHuid)?;
            let task = store.get(&h)?;
            if path_only {
                println!("{}", task.path.display());
            } else {
                print_task_report(&task, None);
            }
        }

        Command::Id => match store.paths().current_task_id() {
            Some(id) => print!("{id}"),
            None => {
                return Err(RtatrError::Parse(
                    "you are not inside any task folder".into(),
                ))
            }
        },

        Command::Ref { huid } => {
            let h: String = match huid {
                Some(s) => s
                    .parse::<Huid>()
                    .map_err(RtatrError::InvalidHuid)?
                    .to_string(),
                None => store.paths().current_task_id().ok_or_else(|| {
                    RtatrError::Parse("not inside any task folder; pass the HUID of a task".into())
                })?,
            };
            let root = store.paths().root.clone();
            let status = std::process::Command::new("grep")
                .arg("--exclude-dir=.git")
                .arg("-Irn")
                .arg(&h)
                .arg(&root)
                .status()
                .map_err(|e| RtatrError::Io {
                    path: root.clone(),
                    source: e,
                })?;
            if !status.success() && status.code() != Some(1) {
                return Err(RtatrError::Parse(format!("grep failed: {status:?}")));
            }
        }

        Command::Graph => rtatr::graph::run(&store)?,

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

        Command::Tag {
            tags,
            closed: _,
            query,
        } => {
            if tags.is_empty() {
                return Err(RtatrError::Parse("no tags are provided".into()));
            }
            if query.is_empty() {
                return Err(RtatrError::Parse("no query is provided".into()));
            }
            let alt = format!(
                "rtatr ls -update {} {}",
                tags.iter()
                    .map(|t| format!("+{t}"))
                    .collect::<Vec<_>>()
                    .join(","),
                query.join(" ")
            );
            return Err(RtatrError::Parse(format!(
                "This command is deprecated! Please use `{alt}` instead!"
            )));
        }

        Command::Untag {
            tags,
            closed: _,
            query,
        } => {
            if tags.is_empty() {
                return Err(RtatrError::Parse("no tags are provided".into()));
            }
            if query.is_empty() {
                return Err(RtatrError::Parse("no query is provided".into()));
            }
            let alt = format!(
                "rtatr ls -update {} {}",
                tags.iter()
                    .map(|t| format!("-{t}"))
                    .collect::<Vec<_>>()
                    .join(","),
                query.join(" ")
            );
            return Err(RtatrError::Parse(format!(
                "This command is deprecated! Please use `{alt}` instead!"
            )));
        }

        Command::Help { command } => {
            let mut cmd = Cli::command();
            let res = match command {
                Some(name) => match cmd.find_subcommand_mut(&name) {
                    Some(sub) => sub.print_help(),
                    None => {
                        return Err(RtatrError::Parse(format!("unknown command: {name}")));
                    }
                },
                None => cmd.print_help(),
            };
            res.map_err(|e| RtatrError::Io {
                path: PathBuf::from("<stdout>"),
                source: e,
            })?;
            println!();
        }
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "rtatr", &mut std::io::stdout());
        }
        Command::Version => {
            println!("rtatr {}", env!("CARGO_PKG_VERSION"));
        }
    }
    Ok(())
}

fn print_task_report(task: &Task, updated: Option<&str>) {
    let marker = match updated {
        Some(spec) => format!(" <UPDATED: {spec}>"),
        None => String::new(),
    };
    let tags = if task.tags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", task.tags.join(","))
    };
    println!(
        "{}:1:{} {} [PRIORITY: {:3}]{} {}",
        task.path.display(),
        marker,
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

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
    /// List tasks matching a TQL query (open tasks only unless --closed)
    Ls {
        /// Query (e.g. `:work and not :done`); empty = all
        query: Option<String>,
        /// List closed tasks instead of open ones
        #[arg(short, long)]
        closed: bool,
    },
    /// Add a new task
    Add {
        /// Tags for the task (comma and/or whitespace separated)
        #[arg(short, long)]
        tag: Vec<String>,
        /// Priority of the new task
        #[arg(short, long, default_value_t = NEW_TASK_PRIORITY)]
        priority: i32,
        /// Task title (positional args are joined with spaces)
        title: Vec<String>,
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
        Command::Ls { query: q, closed } => {
            let expr = query::parse(q.as_deref().unwrap_or("")).map_err(RtatrError::Query)?;
            let mut any = false;
            for task in store.query(&expr)? {
                if closed {
                    if task.status != "CLOSED" {
                        continue;
                    }
                } else if task.status == "CLOSED" {
                    continue;
                }
                any = true;
                println!(
                    "{}  {:<7}  {:>3}  {:<30}  {}",
                    task.huid,
                    task.status,
                    task.priority,
                    task.tags.join(","),
                    task.title,
                );
            }
            if !any {
                println!("No tasks were found");
            }
        }

        Command::Add {
            tag,
            priority,
            title,
        } => {
            let title = if title.is_empty() {
                "New Task".to_string()
            } else {
                title.join(" ")
            };
            let tags: Vec<String> = tag.iter().flat_map(|t| split_tags(t)).collect();
            let huid = store.next_huid()?;
            let mut task = Task::new(huid.clone(), title, priority, tags);
            store.save(&mut task)?;
            println!("{huid}");
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
    }
    Ok(())
}

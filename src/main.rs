use std::io::Read;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use rtatr::error::{Result, RtatrError};
use rtatr::huid::Huid;
use rtatr::path::Paths;
use rtatr::query;
use rtatr::store::Store;
use rtatr::task::Task;

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
    /// List tasks matching a TQL query
    Ls {
        /// Query (e.g. `:work and not :done`); empty = all
        query: Option<String>,
    },
    /// Add a new task
    Add {
        /// Tags for the task
        #[arg(short, long)]
        tag: Vec<String>,
        /// Task body (reads stdin when omitted)
        body: Option<String>,
    },
    /// Update a task's tags
    Update {
        huid: String,
        #[arg(long)]
        add_tag: Vec<String>,
        #[arg(long)]
        remove_tag: Vec<String>,
    },
    /// Mark a task done
    Done { huid: String },
    /// Delete a task
    Rm { huid: String },
    /// Print a task
    Show { huid: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = Store::new(Paths::new(cli.root));

    match cli.command {
        Command::Ls { query: q } => {
            let expr = query::parse(q.as_deref().unwrap_or("")).map_err(RtatrError::Query)?;
            for task in store.query(&expr)? {
                let tags: Vec<String> = task.tags.iter().map(|t| format!(":{t}")).collect();
                println!("{}  {:<30}  {}", task.huid, tags.join(" "), task.title());
            }
        }
        Command::Add { tag, body } => {
            let body = body.unwrap_or_else(|| {
                let mut s = String::new();
                std::io::stdin().read_to_string(&mut s).ok();
                s
            });
            let huid = store.next_huid()?;
            let mut task = Task {
                huid: huid.clone(),
                tags: tag
                    .into_iter()
                    .map(|t| t.trim_start_matches(':').to_string())
                    .collect(),
                props: Default::default(),
                body,
                path: PathBuf::new(),
            };
            store.save(&mut task)?;
            println!("{huid}");
        }
        Command::Update { huid, add_tag, remove_tag } => {
            let h: Huid = huid.parse().map_err(RtatrError::InvalidHuid)?;
            let mut task = store.get(&h)?;
            for t in add_tag {
                task.tags.insert(t.trim_start_matches(':').to_string());
            }
            for t in remove_tag {
                task.tags.remove(t.trim_start_matches(':'));
            }
            store.save(&mut task)?;
        }
        Command::Done { huid } => {
            let h: Huid = huid.parse().map_err(RtatrError::InvalidHuid)?;
            let mut task = store.get(&h)?;
            task.tags.insert("done".to_string());
            store.save(&mut task)?;
        }
        Command::Rm { huid } => {
            let h: Huid = huid.parse().map_err(RtatrError::InvalidHuid)?;
            store.delete(&h)?;
        }
        Command::Show { huid } => {
            let h: Huid = huid.parse().map_err(RtatrError::InvalidHuid)?;
            let task = store.get(&h)?;
            println!("HUID: {}", task.huid);
            println!(
                "Tags: {}",
                task.tags.iter().map(|t| format!(":{t}")).collect::<Vec<_>>().join(" ")
            );
            println!();
            println!("{}", task.body);
        }
    }
    Ok(())
}
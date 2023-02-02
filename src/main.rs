use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Table};
use indicatif::ProgressBar;
use rand::{seq::IteratorRandom, thread_rng};
use std::{
    collections::BTreeSet,
    env,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    process::Command,
    thread,
    time::Duration,
};

#[derive(Parser, Debug)]
struct Arguments {
    #[command(subcommand)]
    command: Commands,

    /// Operate on a list with the given name
    #[arg(short, long)]
    list: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Adds an item to the list
    Add { name: String },

    /// Removes an item from the list
    Remove { name: String },

    /// Shows items in the list
    List,

    /// Rolls a random marble from the list, removing it
    Roll,

    /// Edits a list with $EDITOR
    Edit,
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();
    let list_name = arguments.list.unwrap_or_else(|| "default_list".to_string());

    let mut list = ItemList::new(&list_name)?;

    match arguments.command {
        Commands::Add { name } => {
            println!("Added {} to {}", name.underline(), list_name.bold().green());
            list.add(name);
        }
        Commands::Remove { name } => {
            println!(
                "Removing {} from {}",
                name.underline(),
                list_name.bold().green()
            );
            if !list.remove(&name) {
                println!(
                    "{} {} was not in list",
                    "error:".bold().red(),
                    name.underline()
                );
            }
        }
        Commands::List => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL).set_header(vec!["#", "Title"]);
            for (i, item) in list.items.iter().enumerate() {
                table.add_row(vec![(i + 1).to_string(), item.to_string()]);
            }
            println!("{table}");
        }
        Commands::Roll => {
            println!(
                "Rolling a marble for {} of {} choices",
                "1".bold(),
                list.items.len().to_string().bold()
            );

            // make it dramatic
            let bar = ProgressBar::new(list.items.len() as u64);

            for _ in 0..list.items.len() {
                thread::sleep(Duration::from_millis(200));
                bar.inc(1);
            }

            bar.finish_and_clear();

            let choice = list.take_random();
            let message = match choice {
                Some(choice) => format!("  rolled: {}", choice.bold().green().reversed()),
                None => format!(
                    "{} No marbles. You can add some with\n    {}",
                    "error:".bold().red(),
                    "marbles add <NAME>".bold()
                ),
            };

            println!("{message}");
        }
        Commands::Edit => {
            let Ok(mut child) = Command::new(env::var("EDITOR").unwrap_or_else(|_| "vim".to_string())).arg(&list.path).spawn() else {
                format!("{} Could not open EDITOR", "error:".red().bold());
                // this is wrong
                return Ok(());
            };

            child.wait()?;
            return Ok(());
        }
    }

    list.save()?;

    Ok(())
}

struct ItemList {
    path: PathBuf,
    items: BTreeSet<String>,
}

impl ItemList {
    fn new(list_name: &str) -> Result<ItemList> {
        // get movie list file
        let mut data_dir = dirs::data_local_dir().expect("Could not load data directory");
        data_dir.push("marbles");

        fs::create_dir_all(&data_dir)?;

        data_dir.push(list_name);

        let items = match File::open(&data_dir) {
            Ok(file) => BufReader::new(file).lines().flatten().collect(),
            Err(_) => BTreeSet::new(),
        };

        Ok(ItemList {
            path: data_dir,
            items,
        })
    }

    fn save(&self) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&self.path)?;
        let mut writer = BufWriter::new(file);

        for line in self.items.iter() {
            writeln!(writer, "{}", line)?;
        }

        writer.flush()?;

        Ok(())
    }

    fn take_random(&mut self) -> Option<String> {
        let mut rng = thread_rng();
        let item = self.items.iter().choose(&mut rng)?.to_string();
        self.items.remove(&item);
        Some(item)
    }

    fn add(&mut self, item: String) {
        self.items.insert(item);
    }

    fn remove(&mut self, item: &String) -> bool {
        self.items.remove(item)
    }
}

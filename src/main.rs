use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{presets::UTF8_FULL, CellAlignment, ColumnConstraint, Row, Table, Width::Fixed};
use crossterm::{
    cursor::{Hide, MoveDown, MoveUp, Show},
    execute,
    style::Stylize,
    terminal::{Clear, ClearType},
};
use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng, Rng,
};
use std::{
    collections::BTreeSet,
    env,
    fs::{self, File, OpenOptions},
    io::{stdout, BufRead, BufReader, BufWriter, Write},
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
            println!(
                "Added {} to {}",
                name.as_str().underlined(),
                list_name.bold().green()
            );
            list.add(name);
        }
        Commands::Remove { name } => {
            println!(
                "Removing {} from {}",
                name.as_str().underlined(),
                list_name.bold().green()
            );
            if !list.remove(&name) {
                println!(
                    "{} {} was not in list",
                    "error:".bold().red(),
                    name.underlined()
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

            let mut rng = thread_rng();

            let mut choices = list.items.iter().collect::<Vec<_>>();
            choices.shuffle(&mut rng);

            let count = rng.gen_range(300..500);

            let mut first = true;

            execute!(stdout(), Hide)?;

            for _ in 0..count {
                for i in 0..(choices.len() - 1) {
                    if rng.gen_bool(0.2) {
                        choices.swap(i, i + 1);
                    }
                }

                for i in 1..choices.len() {
                    if rng.gen_bool(0.2) {
                        choices.swap(i, i - 1);
                    }
                }

                let mut table = Table::new();
                table.load_preset(UTF8_FULL).set_header(vec!["#", "Title"]);

                let column = table.column_mut(1).unwrap();
                column.set_constraint(ColumnConstraint::Absolute(Fixed(36)));
                column.set_cell_alignment(CellAlignment::Left);

                for (i, item) in choices.iter().take(10).enumerate() {
                    let mut row = Row::from(vec![(i + 1).to_string(), item.to_string()]);
                    row.max_height(1);

                    table.add_row(row);
                }

                if !first {
                    execute!(stdout(), MoveUp(23))?;
                }

                for i in 0..choices.len() {
                    // kill a random element
                    // the `i < choices.len()` is not redundant
                    if rng.gen_bool(0.001) && i < choices.len() {
                        execute!(stdout(), Clear(ClearType::CurrentLine))?;
                        println!("dead: {}", choices.remove(i).as_str().dark_red().bold());
                    }
                }

                println!("{table}");

                thread::sleep(Duration::from_millis(200));

                first = false;
            }

            let choice = choices[0];
            println!("  rolled: {}", choice.as_str().bold().green().reverse());
            execute!(stdout(), Show)?;
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

    fn add(&mut self, item: String) {
        self.items.insert(item);
    }

    fn remove(&mut self, item: &String) -> bool {
        self.items.remove(item)
    }
}

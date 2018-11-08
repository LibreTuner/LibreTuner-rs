#![cfg(feature = "cli")]

use std::ffi::OsString;
use std::cell::RefCell;
use std::cmp::max;

use tuneutils::{
	link,
	definition::Definitions,
	download::DownloadCallback,
	diagnostics::UdsScanner,
};
#[cfg(feature = "socketcan")]
use tuneutils::link::SocketCanDataLinkEntry;

use crate::{
	app::App,
	error::{Error, Result},
};

use clap::value_t;



pub struct CommandContext<'a> {
	app: &'a mut App,
	commands: &'a Vec<Command>,
	args: &'a mut Iterator<Item=String>,
}


pub struct Command {
	pub description: String,
	pub command: String,
	pub callback: RefCell<Box<FnMut(CommandContext) -> Result<()>>>,
}


impl Command {
	/// Creates a new command.
	///
	/// # Arguments
	/// `description` - Description of the command shown in the 'help' command
	/// `command` - Keyword used to invoke the command
	/// `callback` - Function called when the command is invoked
	pub fn new<F: 'static>(command: String, description: String, callback: F) -> Command
	where F: FnMut(CommandContext) -> Result<()> {
		Command {
			description,
			command,
			callback: RefCell::new(Box::new(callback)),
		}
	}
}

/// Cli application
pub struct Cli<'a> {
	app: &'a mut App,
	commands: Vec<Command>,
}

mod commands {
	use super::*;



	pub fn help<'a, I>(commands: I)
	where I: Iterator<Item=&'a Command> {
		for command in commands {
			println!("{} - {}", command.command, command.description);
		}
	}



	pub fn links<'a>(links: &Vec<Box<link::DataLinkEntry>>) {
		// Get longest type and description
		let mut longest_type = "Type".len();
		// Find the amount of spaces the largest id will take. Minus one beacuse the highest id is len() - 1
		let id_len = (|| {
			if links.len() >= 100 {
				return ((links.len() - 1) as f64).log10() as usize + 1
			}
			"Id".len()
		})();

		for entry in links.iter() {
			longest_type = max(entry.typename().len(), longest_type);
		}
			
		println!("{1:<0$}   {3:<2$}   {4}", id_len, "Id", longest_type, "Type", "Description");
		// Print
		for (id, entry) in links.iter().enumerate() {
			println!("{1:<0$}   {3:<2$}   {4}", id_len, id.to_string(), longest_type, entry.typename(), entry.description());
		}
	}



	pub fn add_link(context: &mut CommandContext) -> Result<()> {
		let mut matches = clap::App::new("add_link")
			.about("Adds a datalink")
			.setting(clap::AppSettings::NoBinaryName)
			.setting(clap::AppSettings::SubcommandRequired);
		#[cfg(feature = "socketcan")]
		{
			matches = matches.subcommand(clap::SubCommand::with_name("socketcan")
				.about("Adds a socketcan interface")
				.arg(clap::Arg::with_name("interface")
					.help("SocketCAN interface name")
					.index(1)
					.required(true)));
		}
		let matches = matches.get_matches_from_safe(context.args.into_iter())?;

		#[cfg(feature = "socketcan")]
		{
			if let Some(matches) = matches.subcommand_matches("socketcan") {
				let interface = matches.value_of("interface").unwrap();
				context.app.avail_links.push(Box::new(link::SocketCanDataLinkEntry { interface: interface.to_owned(), }));
				println!("Added SocketCAN interface \"{}\"", interface);
				return Ok(());
			}
		}
		// Should never reach this...
		Ok(())
	}



	pub fn platforms(definitions: &Definitions) {
		// Find the longest definition id & name
		let mut longest_id = "Id".len();

		for definition in definitions.definitions.iter() {
			longest_id = max(definition.id.len(), longest_id);
		}

		// Print the second loop
		println!("{1:<0$}   {2}", longest_id, "Id", "Name");
		for definition in definitions.definitions.iter() {
			println!("{1:<0$}   {2}", longest_id, definition.id, definition.name);
		}
	}



	pub fn download(context: &mut CommandContext) -> Result<()> {
		let matches = clap::App::new("download")
			.about("Downloads a rom from a platform link")
			.setting(clap::AppSettings::NoBinaryName)
			.arg(clap::Arg::with_name("datalink")
				.help("ID of the datalink to use. Can be found using the 'links' command")
				.index(1)
				.required(true))
			.arg(clap::Arg::with_name("platform")
				.help("ID of the platform to download from. Can be found using the 'platforms' command")
				.index(2)
				.required(true))
			.arg(clap::Arg::with_name("id")
				.help("Identifier given to the ROM when saving")
				.index(3)
				.required(true))
			.arg(clap::Arg::with_name("name")
				.help("Name given to the ROM when saving. Defaults to the id")
				.index(4))
			.get_matches_from_safe(context.args.into_iter())?;

		let datalink = context.app.get_datalink(value_t!(matches, "datalink", usize)?)?;

		let platform = context.app.definitions.find(matches.value_of("platform").unwrap()).ok_or(Error::InvalidPlatform)?;
		let downloader = link::PlatformLink::new(datalink, platform.clone()).downloader().ok_or(Error::DownloadUnsupported)?;

		let id = matches.value_of("id").unwrap();
		let name = matches.value_of("name").unwrap_or(id);

		// Begin downloading
		let response = downloader.download(&DownloadCallback::with(|progress| {
			println!("Progress: {:.2}%", progress * 100.0);
		}))?;

		let model = platform.identify(&response.data).ok_or(Error::UnknownModel)?;
		let rom = context.app.roms.new_rom(name.to_owned(), id.to_owned(), platform.clone(), model.clone(), response.data);
        context.app.roms.save_meta().unwrap();
        rom.save().unwrap();

        println!("Saved ROM as \"{}\"", id);

		Ok(())
	}



	pub fn pids(context: &mut CommandContext) -> Result<()> {
		let matches = clap::App::new("download")
			.about("Downloads a rom from a platform link")
			.setting(clap::AppSettings::NoBinaryName)
			.arg(clap::Arg::with_name("platform")
				.help("ID of the platform. Can be found using the 'links' command")
				.index(1)
				.required(true))
			.get_matches_from_safe(context.args.into_iter())?;

		let platform = context.app.definitions.find(matches.value_of("platform").unwrap()).ok_or(Error::InvalidPlatform)?;

		// Find longest id and name
		let mut longest_name = "Name".len();
		let mut longest_id = "Id".len();

		for pid in platform.pids.iter() {
			if pid.id >= 100 {
				longest_id = max((pid.id as f64).log10() as usize + 1, longest_id);
			}
			longest_name = max(pid.name.len(), longest_name);
		}

		println!("{1:<0$}   {3:<2$}   {4}", longest_id, "Id", longest_name, "Name", "Description");
        for pid in platform.pids.iter() {
            println!("{1:<0$}   {3:<2$}   {4}", longest_id, pid.id.to_string(), longest_name, pid.name, pid.description);
        }

		Ok(())
	}



	pub fn roms(context: &mut CommandContext) -> Result<()> {
		let mut longest_id = "Id".len();
		let mut longest_name = "Name".len();
		let mut longest_platform = "Platform".len();

		for rom in context.app.roms.roms.iter() {
			longest_id = max(rom.id.len(), longest_id);
			longest_name = max(rom.name.len(), longest_name);
			longest_platform = max(rom.platform.name.len(), longest_platform);
		}

		println!("{1:<0$}   {3:<2$}   {5:<4$}   {6}", longest_id, "Id", longest_name, "Name", longest_platform, "Platform", "Model");
        for rom in context.app.roms.roms.iter() {
            println!("{1:<0$}   {3:<2$}   {5:<4$}   {6}", longest_id, rom.id, longest_name, rom.name, longest_platform, rom.platform.name, rom.model.name);
        }

        Ok(())
	}



	pub fn tunes(context: &mut CommandContext) -> Result<()> {
		let mut longest_id = "Id".len();
		let mut longest_name = "Name".len();

		for tune in context.app.tunes.tunes.iter() {
			longest_id = max(tune.id.len(), longest_id);
			longest_name = max(tune.name.len(), longest_name);
		}

		println!("{1:<0$}   {3:<2$}   {4}", longest_id, "Id", longest_name, "Name", "ROM Id");
        for tune in context.app.tunes.tunes.iter() {
            println!("{1:<0$}   {3:<2$}   {4}", longest_id, tune.id, longest_name, tune.name, tune.rom_id);
        }

        Ok(())
	}



	pub fn create_tune(context: &mut CommandContext) -> Result<()> {
		let matches = clap::App::new("create_tune")
			.about("Creates a new tune from a ROM")
			.setting(clap::AppSettings::NoBinaryName)
			.arg(clap::Arg::with_name("rom")
				.help("ID of the rom the tune will inherit. See 'roms' for a list")
				.index(1)
				.required(true))
			.arg(clap::Arg::with_name("id")
				.help("Identifier given to the tune when saving")
				.index(2)
				.required(true))
			.arg(clap::Arg::with_name("name")
				.help("Name given to the tune when saving. Defaults to the id")
				.index(3))
			.get_matches_from_safe(context.args.into_iter())?;

		// Check that the ROM exists. We do not need it to create a tune.
		let rom_id = matches.value_of("rom").unwrap();
		context.app.roms.search(rom_id).ok_or(Error::InvalidRom)?;

		let id = matches.value_of("id").unwrap();
		// Name defaults to id
		let name = matches.value_of("name").unwrap_or(id);

		context.app.tunes.add_meta(name.to_owned(), id.to_owned(), rom_id.to_owned());
        context.app.tunes.save()?;
        Ok(())
	}



	pub fn scan(context: &mut CommandContext) -> Result<()> {
		let matches = clap::App::new("scan")
			.about("Scans OBD-II trouble codes")
			.setting(clap::AppSettings::NoBinaryName)
			.arg(clap::Arg::with_name("datalink")
				.help("ID of the datalink to use. Can be found using the 'links' command")
				.index(1)
				.required(true))
			.arg(clap::Arg::with_name("platform")
				.help("ID of the platform to download from. Can be found using the 'platforms' command")
				.index(2)
				.required(true))
			.get_matches_from_safe(context.args.into_iter())?;

		let datalink = context.app.get_datalink(value_t!(matches, "datalink", usize)?)?;

		let platform = context.app.definitions.find(matches.value_of("platform").unwrap()).ok_or(Error::InvalidPlatform)?;
		let interface = link::PlatformLink::new(datalink, platform.clone()).uds().ok_or(Error::InvalidDatalink)?;

		let scanner = UdsScanner::new(interface);
		let codes = scanner.scan()?;
		for code in codes {
			println!("{}", code);
		}

		Ok(())
	}
}


impl<'a> Cli<'a> {
	/// Creates a Cli application that controls a LibreTuner app.
	pub fn new(app: &mut App) -> Cli {
		Cli {
			app,
			commands: Vec::new(),
		}
	}

	/// Registers default commands
	pub fn register_all(&mut self) {
		// Internal help command
		self.commands.push(Command::new("help".to_owned(), "This command".to_owned(),
			|context| {
				commands::help(context.commands.iter());
				Ok(())
			},
		));


		// Lists available datalinks
		self.commands.push(Command::new("links".to_owned(), "Lists available datalinks".to_owned(), 
			|context| {
				commands::links(&context.app.avail_links);
				Ok(())
			},
		));

		// Adds a datalink
		self.commands.push(Command::new("add_link".to_owned(), "Adds a datalink".to_owned(), 
			|mut context| {
				commands::add_link(&mut context)
			},
		));

		// Lists installed platform definitions
		self.commands.push(Command::new("platforms".to_owned(), "Lists all installed platform definitions".to_owned(), 
			|context| {
				commands::platforms(&context.app.definitions);
				Ok(())
			}
		));

		// Downloads a ROM
		self.commands.push(Command::new("download".to_owned(), "Downloads a ROM from a platform link".to_owned(),
			|mut context| {
				commands::download(&mut context)
			}
		));

		// Lists PIDs for a platform
		self.commands.push(Command::new("pids".to_owned(), "Lists PIDs for a platform".to_owned(),
			|mut context| {
				commands::pids(&mut context)
			}
		));

		// Lists ROMs
		self.commands.push(Command::new("roms".to_owned(), "Lists downloaded ROMs".to_owned(),
			|mut context| {
				commands::roms(&mut context)
			}
		));

		self.commands.push(Command::new("tunes".to_owned(), "Lists tunes".to_owned(),
			|mut context| {
				commands::tunes(&mut context)
			}
		));

		self.commands.push(Command::new("create_tune".to_owned(), "Creates a new tune".to_owned(),
			|mut context| {
				commands::create_tune(&mut context)
			}
		));

		self.commands.push(Command::new("scan".to_owned(), "Scans OBD-II trouble codes".to_owned(),
			|mut context| {
				commands::scan(&mut context)
			}
		));
	}

	pub fn register_command(&mut self, command: Command) {
		self.commands.push(command);
	}

	/// Processes an iterator as a command.
	pub fn process<I>(&mut self, itr: I) -> Result<()>
	where I: Iterator<Item=String>,
	{
		let mut it = itr.into_iter();
		let cmd = it.next().ok_or(Error::InvalidCommand)?;

		let command = self.commands.iter().find(|ref x| x.command == cmd).ok_or(Error::InvalidCommand)?;

		// Command exists
		let closure = &mut *command.callback.borrow_mut();
		let result = (closure)(CommandContext {
			app: self.app,
			commands: &self.commands,
			args: &mut it,
		});
		if let Err(err) = result {
			match err {
				Error::Clap(err) => println!("{}", err),
				_ => println!("Error: {}", err),
			}
		}

		Ok(())
	}
}
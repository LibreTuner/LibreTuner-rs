#![cfg(feature = "cli")]

use std::ffi::OsString;
use std::cell::RefCell;

use crate::{
	app::App,
	error::{Error, Result},
};

pub struct CommandContext<'a> {
	app: &'a mut App,
	commands: &'a Vec<Command>,
}


pub struct Command {
	pub description: String,
	pub command: String,
	pub callback: RefCell<Box<FnMut(CommandContext)>>,
}

impl Command {
	/// Creates a new command.
	///
	/// # Arguments
	/// `description` - Description of the command shown in the 'help' command
	/// `command` - Keyword used to invoke the command
	/// `callback` - Function called when the command is invoked
	pub fn new<F: 'static>(command: String, description: String, callback: F) -> Command
	where F: FnMut(CommandContext) {
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


fn command_help<'a, I>(commands: I)
where I: Iterator<Item=&'a Command>
{
	for command in commands {
		println!("{} - {}", command.command, command.description);
	}
}


impl<'a> Cli<'a> {
	/// Creates a Cli application that controls a LibreTuner app.
	pub fn new(app: &mut App) -> Cli {
		let mut commands = Vec::new();

		Cli {
			app,
			commands,
		}
	}

	pub fn register_all(&mut self) {
		// Internal help command
		self.commands.push(Command::new("help".to_owned(), "This command".to_owned(),
			|context| {
				command_help(context.commands.iter());
			},
		));
	}

	pub fn register_command(&mut self, command: Command) {
		self.commands.push(command);
	}

	pub fn process<I, T>(&mut self, itr: I) -> Result<()>
	where I: Iterator<Item=T>,
			T: Into<String>,
	{
		let mut it = itr.into_iter();
		let cmd = it.next().ok_or(Error::InvalidCommand)?;
		let cmd = cmd.into();

		let command = self.commands.iter().find(|ref x| x.command == cmd).ok_or(Error::InvalidCommand)?;

		// Command exists
		let closure = &mut *command.callback.borrow_mut();
		(closure)(CommandContext {
			app: self.app,
			commands: &self.commands,
		});

		Ok(())
	}
}
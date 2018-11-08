use tuneutils;
use std::{result, io, fmt};

#[derive(Debug)]
pub enum Error {
	TuneUtils(tuneutils::error::Error),
	Io(io::Error),
	NoHome,
	#[cfg(feature = "cli")]
	InvalidCommand,
	#[cfg(feature = "cli")]
	Clap(clap::Error),
	InvalidPlatform,
	UnknownModel,
	InvalidDatalink,
	DownloadUnsupported,
	InvalidRom,
}

pub type Result<T> = result::Result<T, Error>;

impl From<tuneutils::error::Error> for Error {
	fn from(err: tuneutils::error::Error) -> Error {
		Error::TuneUtils(err)
	}
}

impl From<io::Error> for Error {
	fn from(err: io::Error) -> Error {
		Error::Io(err)
	}
}

#[cfg(feature = "cli")]
impl From<clap::Error> for Error {
	fn from(err: clap::Error) -> Error {
		Error::Clap(err)
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::TuneUtils(ref err) => write!(f, "TuneUtils error: {}", err),
			Error::NoHome => write!(f, "No valid home directory path could be retrieved from the operating system"),
			Error::Io(ref err) => write!(f, "IO error: {}", err),
			#[cfg(feature = "cli")]
			Error::InvalidCommand => write!(f, "Invalid command"),
			#[cfg(feature = "cli")]
			Error::Clap(ref err) => write!(f, "{}", err),
			Error::InvalidPlatform => write!(f, "Invalid platform"),
			Error::InvalidDatalink => write!(f, "Invalid datalink"),
			Error::DownloadUnsupported => write!(f, "Downloading unsupported for a datalink or platform"),
			Error::UnknownModel => write!(f, "Unknown model"),
			Error::InvalidRom => write!(f, "Invalid ROM"),
		}
	}
}
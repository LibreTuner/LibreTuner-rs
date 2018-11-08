use std::{
	fs,
	path::PathBuf,
	cell::RefCell,
};

use crate::error::{Error, Result};
use directories::ProjectDirs;

use tuneutils::{
	protocols::{can, isotp, uds::{UdsInterface, UdsIsotp}},
	definition::Definitions,
	link, rom,
};

pub struct App {
	pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub avail_links: Vec<Box<link::DataLinkEntry>>,
    pub definitions: Definitions,
    pub roms: rom::RomManager,
    pub tunes: rom::tune::TuneManager,
}

impl App {
	/// Attempts to load LibreTuner. This will create any necessary data or
	/// configuration directories and loads ROM and tune data.
	pub fn new() -> Result<App> {
		// Create config and data directories if they do not exist
		let proj_dirs = ProjectDirs::from("org", "LibreTuner",  "TuneUtils").ok_or(Error::NoHome)?;
        let config_dir = proj_dirs.config_dir().to_path_buf();
        let data_dir = proj_dirs.data_dir().to_path_buf();
        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&data_dir)?;

        // Load definitions
        let mut definitions = Definitions::default();
        definitions.load(&config_dir.join("definitions"))?;

        // Load ROMs and tunes
        let rom_dir = data_dir.join("roms");
        fs::create_dir_all(&rom_dir)?;
        let mut roms = rom::RomManager::new(rom_dir);
        roms.load(&definitions)?;

        let tune_dir = data_dir.join("tunes");
        fs::create_dir_all(&tune_dir)?;
        let tunes = rom::tune::TuneManager::load(tune_dir)?;

        Ok(App {
            config_dir,
            data_dir,
            avail_links: link::discover_datalinks(),
            definitions,
            roms,
            tunes,
        })
	}

	/// Loads a datalink by id or returns Error::InvalidDatalink
	pub fn get_datalink(&self, id: usize) -> Result<Box<link::DataLink>> {
		if id >= self.avail_links.len() {
			return Err(Error::InvalidDatalink);
		}
		Ok(self.avail_links[id].create()?)
	}
}
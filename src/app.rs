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
	link::{self, PlatformLink}, rom,
    download::DownloadCallback,
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
        println!("Getting link {}", id);
		if id >= self.avail_links.len() {
			return Err(Error::InvalidDatalink);
		}
        println!("Getting and creating....");
		Ok(self.avail_links[id].create()?)
	}

    /// Creates a platform link from a datalink ID and platform ID
    pub fn create_platform_link(&self, datalink: usize, platform: &str) -> Result<PlatformLink> {
        let datalink = self.get_datalink(datalink);
        let datalink = datalink?;
        let platform = self.definitions.find(platform).ok_or(Error::InvalidPlatform)?;
        Ok(PlatformLink::new(datalink, platform.clone()))
    }

    /// Returns a list of all platform definitions in the format (name, id)
    pub fn list_platforms(&self) -> Vec<(&str, &str)> {
        let mut platforms = Vec::new();

        for platform in &self.definitions.definitions {
            platforms.push((platform.name.as_str(), platform.id.as_str()));
        }

        platforms
    }

    pub fn download(&mut self, link: &PlatformLink, id: &str, name: &str, callback: &DownloadCallback) -> Result<()> {
        let downloader = link.downloader().ok_or(Error::DownloadUnsupported)?;
        let response = downloader.download(callback)?;

        let model = link.platform.identify(&response.data).ok_or(Error::UnknownModel)?;
        let rom = self.roms.new_rom(name.to_owned(), id.to_owned(), link.platform.clone(), model.clone(), response.data);
        self.roms.save_meta().unwrap();
        rom.save().unwrap();

        Ok(())
    }
}
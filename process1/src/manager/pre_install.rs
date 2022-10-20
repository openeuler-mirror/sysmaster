#![allow(non_snake_case)]
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File},
    io::{self, BufRead, Error, ErrorKind},
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use bitflags::bitflags;
use nix::unistd::UnlinkatFlags;

use crate::manager::{DeserializeWith, UnitType};
use confique::Config;
use serde::{Deserialize, Serialize};
use utils::path_lookup::LookupPaths;
use walkdir::{DirEntry, WalkDir};

use utils::fs_util;

use super::unit::{unit_name_to_type, UeConfigInstall};

#[derive(PartialEq, Eq)]
pub(crate) enum PresetMode {
    // All,
    Enable,
    Disable,
}

bitflags! {
    struct SearchFlags: u8 {
        const LOAD = 1 << 0;
        const FOLLOW_SYMLINKS = 1 << 1;
        const DROPIN = 1 << 2;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PresetAction {
    Enable,
    Disable,
    Unknown,
}

impl FromStr for PresetAction {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ret = match s {
            "enable" => PresetAction::Enable,
            "disable" => PresetAction::Disable,
            _ => PresetAction::Unknown,
        };
        Ok(ret)
    }
}

struct PresetRule {
    action: PresetAction,
    unit: String,
}

impl PresetRule {
    fn new(action: PresetAction, unit: String) -> Self {
        PresetRule { action, unit }
    }
}

pub(crate) struct Presets {
    rules: Vec<PresetRule>,
}

impl Presets {
    fn new() -> Self {
        Presets { rules: Vec::new() }
    }

    fn unit_preset_action(&self, unit: String) -> PresetAction {
        for rule in self.rules.iter() {
            let re = fnmatch_regex::glob_to_regex(&rule.unit);
            if let Err(_e) = re {
                continue;
            }
            if re.unwrap().is_match(&unit) {
                return rule.action;
            }
        }

        PresetAction::Unknown
    }

    fn add_rule(&mut self, rule: PresetRule) {
        self.rules.push(rule)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum UnitFileType {
    Regular,
    Symlink,
    Masked,
    Invalid,
}

#[derive(Debug)]
struct UnitInstall {
    name: String,
    path: RefCell<PathBuf>,
    aliases: RefCell<Vec<String>>,
    wanted_by: RefCell<Vec<String>>,
    required_by: RefCell<Vec<String>>,
    also: RefCell<Vec<String>>,
    u_type: RefCell<UnitFileType>,
}

impl UnitInstall {
    fn new(unit: &str) -> Self {
        UnitInstall {
            name: unit.to_string(),
            path: RefCell::new(PathBuf::new()),
            aliases: RefCell::new(Vec::new()),
            wanted_by: RefCell::new(Vec::new()),
            required_by: RefCell::new(Vec::new()),
            also: RefCell::new(Vec::new()),
            u_type: RefCell::new(UnitFileType::Invalid),
        }
    }

    fn u_type(&self) -> UnitFileType {
        *self.u_type.borrow()
    }

    fn set_u_type(&self, t: UnitFileType) {
        *self.u_type.borrow_mut() = t;
    }

    fn name(&self) -> String {
        self.name.to_string()
    }

    fn path(&self) -> String {
        self.path.borrow().to_str().unwrap().to_string()
    }

    fn set_path(&self, path: String) {
        *self.path.borrow_mut() = PathBuf::from(path);
    }

    fn fill_struct(&self, config: &UeConfigData) {
        for v in &config.Install.Alias {
            self.aliases.borrow_mut().push(v.to_string());
        }

        for v in &config.Install.WantedBy {
            self.wanted_by.borrow_mut().push(v.to_string());
        }

        for v in &config.Install.RequiredBy {
            self.required_by.borrow_mut().push(v.to_string());
        }

        for v in &config.Install.Also {
            self.also.borrow_mut().push(v.to_string());
        }
    }

    fn wanted_by(&self) -> Vec<String> {
        self.wanted_by.borrow().to_vec()
    }

    fn required_by(&self) -> Vec<String> {
        self.required_by.borrow().to_vec()
    }

    fn alias(&self) -> Vec<String> {
        self.aliases.borrow().to_vec()
    }
}

struct InstallContext {
    processed: RefCell<HashMap<String, Rc<UnitInstall>>>,
    will_process: RefCell<BTreeMap<String, Rc<UnitInstall>>>,
}

impl InstallContext {
    fn new() -> Self {
        InstallContext {
            processed: RefCell::new(HashMap::new()),
            will_process: RefCell::new(BTreeMap::new()),
        }
    }

    fn unit_install(&self, name: String) -> Option<Rc<UnitInstall>> {
        if let Some(v) = self.processed.borrow().get(&name) {
            return Some(v.clone());
        }

        self.will_process.borrow().get(&name).cloned()
    }

    fn installed_unit(&self, unit: &str) -> bool {
        self.processed.borrow().contains_key(unit) || self.will_process.borrow().contains_key(unit)
    }

    fn add_unit_install(&self, unit: &str, unit_install: Rc<UnitInstall>) {
        self.will_process
            .borrow_mut()
            .insert(unit.to_string(), unit_install);
    }

    fn apply_enable_unit_install(&self, target_path: &str) {
        if self.will_process.borrow().is_empty() {
            return;
        }

        let keys: Vec<_> = self.will_process.borrow().keys().cloned().collect();

        let mut will_process = self.will_process.borrow_mut();
        for key in keys.iter() {
            let unit_install = will_process.get(key);

            if unit_install.is_none() {
                continue;
            }

            let i = unit_install.unwrap();
            if i.u_type() != UnitFileType::Regular {
                println!(
                    "apply unit install is symlink, skip it*****************: {:?}",
                    i.name()
                );
                continue;
            }

            self.install_symlinks(i.clone(), target_path);

            self.processed
                .borrow_mut()
                .insert(key.to_string(), i.clone());

            will_process.remove(key);
        }
    }

    fn collect_disable_install(&self, removal_symlinks: &mut HashSet<String>) {
        let keys: Vec<_> = self.will_process.borrow().keys().cloned().collect();

        let mut will_process = self.will_process.borrow_mut();
        for key in keys.iter() {
            let unit_install = will_process.get(key);

            if unit_install.is_none() {
                continue;
            }

            let i = unit_install.unwrap();
            removal_symlinks.insert(i.name());

            self.processed
                .borrow_mut()
                .insert(key.to_string(), i.clone());

            will_process.remove(key);
        }
    }

    fn install_symlinks(&self, install: Rc<UnitInstall>, target_path: &str) {
        if let Err(e) = self.install_symlinks_alias(install.clone(), target_path, install.alias()) {
            log::warn!("create unit {} alias error: {}", install.name(), e);
        }

        if let Err(e) = self.install_symlinks_wants(
            install.clone(),
            target_path,
            "wants".to_string(),
            install.wanted_by(),
        ) {
            log::warn!("create unit {} wants error: {}", install.name(), e);
        }

        if let Err(e) = self.install_symlinks_wants(
            install.clone(),
            target_path,
            "requires".to_string(),
            install.required_by(),
        ) {
            log::warn!("create unit {} requires error: {}", install.name(), e);
        }
    }

    fn install_symlinks_alias(
        &self,
        install: Rc<UnitInstall>,
        target_path: &str,
        symlinks: Vec<String>,
    ) -> Result<i32, Error> {
        let mut n = 0;
        if symlinks.is_empty() {
            return Ok(0);
        }

        let source = install.path();
        for symlink in symlinks {
            let target = format!("{}/{}", target_path, symlink);
            if let Err(e) = fs_util::symlink(&source, &target, false) {
                log::warn!(
                    "create symlink from {} to {}, errno is: {}",
                    &source,
                    &target,
                    e
                );
                continue;
            }
            n += 1;
        }

        Ok(n)
    }

    fn install_symlinks_wants(
        &self,
        install: Rc<UnitInstall>,
        target_path: &str,
        suffix: String,
        symlinks: Vec<String>,
    ) -> Result<i32, Error> {
        if symlinks.is_empty() {
            return Ok(0);
        }

        let mut n = 0;
        let source = install.path();

        for symlink in symlinks {
            let target = format!("{}/{}.{}/{}", target_path, symlink, suffix, install.name());

            let path = Path::new(&target);
            let parent_path = path.parent();
            if let Err(e) = fs::create_dir_all(parent_path.unwrap()) {
                if e.kind() != io::ErrorKind::AlreadyExists {
                    return Err(e);
                }
            }

            if let Err(e) = fs_util::symlink(&source, &target, false) {
                log::warn!(
                    "create symlink from {} to {}, errno is: {}",
                    &source,
                    &target,
                    e
                );
                continue;
            }
            n += 1;
        }

        Ok(n)
    }
}

#[derive(Config, Default, Debug)]
pub(crate) struct UeConfigData {
    #[config(nested)]
    pub Install: UeConfigInstall,
}

pub(crate) struct Install {
    mode: PresetMode,
    enable_ctx: Rc<InstallContext>,
    disable_ctx: Rc<InstallContext>,

    lookup_path: Rc<LookupPaths>,
}

impl Install {
    pub fn new(p_mode: PresetMode, lookup_path: Rc<LookupPaths>) -> Self {
        Install {
            mode: p_mode,
            enable_ctx: Rc::new(InstallContext::new()),
            disable_ctx: Rc::new(InstallContext::new()),

            lookup_path,
        }
    }

    /// preset all files depend on .preset files
    pub fn preset_all(&self) -> Result<(), Error> {
        let target_path = &self.lookup_path.persistent_path;

        let presets = self.read_presets();

        for v in &self.lookup_path.search_path {
            let dir = Path::new(v);
            if !dir.is_dir() {
                continue;
            }

            let read_dir = dir.read_dir()?;
            for entry in read_dir {
                if let Err(e) = entry {
                    log::warn!("iter read dir error {}", e);
                    continue;
                }
                let u_path = entry.unwrap().path();
                if !u_path.is_file() && !u_path.is_symlink() {
                    continue;
                }

                let file_name = String::from(u_path.file_name().unwrap().to_str().unwrap());
                let unit_type = unit_name_to_type(&file_name);

                if unit_type == UnitType::UnitTypeInvalid {
                    continue;
                }

                self.preset_one_file(&file_name, &presets)?;
            }
        }

        self.execute_preset(target_path);
        Ok(())
    }

    fn preset_one_file(&self, unit: &str, presets: &Presets) -> Result<(), Error> {
        log::debug!("preset one unit file {}", unit);
        if self.installed_unit(unit) {
            return Ok(());
        }

        let action = presets.unit_preset_action(unit.to_string());
        println!("unit: {}, action is: {:?}", unit, action);
        match action {
            PresetAction::Enable | PresetAction::Unknown => {
                self.unit_install_discover(unit, self.enable_ctx.clone())?;
            }
            PresetAction::Disable => {
                self.unit_install_discover(unit, self.disable_ctx.clone())?;
            }
        }

        Ok(())
    }

    fn unit_install_discover(&self, unit: &str, ctx: Rc<InstallContext>) -> Result<(), Error> {
        let unit_install = self.prepare_unit_install(unit, ctx.clone());

        self.unit_file_search(unit_install, ctx)?;

        Ok(())
    }

    /// if the UnitInstall is already exist in the InstallContext, return the exist UnitInstall
    /// others, create a new UnitInstall and add to InstallContext
    fn prepare_unit_install(&self, unit: &str, ctx: Rc<InstallContext>) -> Rc<UnitInstall> {
        let unit_install = ctx.unit_install(unit.to_string());

        if let Some(install) = unit_install {
            return install;
        }

        let unit_install = Rc::new(UnitInstall::new(unit));

        ctx.add_unit_install(unit, unit_install.clone());

        unit_install
    }

    fn unit_file_search(
        &self,
        unit_install: Rc<UnitInstall>,
        ctx: Rc<InstallContext>,
    ) -> Result<(), Error> {
        if unit_install.u_type() != UnitFileType::Invalid {
            return Ok(());
        }

        if !unit_install.path().is_empty() {
            self.unit_file_load(&unit_install.path(), unit_install.clone(), ctx.clone())?;
        }

        for v in &self.lookup_path.search_path {
            let unit = Path::new(v).join(unit_install.clone().name());
            if !unit.exists() {
                continue;
            }

            self.unit_file_load(unit.to_str().unwrap(), unit_install.clone(), ctx.clone())?;
            unit_install.set_path(unit.to_str().unwrap().to_string());
        }

        Ok(())
    }

    fn unit_file_load(
        &self,
        path: &str,
        unit_install: Rc<UnitInstall>,
        ctx: Rc<InstallContext>,
    ) -> Result<(), Error> {
        let path = Path::new(&path);

        let meta = path.metadata()?;
        if meta.is_file() && meta.len() == 0 {
            unit_install.set_u_type(UnitFileType::Masked);
        } else if meta.is_file() {
            unit_install.set_u_type(UnitFileType::Regular);
        } else if meta.is_symlink() {
            unit_install.set_u_type(UnitFileType::Symlink);
        }

        let canon_path = path.canonicalize()?;
        let tmp = format!("{}.toml", canon_path.as_path().display());
        if let Err(e) = std::fs::copy(canon_path, &tmp) {
            log::warn!("copy file content to toml file error: {}", e);
            return Err(e);
        }

        let mut builder = UeConfigData::builder().env();
        builder = builder.file(&tmp);

        let dropin_dir_name = format!("{}.d", unit_install.name());

        for v in &self.lookup_path.search_path {
            let base_dir = Path::new(v);
            let dropin_dir = base_dir.join(&dropin_dir_name);

            if !dropin_dir.exists() {
                log::debug!("dropin path is not exist, ignore it: {:?}", &dropin_dir);
                continue;
            }

            let dirs = dropin_dir.read_dir()?;
            for entry in dirs {
                let dir_entry = entry?;
                let fragment = dir_entry.path();
                if fragment.is_file() {
                    let file_name = String::from(fragment.file_name().unwrap().to_str().unwrap());
                    if file_name.starts_with('.') || !file_name.ends_with(".toml") {
                        continue;
                    }

                    builder = builder.file(fragment);
                }
            }
        }

        let configer = builder
            .load()
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        unit_install.fill_struct(&configer);

        for also in configer.Install.Also {
            self.unit_install_discover(&also, ctx.clone())?;
        }
        // fs::remove_file(&tmp);

        Ok(())
    }

    fn execute_preset(&self, target_path: &str) {
        if self.mode != PresetMode::Enable {
            let mut removal_symlinks = HashSet::new();
            self.disable_ctx
                .collect_disable_install(&mut removal_symlinks);

            self.remove_symlinks(&mut removal_symlinks, target_path);
        }

        if self.mode != PresetMode::Disable {
            self.install_symlinks(target_path);
        }
    }

    fn remove_symlinks(&self, removal_symlinks: &mut HashSet<String>, target_path: &str) {
        for entry in WalkDir::new(target_path).min_depth(1).into_iter() {
            if let Err(_e) = entry {
                continue;
            }
            let entry = entry.unwrap();

            let file_type = entry.file_type();

            if !file_type.is_symlink() {
                continue;
            }

            let file_name = entry.file_name();

            if !removal_symlinks.contains(file_name.to_str().unwrap()) {
                continue;
            }

            if let Err(e) = nix::unistd::unlinkat(None, entry.path(), UnlinkatFlags::RemoveDir) {
                log::warn!("unlink path: {:?}, error: {}", entry.path(), e);
            }
        }
    }

    fn install_symlinks(&self, target_path: &str) {
        self.enable_ctx.apply_enable_unit_install(target_path);
    }

    fn installed_unit(&self, unit: &str) -> bool {
        self.enable_ctx.installed_unit(unit) || self.disable_ctx.installed_unit(unit)
    }

    /// read from .preset files to rules
    pub fn read_presets(&self) -> Presets {
        let mut presets = Presets::new();
        let preset_files = self.preset_config_files();

        for file in preset_files.iter() {
            let f = match File::open(file) {
                Err(why) => {
                    log::warn!("Error: Open file failed detail {} {:?}!", why, file);
                    continue;
                }
                Ok(file) => file,
            };

            for line in io::BufReader::new(f).lines().flatten() {
                if line.trim().starts_with('#') {
                    continue;
                }

                let contents: Vec<String> =
                    line.split_whitespace().map(|s| s.to_string()).collect();

                if contents.len() != 2 {
                    continue;
                }

                presets.add_rule(PresetRule::new(
                    PresetAction::from_str(&contents[0]).unwrap(),
                    contents[1].to_string(),
                ));
            }
        }

        presets
    }

    fn preset_config_files(&self) -> Vec<PathBuf> {
        let search_dirs = vec![
            "/etc/process1/system-preset",
            "/usr/lib/process1/system-preset",
            "/lib/process1/system-preset",
        ];

        let mut files_hash = HashMap::new();
        for dir in search_dirs.iter() {
            self.add_preset_file(dir, &mut files_hash)
        }

        let mut preset_files = vec![];
        for (_, path) in files_hash.iter() {
            preset_files.push(path.clone());
        }

        // priority by file names
        preset_files.sort_by(|a, b| a.partial_cmp(b).unwrap());
        preset_files
    }

    fn valid_preset_file(entry: &DirEntry) -> bool {
        let file_type = entry.file_type();
        if !file_type.is_file() {
            return false;
        }

        let file_name = entry.file_name();
        if file_name
            .to_str()
            .map(|s| s.ends_with(".preset"))
            .unwrap_or(false)
        {
            return true;
        }

        false
    }

    fn add_preset_file(&self, dir: &str, files: &mut HashMap<String, PathBuf>) {
        for entry in WalkDir::new(dir)
            .min_depth(1)
            .into_iter()
            .filter_entry(Self::valid_preset_file)
        {
            if let Err(_e) = entry {
                continue;
            }
            let entry = entry.unwrap();
            let file_name = entry.file_name();
            log::debug!("filename is : {}", file_name.to_str().unwrap());
            if files.contains_key(file_name.to_str().unwrap()) {
                continue;
            }

            files.insert(
                file_name.to_str().unwrap().to_string(),
                Path::new(dir).join(file_name),
            );
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Install, PresetAction, PresetMode, PresetRule, Presets};
    use std::rc::Rc;
    use utils::path_lookup::LookupPaths;

    #[test]
    fn test_presets() {
        let mut presets = Presets::new();

        presets.add_rule(PresetRule::new(
            PresetAction::Enable,
            "test.service".to_string(),
        ));
        presets.add_rule(PresetRule::new(
            PresetAction::Enable,
            "rsyslog.*".to_string(),
        ));
        presets.add_rule(PresetRule::new(PresetAction::Disable, "*".to_string()));

        assert_eq!(
            presets.unit_preset_action("test.service".to_string()),
            PresetAction::Enable
        );
        assert_eq!(
            presets.unit_preset_action("rsyslog.service".to_string()),
            PresetAction::Enable
        );
        assert_eq!(
            presets.unit_preset_action("rsyslog.socket".to_string()),
            PresetAction::Enable
        );
        assert_eq!(
            presets.unit_preset_action("tmp2.service".to_string()),
            PresetAction::Disable
        );
    }

    #[test]
    fn test_read_presets() {
        let mut l_path = LookupPaths::new();
        l_path.init_lookup_paths();
        let lookup_path = Rc::new(l_path);

        let install = Install::new(PresetMode::Enable, lookup_path);
        let presets = install.read_presets();

        assert_eq!(
            presets.unit_preset_action("basic.target".to_string()),
            PresetAction::Disable
        );
        assert_eq!(
            presets.unit_preset_action("tmp.service".to_string()),
            PresetAction::Disable
        );
    }

    #[test]
    fn test_preset_all() {
        let mut l_path = LookupPaths::new();
        l_path.init_lookup_paths();
        let lookup_path = Rc::new(l_path);
        let install = Install::new(PresetMode::Enable, lookup_path);
        assert!(install.preset_all().is_ok());
    }
}

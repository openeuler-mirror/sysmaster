// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use basic::path_lookup::LookupPaths;
use basic::time_util;
use siphasher::sip::SipHasher24;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct UnitFile {
    data: RefCell<UnitFileData>,
}

impl UnitFile {
    pub fn new(lookup_path: &Rc<LookupPaths>) -> UnitFile {
        UnitFile {
            data: RefCell::new(UnitFileData::new(lookup_path)),
        }
    }

    pub fn build_name_map(&self, name: String, has_loaded: bool) {
        self.data.borrow_mut().build_id_map(name, has_loaded);
    }

    pub fn get_unit_id_fragment_pathbuf(&self, name: &String) -> Vec<PathBuf> {
        self.data.borrow().get_unit_id_fragment_pathbuf(name)
    }

    pub fn get_unit_wants_symlink_units(&self, name: &String) -> Vec<PathBuf> {
        self.data.borrow().get_unit_wants_symlink_units(name)
    }

    pub fn get_unit_requires_symlink_units(&self, name: &String) -> Vec<PathBuf> {
        self.data.borrow().get_unit_requires_symlink_units(name)
    }
}

#[derive(Debug)]
struct UnitFileData {
    pub unit_id_fragment: HashMap<String, Vec<PathBuf>>,
    pub unit_wants_symlink_units: HashMap<String, Vec<PathBuf>>,
    pub unit_requires_symlink_units: HashMap<String, Vec<PathBuf>>,
    _unit_name_map: HashMap<String, String>,
    last_updated_timestamp_hash: u64,
    lookup_path: Rc<LookupPaths>,
}

// the declaration "pub(self)" is for identification only.
impl UnitFileData {
    pub(self) fn new(lookup_path: &Rc<LookupPaths>) -> UnitFileData {
        UnitFileData {
            unit_id_fragment: HashMap::new(),
            unit_wants_symlink_units: HashMap::new(),
            unit_requires_symlink_units: HashMap::new(),
            _unit_name_map: HashMap::new(),
            lookup_path: lookup_path.clone(),
            last_updated_timestamp_hash: 0,
        }
    }

    pub(self) fn get_unit_id_fragment_pathbuf(&self, name: &String) -> Vec<PathBuf> {
        match self.unit_id_fragment.get(name) {
            Some(v) => v.to_vec(),
            None => Vec::new(),
        }
    }

    pub(self) fn get_unit_wants_symlink_units(&self, name: &String) -> Vec<PathBuf> {
        match self.unit_wants_symlink_units.get(name) {
            Some(v) => v.to_vec(),
            None => Vec::<PathBuf>::new(),
        }
    }

    pub(self) fn get_unit_requires_symlink_units(&self, name: &String) -> Vec<PathBuf> {
        match self.unit_requires_symlink_units.get(name) {
            Some(v) => v.to_vec(),
            None => Vec::<PathBuf>::new(),
        }
    }

    pub(self) fn build_id_map(&mut self, name: String, has_loaded: bool) {
        if !has_loaded || self.lookup_paths_updated() {
            self.build_id_fragment(&name);
            self.build_id_dropin(&name, "wants".to_string());
            self.build_id_dropin(&name, "requires".to_string());
        }
    }

    fn build_id_fragment_by_name(path: &String, name: &String) -> Option<Vec<PathBuf>> {
        let mut res: Vec<PathBuf> = Vec::new();
        if fs::metadata(path).is_err() {
            return None;
        }
        /* {/etc/sysmaster/system, /usr/lib/sysmaster/system}/foo.service.d */
        let pathd_str = format!("{path}/{name}.d");
        let dir = Path::new(&pathd_str);
        if dir.is_dir() {
            for entry in dir.read_dir().unwrap() {
                let fragment = entry.unwrap().path();
                if !fragment.is_file() {
                    continue;
                }
                let file_name = String::from(fragment.file_name().unwrap().to_str().unwrap());
                if file_name.starts_with('.') || file_name.ends_with(".toml") {
                    continue;
                }
                res.push(fragment);
            }
        }
        /* {/etc/sysmater/system, /usr/lib/sysmaster/system}/foo.service */
        let config_path = Path::new(path).join(name);
        if !config_path.exists() {
            return None;
        }
        /* Symlink is complicated, it is related to Alias, skip it for now. */
        if config_path.is_symlink() {
            return None;
        }

        res.push(config_path);
        Some(res)
    }

    fn build_id_fragment(&mut self, name: &String) {
        let mut pathbuf_fragment = Vec::new();
        for search_path in &self.lookup_path.search_path {
            let mut v = match Self::build_id_fragment_by_name(search_path, name) {
                None => continue,
                Some(v) => v,
            };
            pathbuf_fragment.append(&mut v);
        }
        if !pathbuf_fragment.is_empty() || !name.contains('@') {
            self.unit_id_fragment
                .insert(name.to_string(), pathbuf_fragment);
            return;
        }
        let template_name = name.split_once('@').unwrap().0.to_string() + "@.service";
        for search_path in &self.lookup_path.search_path {
            let mut v = match Self::build_id_fragment_by_name(search_path, &template_name) {
                None => continue,
                Some(v) => v,
            };
            pathbuf_fragment.append(&mut v);
        }
        self.unit_id_fragment
            .insert(name.to_string(), pathbuf_fragment);
    }

    fn build_id_dropin(&mut self, name: &String, suffix: String) {
        let mut pathbuf_dropin = Vec::new();
        for v in &self.lookup_path.search_path {
            let path = format!("{v}/{name}.{suffix}");
            let dir = Path::new(&path);
            if !dir.is_dir() {
                continue;
            }
            for entry in dir.read_dir().unwrap() {
                let symlink_unit = entry.unwrap().path();
                if !symlink_unit.is_symlink() {
                    continue;
                }
                let abs_path = match symlink_unit.canonicalize() {
                    Err(_) => continue,
                    Ok(v) => v,
                };
                let mut file_name = PathBuf::new();
                file_name.push(abs_path.file_name().unwrap());
                pathbuf_dropin.push(file_name);
            }
        }

        match suffix.as_str() {
            "wants" => self
                .unit_wants_symlink_units
                .insert(name.to_string(), pathbuf_dropin),
            "requires" => self
                .unit_requires_symlink_units
                .insert(name.to_string(), pathbuf_dropin),
            _ => unimplemented!(),
        };
    }

    pub(self) fn lookup_paths_updated(&mut self) -> bool {
        let mut siphash24 = SipHasher24::new_with_keys(0, 0);
        for dir in &self.lookup_path.search_path {
            let metadata = match fs::metadata(dir) {
                Err(e) => {
                    log::debug!("Couldn't find unit config lookup path {dir}: {e}");
                    continue;
                }
                Ok(v) => v,
            };
            let time = match metadata.modified() {
                Err(_) => {
                    log::error!("Failed to get mtime of {dir}");
                    continue;
                }
                Ok(v) => v,
            };
            siphash24.write_u128(time_util::timespec_load(time));
        }

        let updated: u64 = siphash24.finish();

        let path_updated = updated != self.last_updated_timestamp_hash;
        self.last_updated_timestamp_hash = updated;
        path_updated
    }
}

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

//!
use crate::config;
use crate::error::*;
use nix::unistd::Pid;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

fn cmdline_content() -> Result<String> {
    let mut file = File::open("/proc/cmdline").context(IoSnafu)?;

    let mut buf = String::new();
    match file.read_to_string(&mut buf) {
        Ok(s) => s,
        Err(e) => {
            return Err(Error::Io { source: e });
        }
    };
    Ok(buf)
}

/// read the content from /proc/cmdline and return the value depend the key
pub fn cmdline_get_value(key: &str) -> Result<Option<String>, Error> {
    let buf = cmdline_content()?;

    let cmdline: Vec<&str> = buf.split_whitespace().collect();

    for cmd in cmdline.iter() {
        if let Some(k_val) = cmd.split_once('=') {
            if k_val.0 == key {
                return Ok(Some(k_val.1.to_string()));
            }
        }
    }

    Ok(None)
}

/// read the content from /proc/cmdline and return specified item
///-
/// take `crashkernel=512M ro` for example, given `crashkernel` will
/// return `crashkernel=512M`, given `ro` will return `ro`, given
/// `foo` will return None.
pub fn cmdline_get_item(item: &str) -> Result<Option<String>, Error> {
    let buf = cmdline_content()?;
    let pair_item = item.to_string() + "=";
    let cmdline: Vec<&str> = buf.split_whitespace().collect();

    for cmd in cmdline.iter() {
        if cmd.starts_with(&pair_item) || cmd.eq(&item) {
            return Ok(Some(cmd.to_string()));
        }
    }

    Ok(None)
}

/// read the content from /proc/cmdline and return the bool value depend the key
pub fn proc_cmdline_get_bool(key: &str) -> Result<bool, Error> {
    let val = cmdline_get_value(key)?;

    if val.is_none() {
        return Ok(false);
    }

    let r = config::parse_boolean(&val.unwrap())?;

    Ok(r)
}

/// parse cmdline item, insert module_blacklist's value to data
pub fn parse_proc_cmdline_item(key: String, value: String, data: &mut HashSet<String>) {
    if key.eq("module_blacklist") {
        if value.is_empty() {
            return;
        }

        let k: Vec<&str> = value.split(',').collect();

        for i in k {
            data.insert(i.to_string());
        }
    }
}

/// parse /proc/cmdline, Distinguish between key and value based on '='
pub fn proc_cmdline_parse<F, T>(parse_item: F, data: &mut T)
where
    F: Fn(String, String, &mut T),
{
    let line = read_file(Path::new("/proc/cmdline"));
    if line.is_empty() {
        log::info!("/proc/1/cmdline is empty!");
        return;
    }

    let v: Vec<&str> = line.split(' ').collect();
    for i in &v {
        let key = match i.to_string().split('=').next() {
            None => continue,
            Some(k) => k.to_string(),
        };
        let value = match i.to_string().split('=').nth(1) {
            None => continue,
            Some(v) => v.to_string(),
        };
        parse_item(key, value, data);
    }
}

/// read /proc/PID/cmdline and return
pub fn get_process_cmdline(pid: &Pid) -> String {
    let pid_str = pid.to_string();
    read_file(&Path::new("/proc").join(pid_str).join("cmdline"))
}

fn read_file(path: &Path) -> String {
    let cmdline_path = path;
    let file = match File::open(cmdline_path) {
        Ok(file) => file,
        Err(_) => {
            return String::from("");
        }
    };
    let buf_reader = BufReader::new(file);
    let mut cmdline_content = String::new();
    for byte in buf_reader.bytes() {
        let b = match byte {
            Ok(b) => b,
            Err(_) => break,
        };
        let b = if b != 0 { b as char } else { ' ' };
        cmdline_content += &b.to_string();
    }
    cmdline_content
}

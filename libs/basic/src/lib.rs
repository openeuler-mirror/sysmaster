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
#[cfg(feature = "argv")]
pub mod argv;
#[cfg(feature = "capability")]
pub mod capability;
#[cfg(feature = "cargo")]
pub mod cargo;
#[cfg(feature = "cmdline")]
pub mod cmdline;
#[cfg(feature = "condition")]
pub mod condition;
#[cfg(feature = "config")]
pub mod config;
#[cfg(feature = "cpu")]
pub mod cpu;
#[cfg(feature = "disk")]
pub mod disk;
#[cfg(feature = "exec")]
pub mod exec;
#[cfg(feature = "fd")]
pub mod fd;
#[cfg(feature = "fs")]
pub mod fs;
#[cfg(feature = "host")]
pub mod host;
#[cfg(feature = "id128")]
pub mod id128;
#[cfg(feature = "io")]
pub mod io;
#[cfg(feature = "machine")]
pub mod machine;
#[cfg(feature = "memory")]
pub mod memory;
#[cfg(feature = "mount")]
pub mod mount;
#[cfg(feature = "murmurhash2")]
pub mod murmurhash2;
#[cfg(feature = "namespace")]
pub mod namespace;
#[cfg(feature = "naming_scheme")]
pub mod naming_scheme;
#[cfg(feature = "network")]
pub mod network;
#[cfg(feature = "os_release")]
pub mod os_release;
#[cfg(feature = "parse")]
pub mod parse;
#[cfg(feature = "path")]
pub mod path;
#[cfg(feature = "process")]
pub mod process;
#[cfg(feature = "random")]
pub mod random;
#[cfg(feature = "rlimit")]
pub mod rlimit;
#[cfg(feature = "security")]
pub mod security;
#[cfg(feature = "sensors")]
pub mod sensors;
#[cfg(feature = "show_table")]
pub mod show_table;
#[cfg(feature = "signal")]
pub mod signal;
#[cfg(feature = "socket")]
pub mod socket;
#[cfg(feature = "stat")]
pub mod stat;
#[cfg(feature = "strbuf")]
pub mod strbuf;
#[cfg(feature = "string")]
pub mod string;
#[cfg(feature = "sysfs")]
pub mod sysfs;
#[cfg(feature = "time")]
pub mod time;
#[cfg(feature = "unistd")]
pub mod unistd;
#[cfg(feature = "unit_name")]
pub mod unit_name;
#[cfg(feature = "uuid")]
pub mod uuid;

pub mod error;
pub mod macros;

pub use error::*;

/// default startup target
pub const DEFAULT_TARGET: &str = "default.target";
/// the shutdown target
pub const SHUTDOWN_TARGET: &str = "shutdown.target";
/// the socketc target
pub const SOCKETS_TARGET: &str = "sockets.target";

/// the timer target
pub const TIMERS_TARGET: &str = "timers.target";

/// early boot targets
pub const SYSINIT_TARGET: &str = "sysinit.target";
/// the basic start target
pub const BASIC_TARGET: &str = "basic.target";

/// Special user boot targets */
pub const MULTI_USER_TARGET: &str = "multi-user.target";

/// the init scope
pub const INIT_SCOPE: &str = "init.scope";
/// sysmaster service slice
pub const SYSMASTER_SLICE: &str = "system.slice";

/// the unit store sysmaster itself
pub const CGROUP_SYSMASTER: &str = "sysmaster";

/// the default running time directory of sysmaster
pub const EXEC_RUNTIME_PREFIX: &str = "/run";

/// the default mount command path
pub const MOUNT_BIN: &str = "/usr/bin/mount";

/// the default umount command path
pub const UMOUNT_BIN: &str = "/usr/bin/umount";

/// the default swapon path
pub const SWAP_BIN: &str = "/usr/sbin/swapon";

/// the default fstab config file path
pub const FSTAB_PATH: &str = "/etc/fstab";

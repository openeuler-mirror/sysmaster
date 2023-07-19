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

use crate::mng::RunningData;

use super::comm::ServiceUnitComm;
use super::config::ServiceConfig;
use super::pid::ServicePid;
use super::rentry::ServiceType;
use basic::fd_util;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use sysmaster::error::*;
use sysmaster::exec::{ExecCommand, ExecContext, ExecFlags, ExecParameters};

pub(super) struct ServiceSpawn {
    comm: Rc<ServiceUnitComm>,
    pid: Rc<ServicePid>,
    socket_fd: RefCell<i32>,
    config: Rc<ServiceConfig>,
    exec_ctx: Rc<ExecContext>,
    rd: Rc<RunningData>,
    exec_params: RefCell<Option<ExecParameters>>,
}

impl ServiceSpawn {
    pub(super) fn new(
        commr: &Rc<ServiceUnitComm>,
        pidr: &Rc<ServicePid>,
        configr: &Rc<ServiceConfig>,
        exec_ctx: &Rc<ExecContext>,
        rd: &Rc<RunningData>,
    ) -> ServiceSpawn {
        ServiceSpawn {
            comm: Rc::clone(commr),
            pid: Rc::clone(pidr),
            socket_fd: RefCell::new(-1),
            config: configr.clone(),
            exec_ctx: exec_ctx.clone(),
            rd: rd.clone(),
            exec_params: RefCell::new(None),
        }
    }

    pub(super) fn start_service(
        &self,
        cmdline: &ExecCommand,
        time_out: u64,
        ec_flags: ExecFlags,
    ) -> Result<Pid> {
        let mut params = ExecParameters::new();
        let config_refcell = self.config.config_data();
        let service_config = &config_refcell.borrow().Service;

        params.set_exec_flags(ec_flags);
        params.set_nonblock(service_config.NonBlocking);

        params.add_env(
            "PATH",
            env::var("PATH").unwrap_or_else(|_| {
                "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()
            }),
        );

        if let Some(pid) = self.pid.main() {
            params.add_env("MAINPID", format!("{pid}"));
        }
        let unit = match self.comm.owner() {
            None => {
                return Err("spawn exec return error".to_string().into());
            }
            Some(v) => v,
        };
        let um = self.comm.um();
        unit.prepare_exec()?;

        self.rd.enable_timer(time_out)?;

        if ec_flags.contains(ExecFlags::PASS_FDS) {
            params.insert_fds(self.collect_socket_fds());
        }

        if self.config.service_type() == ServiceType::Notify || service_config.WatchdogSec > 0 {
            let notify_sock = um.notify_socket().unwrap();
            log::debug!("add NOTIFY_SOCKET env: {}", notify_sock.to_str().unwrap());
            params.add_env("NOTIFY_SOCKET", notify_sock.to_str().unwrap().to_string());
            params.set_notify_sock(notify_sock);
        }

        if let Err(e) = params.add_user(service_config.User.clone()) {
            log::error!("Failed to add user to execute parameters: {e}");
            return Err(e);
        }

        if let Err(e) = params.add_group(service_config.Group.clone()) {
            log::error!("Failed to add group to execute parameters: {e}");
            return Err(e);
        }

        if let Err(e) = params.add_umask(service_config.UMask.clone()) {
            log::error!("Failed to add umask to execute parameters: {e}");
            return Err(e);
        }

        if let Err(e) = params.add_root_directory(service_config.RootDirectory.clone()) {
            log::error!("Failed to add root directory: {e}");
            return Err(e);
        }

        if let Some(runtime_directories) = &service_config.RuntimeDirectory {
            if let Err(e) = params.add_runtime_directory(runtime_directories) {
                log::error!("Failed to add runtime directories: {e}");
                return Err(e);
            }
        }

        if let Some(state_directories) = &service_config.StateDirectory {
            if let Err(e) = params.add_state_directory(state_directories) {
                log::error!("Failed to add state directories: {e}");
                return Err(e);
            }
        }

        if let Err(e) = params.add_working_directory(service_config.WorkingDirectory.clone()) {
            log::error!("Failed to add working directory: {e}");
            return Err(e);
        }

        params.set_watchdog_usec(self.watchdog_timer());

        log::debug!("begin to exec spawn");
        let pid = match um.exec_spawn(unit.id(), cmdline, &params, self.exec_ctx.clone()) {
            Ok(pid) => {
                um.child_watch_pid(unit.id(), pid);
                pid
            }
            Err(e) => {
                log::error!("failed to start service: {}, error:{:?}", unit.id(), e);
                return Err("spawn exec return error".to_string().into());
            }
        };
        *(self.exec_params.borrow_mut()) = Some(params);
        Ok(pid)
    }

    pub fn get_runtime_directory(&self) -> Option<Vec<PathBuf>> {
        match self.exec_params.borrow().as_ref() {
            None => None,
            Some(v) => v.get_runtime_directory(),
        }
    }

    fn collect_socket_fds(&self) -> Vec<i32> {
        if self.get_socket_fd() >= 0 {
            vec![self.get_socket_fd()]
        } else {
            self.comm.um().collect_socket_fds(&self.comm.get_owner_id())
        }
    }

    fn watchdog_timer(&self) -> u64 {
        self.config.config_data().borrow().Service.WatchdogSec
    }

    pub(super) fn set_socket_fd(&self, fd: i32) {
        *self.socket_fd.borrow_mut() = fd;
    }

    pub(super) fn get_socket_fd(&self) -> i32 {
        *self.socket_fd.borrow()
    }

    pub(super) fn release_socket_fd(&self, fd: i32) {
        fd_util::close(fd);
        *self.socket_fd.borrow_mut() = -1;
    }
}

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
//
#![allow(non_snake_case)]
use confique::Config;
use core::exec::ExecCommand;
use core::rel::{ReDb, ReDbRwTxn, ReDbTable, Reliability};
use core::serialize::DeserializeWith;
use core::unit::KillMode;
use macros::EnumDisplay;
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::os::unix::prelude::RawFd;
use std::path::PathBuf;
use std::rc::Rc;

struct SocketReDb<K, V>(ReDb<K, V>);

const RELI_DB_HSOCKET_CONF: &str = "sockconf";
const RELI_DB_HSOCKET_MNG: &str = "sockmng";
const RELI_DB_HSOCKETM_FRAME: &str = "sockm-frame";
const RELI_LAST_KEY: u32 = 0; // singleton

#[derive(Config, Default, Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub(super) struct SectionSocket {
    #[config(deserialize_with = ExecCommand::deserialize_with)]
    pub ExecStartPre: Option<VecDeque<ExecCommand>>,
    #[config(deserialize_with = ExecCommand::deserialize_with)]
    pub ExecStartChown: Option<VecDeque<ExecCommand>>,
    #[config(deserialize_with = ExecCommand::deserialize_with)]
    pub ExecStartPost: Option<VecDeque<ExecCommand>>,
    #[config(deserialize_with = ExecCommand::deserialize_with)]
    pub ExecStopPre: Option<VecDeque<ExecCommand>>,
    #[config(deserialize_with = ExecCommand::deserialize_with)]
    pub ExecStopPost: Option<VecDeque<ExecCommand>>,

    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    pub ListenStream: Option<Vec<String>>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    pub ListenDatagram: Option<Vec<String>>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    pub ListenNetlink: Option<Vec<String>>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    pub ListenSequentialPacket: Option<Vec<String>>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    pub ListenFIFO: Option<Vec<String>>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    pub ListenSpecial: Option<Vec<String>>,

    #[config(default = false)]
    pub Accept: bool,
    #[config(default = false)]
    pub FlushPending: bool,
    pub Service: Option<String>,
    pub ReceiveBuffer: Option<u64>,
    pub SendBuffer: Option<u64>,
    pub PassCredentials: Option<bool>,
    pub PassPacketInfo: Option<bool>,
    pub KeepAlive: Option<bool>,
    pub KeepAliveTimeSec: Option<u32>,
    pub KeepAliveIntervalSec: Option<u32>,
    pub KeepAliveProbes: Option<u32>,
    pub Broadcast: Option<bool>,
    #[config(default = false)]
    pub RemoveOnStop: bool,
    #[config(deserialize_with = Vec::<PathBuf>::deserialize_with)]
    pub Symlinks: Option<Vec<PathBuf>>,
    pub PassSecurity: Option<bool>,
    #[config(default = 0o666)]
    pub SocketMode: u32,
    #[config(default = "")]
    pub SocketUser: String,
    #[config(default = "")]
    pub SocketGroup: String,
    #[config(deserialize_with = KillMode::deserialize_with)]
    #[config(default = "none")]
    pub KillMode: KillMode,
    #[config(default = "SIGTERM")]
    pub KillSignal: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SocketReConf {
    socket: SectionSocket,
    service: Option<String>,
}

impl SocketReConf {
    fn new(socketr: &SectionSocket, service: Option<String>) -> SocketReConf {
        SocketReConf {
            socket: socketr.clone(),
            service,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize, EnumDisplay)]
pub(crate) enum SocketState {
    Dead,
    StartPre,
    StartChown,
    StartPost,
    Listening,
    Running,
    StopPre,
    StopPreSigterm,
    StopPreSigkill,
    StopPost,
    FinalSigterm,
    FinalSigkill,
    Failed,
    Cleaning,
    StateMax,
}

#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub(super) enum SocketResult {
    Success,
    FailureResources,
    FailureTimeout,
    FailureExitCode,
    FailureSignal,
    FailureCoreDump,
    FailureStartLimitHit,
    FailureTriggerLimitHit,
    FailureServiceStartLimitHit,
    ResultInvalid,
}

/// the command that running in different stage.
#[allow(dead_code)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone, Serialize, Deserialize)]
pub(super) enum SocketCommand {
    StartPre,
    StartPost,
    StopPre,
    StopPost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(super) enum PortType {
    Socket,
    Fifo,
    Special,
    Invalid,
}

impl Default for PortType {
    fn default() -> Self {
        Self::Socket
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SocketReMng {
    state: SocketState,
    result: SocketResult,
    control_pid: Option<i32>, // i32 ==> nix::unistd::Pid ==> libc::pid_t
    control_cmd_type: Option<SocketCommand>,
    control_cmd_len: usize,
    refused: i32,
    ports: Vec<(PortType, String, i32)>, // i32 ==> std::os::unix::prelude::RawFd ==> std::os::raw::c_int
}

impl SocketReMng {
    fn new(
        state: SocketState,
        result: SocketResult,
        control_pid: Option<i32>,
        control_cmd_type: Option<SocketCommand>,
        control_cmd_len: usize,
        refused: i32,
        ports: Vec<(PortType, String, i32)>,
    ) -> SocketReMng {
        SocketReMng {
            state,
            result,
            control_pid,
            control_cmd_type,
            control_cmd_len,
            refused,
            ports,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) enum SocketReFrame {
    FdListen(bool), // spread?
}

pub(super) struct SocketRe {
    // database: multi-instance(N)
    conf: Rc<SocketReDb<String, SocketReConf>>, // RELI_DB_HSOCKET_CONF; key: unit_id, data: config;
    mng: Rc<SocketReDb<String, SocketReMng>>, // RELI_DB_HSOCKET_MNG; key: unit_id, data: state+result+control(pid+cmd)+refused+ports(fd);

    // database: singleton(1)
    frame: Rc<SocketReDb<u32, SocketReFrame>>, // RELI_DB_HSOCKETM_FRAME; key: RELI_LAST_KEY, data: SocketReFrame;
}

impl SocketRe {
    pub(super) fn new(relir: &Rc<Reliability>) -> SocketRe {
        let conf = Rc::new(SocketReDb(ReDb::new(relir, RELI_DB_HSOCKET_CONF)));
        let mng = Rc::new(SocketReDb(ReDb::new(relir, RELI_DB_HSOCKET_MNG)));
        let frame = Rc::new(SocketReDb(ReDb::new(relir, RELI_DB_HSOCKETM_FRAME)));
        let rentry = SocketRe { conf, mng, frame };
        rentry.register(relir);
        rentry
    }

    pub(super) fn conf_insert(
        &self,
        unit_id: &str,
        socket: &SectionSocket,
        service: Option<String>,
    ) {
        let conf = SocketReConf::new(socket, service);
        self.conf.0.insert(unit_id.to_string(), conf);
    }

    pub(super) fn _conf_remove(&self, unit_id: &str) {
        self.conf.0.remove(&unit_id.to_string());
    }

    pub(super) fn conf_get(&self, unit_id: &str) -> Option<(SectionSocket, Option<String>)> {
        let conf = self.conf.0.get(&unit_id.to_string());
        conf.map(|c| (c.socket, c.service))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn mng_insert(
        &self,
        unit_id: &str,
        state: SocketState,
        result: SocketResult,
        control_pid: Option<Pid>,
        control_cmd_type: Option<SocketCommand>,
        control_cmd_len: usize,
        refused: i32,
        ports: Vec<(PortType, String, RawFd)>,
    ) {
        let c_pid = control_pid.map(|x| x.as_raw());
        let ps = ports
            .iter()
            .map(|(t, l, id)| (*t, l.clone(), *id))
            .collect::<_>();
        let mng = SocketReMng::new(
            state,
            result,
            c_pid,
            control_cmd_type,
            control_cmd_len,
            refused,
            ps,
        );
        self.mng.0.insert(unit_id.to_string(), mng);
    }

    pub(super) fn _mng_remove(&self, unit_id: &str) {
        self.mng.0.remove(&unit_id.to_string());
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn mng_get(
        &self,
        unit_id: &str,
    ) -> Option<(
        SocketState,
        SocketResult,
        Option<Pid>,
        Option<SocketCommand>,
        usize,
        i32,
        Vec<(PortType, String, RawFd)>,
    )> {
        let mng = self.mng.0.get(&unit_id.to_string());
        mng.map(|m| {
            (
                m.state,
                m.result,
                m.control_pid.map(Pid::from_raw),
                m.control_cmd_type,
                m.control_cmd_len,
                m.refused,
                m.ports
                    .iter()
                    .map(|(t, l, id)| (*t, l.clone(), *id as RawFd))
                    .collect::<_>(),
            )
        })
    }

    pub(super) fn set_last_frame(&self, frame: SocketReFrame) {
        self.frame.0.insert(RELI_LAST_KEY, frame);
    }

    pub(super) fn clear_last_frame(&self) {
        self.frame.0.remove(&RELI_LAST_KEY);
    }

    pub(super) fn last_frame(&self) -> Option<SocketReFrame> {
        self.frame.0.get(&RELI_LAST_KEY)
    }

    fn register(&self, relir: &Reliability) {
        // rel-db: RELI_DB_HSOCKET_CONF
        let db = Rc::clone(&self.conf);
        relir.history_db_register(RELI_DB_HSOCKET_CONF, db);

        // rel-db: RELI_DB_HSOCKET_MNG
        let db = Rc::clone(&self.mng);
        relir.history_db_register(RELI_DB_HSOCKET_MNG, db);

        // rel-db: RELI_DB_HSOCKETM_FRAME
        let db = Rc::clone(&self.frame);
        relir.history_db_register(RELI_DB_HSOCKETM_FRAME, db);
    }
}

impl ReDbTable for SocketReDb<String, SocketReConf> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn flush(&self, db_wtxn: &mut ReDbRwTxn, switch: bool) {
        self.0.data_2_db(db_wtxn, switch);
    }

    fn import<'a>(&self) {
        self.0.db_2_cache();
    }

    fn switch_set(&self, switch: Option<bool>) {
        self.0.switch_buffer(switch);
    }
}

impl ReDbTable for SocketReDb<String, SocketReMng> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn flush(&self, db_wtxn: &mut ReDbRwTxn, switch: bool) {
        self.0.data_2_db(db_wtxn, switch);
    }

    fn import<'a>(&self) {
        self.0.db_2_cache();
    }

    fn switch_set(&self, switch: Option<bool>) {
        self.0.switch_buffer(switch);
    }
}

impl ReDbTable for SocketReDb<u32, SocketReFrame> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn flush(&self, db_wtxn: &mut ReDbRwTxn, switch: bool) {
        self.0.data_2_db(db_wtxn, switch);
    }

    fn import<'a>(&self) {
        self.0.db_2_cache();
    }

    fn switch_set(&self, switch: Option<bool>) {
        self.0.switch_buffer(switch);
    }
}

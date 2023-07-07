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

use crate::unit::rentry::{UeConfigInstall, UeConfigUnit, UnitLoadState, UnitRe, UnitRePps};
use core::rel::ReStation;
use core::unit::UnitType;
use nix::unistd::Pid;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub(super) struct UeBase {
    // associated objects
    rentry: Rc<UnitRe>,

    // owned objects
    id: String,
    unit_type: UnitType,
}

impl ReStation for UeBase {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        if reload {
            return;
        }
        let unit_type = self.rentry.base_get(&self.id).unwrap();
        assert_eq!(self.unit_type, unit_type);
    }

    fn db_insert(&self) {
        self.rentry.base_insert(&self.id, self.unit_type);
        self.rentry.pps_insert(&self.id);
    }

    // reload: no external connections, no entry
}

impl UeBase {
    pub(super) fn new(rentryr: &Rc<UnitRe>, id: String, unit_type: UnitType) -> UeBase {
        let base = UeBase {
            rentry: Rc::clone(rentryr),
            id,
            unit_type,
        };
        base.db_insert();
        base
    }

    pub(super) fn id(&self) -> &String {
        &self.id
    }

    pub(super) fn unit_type(&self) -> UnitType {
        self.unit_type
    }

    pub(super) fn rentry_load_insert(&self, load_state: UnitLoadState) {
        self.rentry.load_insert(&self.id, load_state);
    }

    pub(super) fn rentry_load_get(&self) -> Option<UnitLoadState> {
        self.rentry.load_get(&self.id)
    }

    pub(super) fn rentry_conf_insert(&self, unit: &UeConfigUnit, install: &UeConfigInstall) {
        self.rentry.conf_insert(&self.id, unit, install);
    }

    pub(super) fn rentry_conf_get(&self) -> Option<(UeConfigUnit, UeConfigInstall)> {
        self.rentry.conf_get(&self.id)
    }

    pub(super) fn rentry_cgroup_insert(&self, cg_path: &Path) {
        self.rentry.cgroup_insert(&self.id, cg_path);
    }

    pub(super) fn rentry_cgroup_get(&self) -> Option<PathBuf> {
        self.rentry.cgroup_get(&self.id)
    }

    pub(super) fn rentry_child_insert(&self, pids: &[Pid]) {
        self.rentry.child_insert(&self.id, pids);
    }

    pub(super) fn rentry_child_get(&self) -> Vec<Pid> {
        self.rentry.child_get(&self.id)
    }

    pub(super) fn rentry_pps_set(&self, pps_mask: UnitRePps) {
        self.rentry.pps_set(&self.id, pps_mask);
    }

    pub(super) fn rentry_pps_clear(&self, pps_mask: UnitRePps) {
        self.rentry.pps_clear(&self.id, pps_mask);
    }

    #[allow(dead_code)]
    pub(super) fn rentry_pps_contains(&self, pps_mask: UnitRePps) -> bool {
        self.rentry.pps_contains(&self.id, pps_mask)
    }
}

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

use super::base::UeBase;
use super::config::UeConfig;
use crate::unit::data::{DataManager, UnitDepConf};
use crate::unit::rentry::{UnitLoadState, UnitRePps};
use crate::unit::util::UnitFile;
use core::error::*;
use core::rel::ReStation;
use core::unit::UnitRelations;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

//#[derive(Debug)]
pub(super) struct UeLoad {
    // associated objects
    dm: Rc<DataManager>,
    file: Rc<UnitFile>,
    base: Rc<UeBase>,
    config: Rc<UeConfig>,

    // owned objects
    load_state: RefCell<UnitLoadState>,
    in_load_queue: RefCell<bool>,
    in_target_dep_queue: RefCell<bool>,
}

impl ReStation for UeLoad {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        if reload {
            return;
        }
        if let Some(load_state) = self.base.rentry_load_get() {
            *self.load_state.borrow_mut() = load_state;
        }
    }

    fn db_insert(&self) {
        self.base.rentry_load_insert(*self.load_state.borrow());
    }

    // reload: no external connections, no entry
}

impl UeLoad {
    pub(super) fn new(
        dmr: &Rc<DataManager>,
        filer: &Rc<UnitFile>,
        baser: &Rc<UeBase>,
        config: &Rc<UeConfig>,
    ) -> UeLoad {
        let load = UeLoad {
            dm: Rc::clone(dmr),
            file: Rc::clone(filer),
            base: Rc::clone(baser),
            config: Rc::clone(config),
            load_state: RefCell::new(UnitLoadState::Stub),
            in_load_queue: RefCell::new(false),
            in_target_dep_queue: RefCell::new(false),
        };
        load.db_insert();
        let flags = UnitRePps::QUEUE_LOAD | UnitRePps::QUEUE_TARGET_DEPS;
        load.base.rentry_pps_clear(flags);
        load
    }

    pub(super) fn get_description(&self) -> Option<String> {
        let res = String::from(&self.config.config_data().borrow().Unit.Description);
        if res.is_empty() {
            None
        } else {
            Some(res)
        }
    }

    pub(super) fn get_documentation(&self) -> Option<String> {
        let res = String::from(&self.config.config_data().borrow().Unit.Documentation);
        if res.is_empty() {
            None
        } else {
            Some(res)
        }
    }

    pub(super) fn get_unit_id_fragment_pathbuf(&self) -> Vec<PathBuf> {
        self.file.get_unit_id_fragment_pathbuf(self.base.id())
    }

    pub(super) fn set_load_state(&self, load_state: UnitLoadState) {
        *self.load_state.borrow_mut() = load_state;
        self.db_update();
    }

    pub(super) fn load_state(&self) -> UnitLoadState {
        let state = self.load_state.clone();
        state.into_inner()
    }

    pub(super) fn set_in_load_queue(&self, t: bool) {
        *self.in_load_queue.borrow_mut() = t;
        if t {
            self.base.rentry_pps_set(UnitRePps::QUEUE_LOAD);
        } else {
            self.base.rentry_pps_clear(UnitRePps::QUEUE_LOAD);
        }
    }

    pub(super) fn in_load_queue(&self) -> bool {
        *self.in_load_queue.borrow()
    }

    pub(super) fn load_unit_confs(&self) -> Result<()> {
        self.file.build_name_map(
            self.base.id().clone(),
            self.load_state() == UnitLoadState::Loaded,
        );
        self.config
            .load_fragment_and_dropin(self.file.as_ref(), self.base.id())?;
        self.parse();
        Ok(())
    }

    pub(super) fn set_in_target_dep_queue(&self, t: bool) {
        self.in_target_dep_queue.replace(t);
        if t {
            self.base.rentry_pps_set(UnitRePps::QUEUE_TARGET_DEPS);
        } else {
            self.base.rentry_pps_clear(UnitRePps::QUEUE_TARGET_DEPS);
        }
    }

    pub(super) fn in_target_dep_queue(&self) -> bool {
        *self.in_target_dep_queue.borrow()
    }

    fn parse(&self) {
        let mut ud_conf = UnitDepConf::new(); // need get config from config database,and update depends hereW
        let config_data = self.config.config_data();
        let ud_conf_insert_table = vec![
            (
                UnitRelations::UnitWants,
                config_data.borrow().Unit.Wants.clone(),
            ),
            (
                UnitRelations::UnitAfter,
                config_data.borrow().Unit.After.clone(),
            ),
            (
                UnitRelations::UnitBefore,
                config_data.borrow().Unit.Before.clone(),
            ),
            (
                UnitRelations::UnitRequires,
                config_data.borrow().Unit.Requires.clone(),
            ),
            (
                UnitRelations::UnitBindsTo,
                config_data.borrow().Unit.BindsTo.clone(),
            ),
            (
                UnitRelations::UnitRequisite,
                config_data.borrow().Unit.Requisite.clone(),
            ),
            (
                UnitRelations::UnitOnFailure,
                config_data.borrow().Unit.OnFailure.clone(),
            ),
            (
                UnitRelations::UnitOnSuccess,
                config_data.borrow().Unit.OnSuccess.clone(),
            ),
            (
                UnitRelations::UnitPartOf,
                config_data.borrow().Unit.PartOf.clone(),
            ),
            (
                UnitRelations::UnitConflicts,
                config_data.borrow().Unit.Conflicts.clone(),
            ),
        ];

        for ud_conf_relation in ud_conf_insert_table {
            ud_conf.deps.insert(ud_conf_relation.0, ud_conf_relation.1);
        }

        self.dm.insert_ud_config(self.base.id().clone(), ud_conf);
    }
}

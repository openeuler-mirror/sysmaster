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

//! example for using enumerator
use device::{device_action::DeviceAction, device_enumerator::DeviceEnumerator};

fn main() {
    let enumerator = DeviceEnumerator::new();

    for device in enumerator {
        println!("{}", device.as_ref().lock().unwrap().get_devpath().unwrap());
        device
            .as_ref()
            .lock()
            .unwrap()
            .trigger(DeviceAction::Change)
            .unwrap();
    }
}

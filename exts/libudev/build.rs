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

//! libudev link options
//!

fn main() {
    let symbols = [
        "udev_device_get_action",
        "udev_device_get_devlinks_list_entry",
        "udev_device_get_devnode",
        "udev_device_get_devnum",
        "udev_device_get_devpath",
        "udev_device_get_devtype",
        "udev_device_get_driver",
        "udev_device_get_is_initialized",
        "udev_device_get_parent",
        "udev_device_get_parent_with_subsystem_devtype",
        "udev_device_get_properties_list_entry",
        "udev_device_get_property_value",
        "udev_device_get_seqnum",
        "udev_device_get_subsystem",
        // "udev_device_get_sysattr_list_entry",
        // "udev_device_get_sysattr_value",
        "udev_device_get_sysname",
        // "udev_device_get_sysnum",
        "udev_device_get_syspath",
        // "udev_device_get_tags_list_entry",
        // "udev_device_get_udev",
        // "udev_device_get_usec_since_initialized",
        "udev_device_has_tag",
        // "udev_device_new_from_devnum",
        "udev_device_new_from_environment",
        "udev_device_new_from_subsystem_sysname",
        "udev_device_new_from_syspath",
        "udev_device_ref",
        "udev_device_unref",
        "udev_enumerate_add_match_is_initialized",
        "udev_enumerate_add_match_parent",
        "udev_enumerate_add_match_property",
        "udev_enumerate_add_match_subsystem",
        "udev_enumerate_add_match_sysattr",
        // "udev_enumerate_add_match_sysname",
        "udev_enumerate_add_match_tag",
        // "udev_enumerate_add_nomatch_subsystem",
        // "udev_enumerate_add_nomatch_sysattr",
        // "udev_enumerate_add_syspath",
        "udev_enumerate_get_list_entry",
        "udev_enumerate_get_udev",
        "udev_enumerate_new",
        "udev_enumerate_ref",
        "udev_enumerate_scan_devices",
        // "udev_enumerate_scan_subsystems",
        "udev_enumerate_unref",
        // "udev_get_log_priority",
        // "udev_get_userdata",
        "udev_list_entry_get_by_name",
        "udev_list_entry_get_name",
        "udev_list_entry_get_next",
        "udev_list_entry_get_value",
        "udev_monitor_enable_receiving",
        "udev_monitor_filter_add_match_subsystem_devtype",
        "udev_monitor_filter_add_match_tag",
        // "udev_monitor_filter_remove",
        // "udev_monitor_filter_update",
        "udev_monitor_get_fd",
        "udev_monitor_get_udev",
        "udev_monitor_new_from_netlink",
        "udev_monitor_receive_device",
        "udev_monitor_ref",
        "udev_monitor_set_receive_buffer_size",
        "udev_monitor_unref",
        "udev_new",
        // "udev_queue_get_kernel_seqnum",
        // "udev_queue_get_queue_is_empty",
        // "udev_queue_get_queued_list_entry",
        // "udev_queue_get_seqnum_is_finished",
        // "udev_queue_get_seqnum_sequence_is_finished",
        // "udev_queue_get_udev",
        "udev_queue_get_udev_is_active",
        // "udev_queue_get_udev_seqnum",
        "udev_queue_new",
        "udev_queue_ref",
        "udev_queue_unref",
        "udev_ref",
        // "udev_set_log_fn",
        // "udev_set_log_priority",
        // "udev_set_userdata",
        "udev_unref",
        // "udev_util_encode_string",
        "udev_device_new_from_device_id",
        "udev_hwdb_new",
        "udev_hwdb_ref",
        "udev_hwdb_unref",
        "udev_hwdb_get_properties_list_entry",
        // "udev_device_set_sysattr_value",
        // "udev_queue_flush",
        // "udev_queue_get_fd",
        "udev_device_has_current_tag",
        // "udev_device_get_current_tags_list_entry",
    ];

    println!("cargo:rustc-cdylib-link-arg=-fuse-ld=lld");
    println!("cargo:rustc-link-arg=-Wl,--version-script=/root/sysmaster/exts/libudev/libudev.sym");

    for s in symbols {
        println!("cargo:rustc-link-arg=-Wl,--defsym={}={}_impl", s, s);
    }
}

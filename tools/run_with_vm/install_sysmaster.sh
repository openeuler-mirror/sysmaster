#!/usr/bin/env bash

mode="$1"
work_dir=$(pwd)
target_dir=${work_dir}/${mode}
install_dir=/usr/lib/sysmaster
conf_dir=/etc/sysmaster

install -Dm0550 -t /usr/bin ${target_dir}/sctl || exit 1
install -Dm0550 ${target_dir}/init /init || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/sysmaster || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/fstab || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/sysmonitor || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/random_seed || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/rc-local-generator || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/hostname_setup || exit 1

install -Dm0640 -t ${install_dir}/system ${target_dir}/basic.target || exit 1
install -Dm0640 -t ${install_dir}/system ${target_dir}/multi-user.target || exit 1
install -Dm0640 -t ${install_dir}/system ${target_dir}/shutdown.target || exit 1
install -Dm0640 -t ${install_dir}/system ${target_dir}/sysinit.target || exit 1

strip ${target_dir}/lib*.so

install -Dm0550 -t ${install_dir}/plugin ${target_dir}/libmount.so || exit 1
install -Dm0550 -t ${install_dir}/plugin ${target_dir}/libservice.so || exit 1
install -Dm0550 -t ${install_dir}/plugin ${target_dir}/libsocket.so || exit 1
install -Dm0550 -t ${install_dir}/plugin ${target_dir}/libtarget.so || exit 1
install -Dm0550 -t ${install_dir}/plugin ${target_dir}/conf/plugin.conf || exit 1
install -Dm0640 -t ${conf_dir} ${target_dir}/conf/system.conf || exit 1

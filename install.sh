#!/usr/bin/env bash

mode="$1"
work_dir=$(pwd)
target_dir=${work_dir}/target/${mode}
install_dir=${work_dir}/target/install/usr/lib/sysmaster

rm -rf "${work_dir}"/target/install

install -Dm0550 -t ${work_dir}/target/install/usr/bin ${target_dir}/sctl || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/init || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/sysmaster || exit 1

install -Dm0550 -t ${install_dir} ${target_dir}/fstab || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/sysmonitor || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/random_seed || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/rc-local-generator || exit 1
install -Dm0550 -t ${install_dir} ${target_dir}/hostname_setup || exit 1

install -Dm0640 -t ${install_dir} ${work_dir}/units/* || exit 1
install -Dm0640 -t ${install_dir}/plugin ${target_dir}/conf/plugin.conf || exit 1
install -Dm0640 -t ${work_dir}/target/install/etc/sysmaster ${target_dir}/conf/system.conf || exit 1

strip ${target_dir}/lib*.so

install -Dm0750 -t ${install_dir}/plugin ${target_dir}/libmount.so || exit 1
install -Dm0750 -t ${install_dir}/plugin ${target_dir}/libservice.so || exit 1
install -Dm0750 -t ${install_dir}/plugin ${target_dir}/libsocket.so || exit 1
install -Dm0750 -t ${install_dir}/plugin ${target_dir}/libtarget.so || exit 1

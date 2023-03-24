#!/bin/bash

work_dir="$(dirname "$0")"
source "${work_dir}"/util_lib.sh

set +e

# usage: test ConditionACPower
function test01() {
    log_info "===== test01 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "/Description=/ a ConditionACPower=\"true\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    check_status base active || return 1

    # clean
    sctl stop base
    kill_sysmaster

    sed -i '/ConditionACPower=/ s#true#false#' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    check_status base inactive || return 1

    # clean
    sctl stop base
    kill_sysmaster
}

# usage: test ConditionFirstBoot
function test02() {
    log_info "===== test02 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "/Description=/ a ConditionFirstBoot=\"false\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    check_status base active || return 1

    # clean
    sctl stop base
    kill_sysmaster

    # create first-boot file
    sed -i '/ConditionFirstBoot=/ s#false#true#' ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    sctl start base
    check_status base inactive || return 1
    sctl stop base
    ls -l /run/sysmaster/first-boot
    expect_eq $? 2
    touch /run/sysmaster/first-boot
    sctl start base
    check_status base active || return 1

    # clean
    rm -rf /run/sysmaster/first-boot
    sctl stop base
    kill_sysmaster
}

# usage: test ConditionNeedsUpdate
function test03() {
    log_info "===== test03 ====="
    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "/Description=/ a ConditionNeedsUpdate=\"/etc\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

    stat /etc.updated
    stat /usr
    sctl start base
    check_status base active || return 1

    sleep 1.5
    touch /usr/aaa
    stat /etc.updated
    stat /usr
    sctl stop base
    sctl start base
    check_status base inactive || return 1
    rm -rf /usr/aaa

    # clean
    sctl stop base
    kill_sysmaster
}

# usage: test ConditionUser
function test04() {
    log_info "===== test04 ====="
    test_user_1="test1_${RANDOM}"
    test_user_2="test2_${RANDOM}"
    user_pw_1="PW!test1_${RANDOM}"
    user_pw_2="PW!test2_${RANDOM}"
    install_pkg shadow sudo
    expect_eq $? 0 || return 1
    useradd "${test_user_1}"
    useradd "${test_user_2}"

    cp -arf "${work_dir}"/tmp_units/base.service ${SYSMST_LIB_PATH} || return 1
    sed -i "/Description=/ a ConditionUser=\"${test_user_1}\"" ${SYSMST_LIB_PATH}/base.service
    run_sysmaster || return 1

#    echo "${user_pw_1}" | passwd --stdin "${test_user_1}"
#    echo "${user_pw_2}" | passwd --stdin "${test_user_2}"

    sctl start base
    check_status base inactive || return 1
    sudo -u "${test_user_1}" sctl start base; sctl status base
    expect_eq $? 0 || return 1
    sctl stop base
    sudo -u "${test_user_2}" sctl start base; sctl status base
    expect_eq $? 3 || return 1

    # clean
    sctl stop base
    userdel -rf "${test_user_1}"
    userdel -rf "${test_user_2}"
    kill_sysmaster
}

test01 || exit 1
test02 || exit 1
# ConditionNeedsUpdate not implemented yet
# test03 || exit 1
test04 || exit 1
exit "${EXPECT_FAIL}"

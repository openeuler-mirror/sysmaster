## 1. 简介

init作为用户态的1号进程，其功能包括管理sysmaster、回收僵尸进程。

## 2. 参数选项

- --timewait=value

    设置等待sysmaster连接超时时长value秒，设置保活超时的时长为value秒，默认超时时长为10秒，最短设置超时时间为10秒。

- --timecnt=value

    设置保活超时的最大次数为value次，与timewait组合使用，超过最大超时次数后认为sysmaster已失活，执行重拉起。默认最大次数为5次，最短设置超时次数为2次。

## 3. 注意事项
sysmaster二进制程序必须存放在`/usr/lib/sysmaster/`目录下。

## 4. 现状
- 一号进程是所有用户态进程的祖先，位置关键，非常容易导致整个系统奔溃。
- 原一号进程使用C语言开发，功能复杂，规模庞大，内存类问题较多。
- 进程升级时会使用execve系统调用，而此调用存在失败的风险。

## 5. 目标：使系统具有一个稳固的一号进程作为支撑点
- 一旦成功运行就不需要关注或升级。
- 在系统处于可恢复故障时提供升级/重执行通道，用于故障恢复。
- 在系统陷入不可恢复故障时提供逃生通道，用于现场收集。

## 6. 场景
- 系统启动
- 系统正常运行
- 系统可恢复异常
- 系统不可恢复异常

## 7. 整体思路
- 将服务管理功能从一号进程(init)中剥离出来以独立进程(manager)形式存在，减少init进程完成的事务范围。
- 业务进程的启动、监管由manager进程完成，init进程通过启动、监管manager进程来完备整体启动、可靠性链条。
- manager处于可恢复异常状态时，init进程维持manager升级/重执行通道。
- manager处于不可恢复状态时，init进程维持系统逃生通道。
- init中的功能只包括：监管manager进程、僵尸进程回收。

## 8. 技术方案
待补充

## 9. 环境变量
在containerd的容器环境中， 由于无方获取当前所处的容器环境， 在启动时需要指定container环境变量， 如`container=containerd`。
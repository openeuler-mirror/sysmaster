# Socket 配置

## ExecStartPre、ExecStartPost、ExecStopPre、ExecStopPost

* 类型：字符串

服务在不同的启动阶段执行的命令。配置多条命令时以`;`隔开。

**注意：** 为了避免`;`解析为命令参数，`;`作为分隔符使用时需要在前后添加空格。详情参考：[service说明文档中对应说明](./service.md)

## ListenStream、ListenDatagram、ListenSequentialPacket

配置SOCK_STREAM、SOCK_DGRAM套接子的监听地址, 支持以下的配置格式。

如果地址以“/”开头, 则创建一个UNIX套接字（AF_UNIX）。

如果地址以“@”开头, 则创建一个抽象空间的UNIX套接字（AF_UNIX）。

如果地址是一个数值类型，则会视为一个IPv6套接子的端口号， 如果不支持IPv6, 则创建一个IPv4套接子的端口号。

如果地址是“a.b.c.d:x”格式， 则绑定IPv4套接子的地址“a.b.c.d”的"x"端口。

如果地址是“[a]:x”, 则绑定IPv6套接子的地址"a"端口“x”。

SOCK_SEQPACKET只有在Unix套接子时才有效。

## ListenNetlink

监听套接子类型为netlink， 配置格式为"{name} {group ID}"。

当前支持的name为route, inet-diag, selinux, iscsi, audit, fib-lookup, netfilter, ip6-fw, dnrtmsg, kobject_ uevent、scsitransport、rdma等。

## ListenFIFO

* 类型：字符串

创建并监听用户配置的FIFO文件，详见：[fifo(7)](https://man7.org/linux/man-pages/man7/fifo.7.html)，仅允许配置为一个绝对路径。

## ListenSpecial

* 类型：字符串

监听一个特殊文件，仅允许配置为绝对路径。特殊文件指：字符设备、/proc、/sys目录下的文件。

## ReceiveBuffer 、SendBuffer

设置socket套接子的receive和send的buffer大小， 当前只支持数值型配置。

## PassPacketInfo

配置类型为true或false, 表示是否允许AF_UNIX套接子接受对端进程在辅助消息中发送证书， 设置的是IP_PKTINFO套接子选项的值， 默认为false。

## PassCredentials

配置类型为true或false, 表示是否允许AF_UNIX套接子接受对端进程在辅助消息中发送证书， 设置的是SO_PASSCRED套接子选项的值， 默认为false。

## PassSecurity

配置类型为true或false, 表示是否允许AF_UNIX套接子接受对端进程在辅助消息中发送安全上下文， 设置的是SO_PASSSEC套接子选项的值， 默认为false。

## SocketMode

设置文件的访问模式， 当为unix套接字或FIFO文件时有效， 默认值为0o666。

## Symlinks

* 类型：字符串

为`AF_UNIX`、`FIFO`类型的socket创建软链接，仅支持配置为绝对路径。允许用户配置`;`隔开的多个路径。

**注意：** 如果使用该配置，必须确保有且仅有一个`AF_UNXI`或`FIFO`的socket路径，否则socket虽然会启动，但软链接并不会创建出来。

## RemoveOnStop

* 类型：布尔值

配置在socket单元停止后是否删除建立的socket文件，以及由`Symlinks`配置生成的软链接。配置为`true`时，删除；配置为`false`时，不删除。默认为`false`。

## SocketUser、SocketGroup

设置文件所属的用户和用户组，当为unix套接子或为FIFO类型的文件时有效，默认值为空字符串，支持配置为用户名或对应的用户id。

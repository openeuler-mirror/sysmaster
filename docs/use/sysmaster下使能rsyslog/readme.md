# sysmaster下使能rsyslog

本文档介绍在`sysmaster`环境中使用`rsyslog`管理日志的方法。

## 安装流程

1. 使用`yum`工具安装`rsyslog`包：

    ```shell
    # yum install rsyslog -y
    ```

2. 执行如下命令，在`sysmaster`的配置路径下安装`rsyslog`服务：

    ```shell
    # sh ./install_rsyslog.sh
    ```

> **注意：**
>
> `install_rsyslog.sh`安装脚本会覆盖系统中已有的`rsyslog`配置文件，安装前请手动备份`/etc/rsyslog.conf`文件。
>

## 部署使用

`sysMaster`的绝大多数组件以`syslog`和`console`作为日志输出对象，但是`sysmaster`和`devmaster`例外。这两个组件需要在配置
文件中指定`syslog`为日志输出对象，具体配置方法参考各自的手册。

部署完成后，重新启动系统，从`/var/log/messages`文件中查看日志内容。

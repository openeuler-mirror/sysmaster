# devmaster使用手册

## 1. 简介

`devmaster`是用户态设备管理工具。其基于`Linux`内核的`uevent`机制，提供了设备热插拔管理的能力。`devmaster`的规则机制可以避免大部分场景下的设备热插拔处理逻辑的硬编码，规则机制提供的功能包括管理设备节点的权限、创建具有唯一标识能力的块设备软链接、网卡重命名等等，另外也支持运行用户自定义的用户态程序。`devmaster`的规则语法、客户端管理工具`devctl`以及用户态广播报文格式等对外接口保持对`udev`良好的兼容能力，使得在大部分场景下依赖`udev`提供的`sdk`、或者订阅了`udev`广播消息的用户态软件，可以在`devmaster`的运行环境中保持正常运行。

## 2. 安装部署

### 安装`devmaster`

使用`yum`安装`sysmaster-devmaster`包：

```
# yum install sysmaster-devmaster
```

### 启动`devmaster`

以后台进程启动`devmaster`，将日志导出到`/tmp/devmaster.log`文件中：

```
# /lib/devmaster/devmaster &>> /tmp/devmaster.log &
```

### 添加规则文件

在`/etc/devmaster/rules.d/`目录下创建自定义规则，通常以两位数字开头，以`.rules`作为后缀，比如`00-test.rules`，规则语法见下文。添加、删除、变更规则后，需要重新启动`devmaster`。

### 触发设备事件

使用`devctl`命令触发设备事件：

```
# devctl trigger --type subsystems --action add
# devctl trigger --type devices --action add
```

查看`/run/devmaster/data/`目录下，生成每种设备和子系统对应的数据库。

## 3. 配置

### 配置文件路径

`devmaster`拥有唯一的配置文件，路径为`/etc/devmaster/config.toml`

### 配置选项

`devmaster`配置文件中支持的配置选项如下：

- `rules_d`: 规则加载路径，默认值为`["/etc/devmaster/rules.d", "/lib/devmaster/rules.d"]`。和`udev`不同，`devmaster`无默认规则加载路径，需要在配置文件中显示指定。`devmaster`目前不支持规则加载路径优先级的功能，不同规则路径下的同名规则文件不会存在覆盖行为。规则文件的加载顺序按照`rules_d`配置项中指定的目录顺序，相同目录下按照规则文件的字典序进行加载。

- `max_workers`: 最大`worker`线程并发数，默认为3。

- `log_level`: 日志级别，支持`"trace"`，`"debug"`，`"info"`，`"warn"`，`"error"`，`"off"`，默认值为`"info"`。

- `network_d`: 网卡配置加载路径，默认值为`["/etc/devmaster/network.d"]`。网卡配置用于控制`devmaster`的内置命令`net_setup_link`的行为，具体可参考`网卡配置`手册。

## 4. 规则

### 规则文件

规则文件包含`.rules`后缀，并且通常以二位数字作为开头，比如`00-test.rules`。每个规则文件中包含若干行规则，每行规则由若干个规则表达式构成。规则表达式之间使用逗号`,`隔开，单个规则表达式的语法形式为`<key>[{attribute}]<op><value>`，其中`key`、`op`和`value`必须声明，而`{attribute}`只在部分规则类型下需要声明。`key`为规则表达式的类型，由大写英文字母组成，目前包含24类规则，部分规则携带的`attribute`和`key`组合成特定的键。`op`为操作符，分成匹配操作符和赋值操作类。根据操作符类型可分为匹配类规则或赋值类规则，匹配类规则表达式的结果为真时，则会执行当前行中后续的规则表达式，否则跳过该行，执行下一行规则。`value`为规则值，被双引号包括，双引号内部不允许嵌套使用双引号。在匹配类规则表达式中，键和值根据操作符类型进行匹配，根据匹配结果返回真或假，在赋值类规则中，值会被赋予给键。规则文件中的注释以`#`开头，规则加载时会跳过。 **`devmaster`的规则文件加载机制和`udev`存在差异，细节可参考`devmaster`配置文件中的`rules_d`配置项的说明。`devmaster`的规则语法友好兼容`udev`，部分差异之处已在本手册中标注。**

### 操作符

规则操作符分为匹配类和赋值类，根据操作符种类，规则表达式分为匹配类规则和赋值类规则。

匹配操作符包括：

- `==`：当规则表达式的键和值相同时，结果为真。
- `!=`：当规则表达式的键和值不同时，结果为真。

赋值操作符包括：

- `=`：将规则表达式的值赋予给键，键原来的值会被覆盖。
- `+=`：将规则表达式的值附加给键。
- `-=`：从键中移除某个值。
- `:=`：将规则表达式的值赋予给键，键原来的值会被覆盖，并且后续规则中对该键的赋值操作不再生效。

### 规则值

规则值由双引号包括起来。 **`devmaster`的规则语法和`udev`存在一些差异，规则值不支持在双引号内部嵌套使用双引号，且规则值中不支持转译符解析，使用过程中如果需要在内部使用引号，则需要用单引号代替。另外`devmaster`也无法使用`udev`规则语法中形如`e"string\n"`的转译符声明。**

### 匹配规则

匹配类规则会检查键和值是否满足匹配条件，再根据匹配操作符选择返回真或假。如果为真则继续执行本行规则中的后续表达式，否则跳到下一行规则继续执行。匹配类规则表达式的值匹配支持纯文本匹配和`shell glob`模式匹配。纯文本匹配需要满足键和值完全相同。`shell glob`模式匹配支持的语法如下：

- `*`：匹配0或多个任意字符。
- `?`：匹配一个任意字符。

- `[]`：匹配括号中的任意一个字符，比如`[ab]`可以匹配单个`a`或`b`，`[0-9]`匹配任意个位数字。如果中括号中的内容以`!`开头，则匹配不属于中括号中内容的单个字符，比如`[!a]`表示匹配任意一个非`a`字符。
- `|`：分隔子匹配模式，比如`a|b`可以匹配`a`或`b`。

匹配类规则的键如下：

- `ACTION`：匹配设备事件的动作类型。
- `DEVPATH`：匹配设备的`devpath`。
- `KERNEL`：匹配设备名`sysname`。
- `NAME`：匹配前文规则中通过`NAME`赋值规则设置的网卡名，初始状态下为空。
- `SYMLINK`：匹配前文规则中赋予该设备的软链接。同一个设备可以同时拥有多个软链接，如果匹配操作符为`==`，只要有一个软链接满足匹配则规则表达式返回真。如果匹配操作符为`!=`，则所有软链接都不匹配是返回真。
- `SUBSYSTEM`：匹配设备的子系统类型。
- `DRIVER`：匹配设备绑定的驱动名。如果设备上报时，该设备未绑定驱动，则键为空。
- `ATTR{filename}`：匹配设备的`attribute`属性。**`devmaster`读取`sysfs`属性时会抛弃首尾空格，和`udev`存在细微差异。**
- `ENV{key}`：匹配设备的某个`property`属性，包括从`uevent`文件中导入的属性以及在规则前文中赋予的属性。
- `TAG`：匹配前文规则中赋予设备的某个标签。如果操作符为`==`，只要存在某个满足匹配的标签，规则表达式就返回真。如果操作符为`!=`，仅当所有标签都不满足匹配，表达式才返回真。
- `TEST[{octal mode mask}]`：检查某个文件是否存在。规则表达式的值可以是文件的绝对路径、相对路径或者`[subsystem/kernel]/attribute`。如果表达式的值为相对路径，则在当前设备的`syspath`下搜索文件。路径中可以使用`*`来替代某个中间目录，比如`a/*/b`，`devmaster`会搜索`a`目录下的所有子目录，如果某个子目录中存在文件`b`，则满足匹配。另外可以指定八进制的访问权限掩码，如果文件存在并且文件权限和掩码的位与结果大于0，则满足匹配。
- `PROGRAM`：立即执行规则表达式值中声明的命令，并获取返回值。
  - 如果返回值为0，则满足匹配，如果返回值不为0，则不满足匹配。如果指定的命令非法，或者执行的命令运行超时，会导致当前设备事件处理失败。如果命令执行成功，其标准输出的内容会保留在内存中，后续可以使用`RESULT`规则匹配输出内容，或者使用`$result`占位符获取输出内容，直到执行下一条`PROGRAM`规则对其进行覆盖。标准错误流的内容则会记录在`devmaster`的日志中。
  - `PROGRAM`规则仅允许使用匹配类操作符，`=`、`:=`和`+=`赋值操作符会自动转换成`==`匹配操作符。 **目前`devmaster`未提供命令超时时延的设置界面，默认为3秒。**
  - 如果外部命令不是绝对路径，会依次在`/lib/udev/`和`/lib/devmaster/`路径下查找， **目前未提供指定搜索路径的设置界面。**

- `RESULT`：匹配最近一次`PROGRAM`规则中执行命令的标准输出结果。
- `IMPORT{type}`：导入`property`属性到设备中。`type`包括：
  - `program`：调用外部命令，如果返回值为0，则从标准输出中获取满足`key=value`格式的文本行，作为`property`属性导入到设备中。`IMPORT{program}`规则在执行是会立即执行外部命令，约束见`PROGRAM`或`RUN{program}`规则。
  - `builtin`：调用内置命令。
  - `file`：读取一个文本文件，从满足`key=value`格式的文本行中获取`property`属性并导入到设备中。
  - `db`：从老的数据库文件中获取`property`属性，并导入到内存中。这些`property`属性会持久化保存到新的数据库中。
  - `cmdline`：从内核的命令行参数中获取属性，并作为`property`属性导入到设备中。如果命令行参数中为指定属性值，则默认为`1`。
  - `parent`：从直接父设备的数据库中获取`property`属性，并导入到当前设备中。
  -  **`IMPORT`规则的操作符必须是匹配类操作符，赋值类操作符会默认转换为`==`。**
- `CONST{key}`：匹配系统级别的常量，支持的`key`包括：
  - `arch`：匹配系统架构。

  - `virt`：检测系统是否处于虚拟化环境。

  -  **`devmaster`尚不支持该规则。**
- `SYSCTL{kernel parameter}`：匹配内核参数。 **`devmaster`尚不支持该规则。**

另外部分匹配类规则会依次匹配当前设备及其父设备，直到找到一个满足匹配的设备。此类匹配规则亦称为`父类匹配规则`。同一条规则行中可以连续声明多条父类匹配规则，仅当某个设备在所有父类匹配规则上均返回真，才会继续执行后续规则，否则获取上一级父设备，并重复执行所有父类匹配规则。 **同一规则行中的多个父类匹配规则需连续声明，如果被非父类匹配规则截断，后续父类匹配规则不会生效。**

- `KERNELS`：匹配当前设备或者父设备的`sysname`。
- `SUBSYSTEMS`：匹配当前设备或者父设备的子系统。
- `DRIVERS`：匹配当前设备或者父设备的绑定驱动。
- `ATTRS{filename}`：匹配当前设备或者父设备的名为`filename`的`attribute`属性。
- `TAGS`：匹配当前设备或者父设备的标签。

### 赋值规则

赋值规则会调整`devmaster`内存中的设备数据，将表达式的值赋予给键。赋值规则的键如下：

- `NAME`：`NAME`赋值规则仅对网卡事件生效，用于设置网卡名。非网卡设备，比如块设备等，无法更改设备节点名。
- `SYMLINK`：添加设备节点的软链接。
  - 软链接名仅支持`合法字符`，非法字符会替换为`_`字符。
  - 一般情况下，允许在规则表达式的值中指定多个软链接，使用空格分隔。如果本行规则中，`SYMLINK`赋值规则前执行了`OPTIONS+="string_escape=replace"`赋值规则，则`SYMLINK`规则值视为单个软链接，空格会转换为下划线`_`。
  - `devmaster`允许多个设备声明同名的软链接，但是软链接会指向链接优先级`link_priority`最高的设备。当链接优先级最高的设备被移除时，会从剩下的设备中选择优先级次高的设备，生成对应的软链接。软链接优先级通过`OPTIONS+="link_priority=value"`规则设置。如果存在多个设备拥有同名软链接，且链接优先级相同，则（伪）随机选择一个设备生成对应的软链接， **此时`devmaster`不保证和`udev`的行为完全一致。**
  - 设备节点的软链接应当避免和现有设备节点同名，否则会出现无法预测的结果。
  -  **`SYMLINK`赋值规则只支持`+=`、`=`和`:=`操作符，不支持使用`-=`操作符移除特定软链接。**
- `OWNER`、`GROUP`、`MODE`：设置设备节点的用户、用户组和权限。
- `SECLABEL{module}`：在设备节点上应用指定的`Linux`安全模块标签。 **`devmaster`尚不支持该规则。**
- `ATTR{key}`：将规则值写入`attribute`属性对应的`sysfs`文件中。
- `SYSCTL{kernel parameter}`：将规则值写入内核参数中。
- `ENV{key}`：设置设备的`property`属性。如果属性值以`.`开头，则该`property`属性不会被持久化记录到数据库中或者以环境变量的形式导出到调用外部命令执行的子进程中。如果前置规则表达式中执行了`OPTIONS+="string_escape=replace"`规则，则属性值中的非法字符和空格会替换为下划线，否则直接使用原来的属性值。
- `TAG`：给设备附加标签。设备标签可以用于为用户态订阅了`devmaster`广播消息或枚举系统设备时对冗余设备进行过滤。标签过滤算法的复杂度为`O(n)`，因此适用于标签数量较少的场景，标签较多时会影响性能。
- `RUN[{type}]`：指定将要运行的命令。这些命令会在规则执行结束后统一执行。规则表达式值中不允许嵌套使用双引号，如果需要为命令附加参数，应当使用单引号代替。命令的类型分为两类：
  - `program`：表示执行的命令为外部程序，如果命令未指定绝对路径，则依次在`/lib/udev`和`/lib/devmaster`中查找。为显示指定`type`时，默认为`program`类型命令。
  - `builtin`：表示执行的命令为内置命令。
  - 执行的外部程序运行超过3秒后会超时失败，因此无法通过该规则启动后台进程或者执行耗时较长的命令， **`devmaster`目前未提供超时设置界面。**
  - **另外`sysmaster`当前未支持`device`类型的`unit`，`devmaster`无法通过给设备添加特殊的属性，比如`SYSTEMD_WANTS`，来激活`sysmaster`服务。这一点和`udev`存在差异。**

- `OPRIONTS`：设置规则或设备选项。选项类型包括：
  - `link_priority=value`：设置当前设备的软链接优先级，如果存在多个设备声明了同名的软链接，则实际生成的软链接指向优先级最高的设备。默认值为0，值越大表示软链接优先级越高。
  - `string_escape=none|replace`：设置规则表达式的值转译模式，可以作用在`NAME`、`ENV`和`SYMLINK`赋值规则上。如果转译模式为`none`，保留原来的规则表达式值内容。如果转译模式为`replace`，值中的非法字符会替换为下划线。
  - `static_node=<sysname>`：将本行中的`OWNER`、`GROUP`和`MODE`选项作用在设备节点`sysname`上，启动`devmaster`后立即生效。 **`devmaster`尚不支持静态标签功能，和`udev`存在差异。**
  - `watch`：使用`inofity`机制监听设备节点。如果设备节点以可写方式打开并关闭后，同步触发该设备的`change`事件。 **`devmaster`尚不支持该功能。**
  - `nowatch`：关闭对该设备节点的`inotify`监听。 **`devmaster`尚不支持该功能。**
  - `db_persist`：设置`devmaster`数据库的持久性，令执行`devctl info --clenup-db`后能保留该设备的数据库内容。 **`devmaster`尚不支持该功能。**
  - `log_level`：设置当前设备事件处理流程的日志级别。 **`devmaster`不支持该功能。**

- `LABEL`：设置跳转位置。
- `GOTO`：跳转到指定的跳转位置，执行后续规则。

## 4. 占位符

在一些规则表达式的值中，可以使用占位符进行字符替换。规则执行过程中，会使用设备的运行时信息替换占位符内容。允许使用占位符的规则包括：`TEST`、`IMPORT{file}`、`IMPORT{program}`、`IMPORT{builtin}`、`IMPORT{parent}`、`PROGRAM`、`OWNER`、`GROUP`、`MODE`、`ENV`、`TAG`、`NAME`、`SYMLINK`、`ATTR`、`RUN`。

合法的占位符类型包括：

- `$kernel, %k`：设备的`sysname`。
- `$number, %n`：设备的内核号。比如，`sda3`的内核号为`3`。
- `$devpath, %p`：设备的`devpath`路径。
- `$id, %b`：设备的`sysname`。
- `$driver`：根据前文规则中通过父类匹配规则找到的父设备，获取该父设备绑定的驱动名。
- `$attr{file}, %s{file}`：如果`file`的格式为`[subsystem/kernel]/attribute`，则使用扩展得到的`attribute`属性。否则获取设备的名为`file`的`attribute`属性，如果当前设备无该`attribute`属性，则查找前文规则中通过父类匹配规则找到的父设备的`attribute`属性。
- `$env{key}, %E{key}`：替换为设备的`property`属性。
- `$major, %M`：主设备号。
- `$minor, %m`：次设备号。
- `$result, %c`：最近一次通过`PROGRAM`规则获取的外部命令输出的文本内容。如果输出内容由空格分隔，可以使用`$result{N}, %c{N}`获取第`N`部分文本，使用`$result{N+}, %c{N+}`获取第`N`部分以及后续所有部分文本。
- `$parent, %P`：直接父设备的`sysname`。
- `$name`：如果是网卡设备，且网卡设备名在前文中通过`NAME`赋值规则改变，则使用改变后的网卡名。否则使用设备的`sysname`。
- `$links`：使用空格连接设备的所有软链接得到的字符串。
- `$root, %r`：扩展为`/dev`。
- `$sys, %S`：扩展为`/sys`。
- `$devnode, %N`：扩展为设备的`devname`。
- `%%`：扩展为`%`字符。
- `$$`：扩展为`$`字符。

## 术语表

| 名词    | 含义 |
| ------- | ---- |
|sysfs|呈现内核设备树信息的伪文件系统，通常挂载在`/sys`目录下。某某设备的`sysfs`路径通常指代其在`/sys/devices`下的路径，也称为`syspath`。|
| syspath | 设备在`sysfs`下的绝对路径，比如`/sys/devices/virtual/net/lo`。 |
|devpath|`syspath`去除`/sys`前缀。|
|sysname|`syspath`的内核名称，比如回环网卡设备`/sys/devices/virtual/net/lo`的`sysname`为`lo`。|
|devname|设备节点在`devtmpfs`下的路径名，比如`/dev/sda`。|
|action|设备事件的动作类型，包括`add`、`remove`、`change`、`move`、`bind`、`unbind`、`online`和`offline`。|
|diskseq|磁盘序号。|
|ifindex|网卡序号。|
|devtype|设备类型，比如整盘块设备的设备类型是`disk`，分区块设备的设备类型是`partition`。|
|attribute|特指设备的`sysfs`属性，和`property`不同，`attribute`为内核设备树下存在的文件，且不会持久化保存到数据库中。另外在某些形如`key{attribute}`的规则键中，有时也会使用`attribute`来表示花括号中的内容。|
|property|特指设备从`uevent`中读取的属性以及规则中导入的属性。规则中导入的属性会被持久化记录在数据库中，而从`uevent`中导入的属性则不会。|
|[subsystem/kernel]/attribute|扩展为`/sys/class/\<subsystem\>/\<kernel\>/\<attribute\>`路径。|
|合法字符|`devmaster`支持的合法字符包括`0-9A-Za-z#+-.:=@_/`、合法的`UTF-8`字符和`\x00`十六进制编码。|
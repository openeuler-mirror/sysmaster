大纲--->详细描述unit框架所需各类unit实现的能力，需要时简要描述关联unit框架实现原理。

一、UnitSubClass实现+插件注册
1、运行环境说明
当前process1采用单线程运行环境，所以各process1管理单元（unit）在实现时需要以异步无阻塞方式编程。
2、实现子类操作对象管理
unit作为所管理的系统资源在process1中的投影，unit在通过os接口操作对应系统资源时，同时维护对应系统资源的状态和历史操作结果。
（1）系统资源管理
根据业务诉求提炼出其所需管理的系统资源，并通过os接口对这些系统资源进行管理，对外提供系统资源的操作接口，包括：start、stop、reload。
（2）通用场景【必选】
a、状态管理
unit框架基于各类unit状态提供了诸多功能，并根据系统资源操作方式制定了标准unit状态组，要求各类unit提供其状态获取接口和状态变更通告。
状态获取：各类unit内部可以根据自身实现来制定其内部状态组，但在框架向各类unit获取状态时需要进行内部状态组到标准unit状态组的装换(current_active_state)。
状态通告：各类unit在其状态发生变化时需要将状态变化信息通告给unit框架，为方便各类unit实现，unit框架提供的状态变更通告接口支持重入。
【编码示例】
（3）典型场景【可选】
a、依赖关系
不同unit间存在依赖关系时，除了在unit配置文件中明确配置外，常见的一种场景是需要根据运行环境对其他相关unit进行默认依赖，这样可以减小维护人员的配置压力。为此，unit框架提供了一些接口供各类unit动态自主添加unit依赖关系，以支持各类unit在此场景中的开发。
添加unit依赖关系：支持在不同unit间添加指定依赖关系。
【编码示例】
b、进程管理
各类unit在管理对应系统资源时，常见的一种场景是需要在一个独立进程环境中完成对应系统资源的操作，这样可以增加系统的健壮性。为此，unit框架提供了一些公共能力来进行（process1进程之外）独立进程的操作，以方便各类unit在此场景中的开发。
进程通信：支持各类unit通过系统调用kill向指定进程组发送各种信号。
进程id跟踪：在各类unit提供pid-unit对应关系数据的情况下，支持根据pid对应关系进行sigchld信号分发(sigchld_events)。
进程拉起：支持各类unit通过系统调用fork+execve拉起子进程（含cgroup设置），同时支持多种运行环境参数设置。
【编码示例】

3、实现子类配置文件解析
定义私有配置段名称(get_private_conf_section_name)、定义配置项字段、实现配置项get/set[可选]函数、实现对外操作接口(load)。
待补充？
【编码示例】

4、插件注册
各类unit以插件方式集成到process1中，unit框架提供了插件注册能力，以支持各类unit完成注册。
插件注册：在各类unit提供分配带默认值实例的情况下，支持根据类型名称注册指定类型的unit子类插件。
【编码示例】

5、增加附加功能
增加配置项：同步骤3
增加处理逻辑：围绕对象管理逻辑增加新增附加功能，在不实质影响对象管理逻辑的条件下，尽可能将新增附加功能以独立模块方式呈现。

二、参考子模块依赖关系
// xxx_base -> {xxx_comm | xxx_config}
// {xxx_mng_util | xxx}
// {xxx_mng | xxx_load} ->
// {xxx} -> xxx_unit
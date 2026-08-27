# PetAgent 插件开发文档

这组文档面向在 `E:\projectdata\PetAgent-plugin` 中开发、测试和发布插件的作者。

## 先读哪一篇

1. [开发教程](./01-开发教程.md)：从目录初始化到本地安装的完整流程。
2. [架构与信任边界](./02-架构与信任边界.md)：理解插件由谁加载、能访问什么，以及哪些检查不是安全隔离。
3. [Manifest 参考](./03-Manifest参考.md)：逐字段说明 `manifest.template.json`。
4. [运行时协议](./04-运行时协议.md)：MCP、sidecar/provider、native-dll 和 frontend 的通信契约。
5. [测试与调试](./05-测试与调试.md)：契约测试、宿主日志和常见错误定位。
6. [打包与市场发布](./06-打包与市场发布.md)：校验、打包、生成市场索引和 Release。

## 仓库边界

- **插件源码、示例和市场元数据**：`E:\projectdata\PetAgent-plugin`。
- **宿主加载器、Cordis service seam、ABI 和前端协议**：`E:\projectdata\PetAgent`。
- 插件通过 `trait + service name + event/protocol` 与宿主对接；不要在插件中复制宿主内部实现。

## 当前信任模型

PetAgent 将已安装插件视为使用者明确选择的可信代码：没有沙箱、签名验证或严格权限 ACL。插件进程和宿主共享用户可用的文件、网络、进程及系统 API 权限。manifest 的 `permissions` 仅用于说明和市场展示；`sha256` 可记录完整性信息，但不作信任决策。

因此，发布者必须提供可审计源码、版本说明和变更记录；使用者只应安装信任来源的包。宿主仍会检查版本、依赖、冲突、目标架构、相对路径、ZIP 路径穿越、大小、入口文件、原生 ABI、初始化和 health，这些是可装配性与回滚检查，不是沙箱。

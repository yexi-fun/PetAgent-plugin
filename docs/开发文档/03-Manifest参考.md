# Manifest 参考

规范文件：[`docs/manifest.schema.json`](../manifest.schema.json)。模板文件命名为 `manifest.template.json`，打包时复制并写入最终 `manifest.json`。

## 顶层字段

| 字段 | 必填 | 说明 |
|---|---:|---|
| `schemaVersion` | 是 | 当前固定为 `1`。 |
| `id` | 是 | 小写插件 ID，匹配 `^[a-z0-9]+(?:[.-][a-z0-9]+)+$`，最多 128 字符。 |
| `name` | 是 | UI 名称，1–80 字符。 |
| `version` | 是 | SemVer，例如 `0.1.0`。 |
| `apiVersion` | 是 | 当前固定为 `1`。 |
| `petAgent` | 是 | 宿主版本范围，例如 `>=0.1.0 <0.2.0`。 |
| `type` | 是 | `mcp`、`sidecar`、`provider`、`native-dll`、`frontend` 或 `app`。 |
| `description` | 是 | 1–500 字符的功能说明。 |
| `entry` | 是 | 入口对象，`entry.kind` 必须与 `type` 相同。 |
| `dependencies` | 否 | `{ "id": "...", "version": "..." }` 数组。缺失依赖会拒绝安装。 |
| `conflicts` | 否 | 互斥插件 ID 数组。 |
| `permissions` | 否 | `network`、`filesystem-read`、`filesystem-write`、`shell`、`model`、`window`、`microphone` 的描述标签。不是 ACL。 |
| `targets` | 是 | `windows-x64` 或 `windows-arm64`，至少一个。 |
| `configSchema` | 否 | 包内 JSON Schema 相对路径。 |
| `sha256` | 否 | 64 位十六进制完整性元数据，不参与信任决策。 |
| `signature` | 否 | 旧版本兼容字段，当前不验证。 |
| `signingKeyId` | 否 | 旧版本兼容字段，当前不验证。 |

## entry 字段

`entry` 至少包含 `kind`，并且只能使用 Schema 声明的字段：

| 类型 | 必填入口字段 |
|---|---|
| `mcp` | `config`，指向 MCP JSON 配置。 |
| `sidecar` | `executable`，指向包内可执行文件。 |
| `provider` | `executable`、`serviceName`、`serviceApiVersion`。当前 service name 为 `llm`、`LlmService` 或 `pet.llm`。 |
| `native-dll` | `library`、`abiVersion: 1`、`serviceName: "tools"`、`serviceApiVersion: 1`。 |
| `frontend` | `frontend.root` 与 `frontend.index`，且 index 必须位于 root 内。 |

frontend 还可选声明 `protocolVersion`（当前为 `1`）、`capabilities`（`host-info`、`config.read`、`config.write`、`window.close`、`window.state`、`window.ready`、`notifications`、`lifecycle-events`）和受宿主边界限制的 `window` 尺寸参数。能力白名单决定可调用的 frontend RPC；`permissions` 仍是可信代码模型下的说明标签，不是沙箱 ACL。

所有路径必须是相对路径，不能包含 `..`、反斜杠、盘符或前导 `/`。运行时还会检查解析后的路径没有逃出插件版本目录。

`app` 必须同时声明 `entry.service` 与 `entry.frontend`。service 使用包内相对 `.exe`、`protocolVersion: 1` 和 `startup: on-demand|enabled`；frontend 可声明 `window.placement: pet-relative` 或 `pet-top-center` 与偏移量。`pet-top-center` 由宿主读取桌宠和 app 窗口实际外部尺寸，计算桌宠正上方的水平居中位置，并在桌宠移动或缩放时自动跟随。顶层 `agent.capabilities` 是 service 能力交集的工具注册候选。

## APP 窗口位置

APP 窗口的位置由宿主控制。插件只能在 `entry.frontend.window` 中声明受支持的锚点和偏移量，不能直接调用 Tauri 窗口 API 或修改其他宿主窗口的位置。

### `pet-relative`

兼容定位方式。宿主以桌宠渲染窗口的左上角和宽度为基准，将 APP 窗口放在桌宠右侧：

```text
x = pet.x + pet.width + offsetX
y = pet.y + offsetY
```

`offsetX` 为正值向右，`offsetY` 为正值向下。例如：

```json
"window": {
  "width": 220,
  "height": 76,
  "placement": "pet-relative",
  "offsetX": 12,
  "offsetY": 0
}
```

### `pet-top-center`

桌宠正上方居中定位。宿主读取桌宠和 APP 窗口的实际外部尺寸（含窗口边界）后计算：

```text
x = pet.x + (pet.width - app.width) / 2 + offsetX
y = pet.y - app.height + offsetY
```

`offsetX` 为正值向右，`offsetY` 为正值向下。要在桌宠上方保留 8px 间距，可使用 `offsetY: -8`：

```json
"window": {
  "width": 220,
  "height": 76,
  "placement": "pet-top-center",
  "offsetX": 0,
  "offsetY": -8
}
```

当桌宠渲染窗口发生移动或缩放时，宿主会自动重新计算并同步所有声明 `pet-top-center` 的 APP 窗口。跟随是事件驱动的，不需要插件轮询；窗口或桌宠几何信息暂不可用时，宿主保留当前窗口位置并等待下一次事件。关闭、停用、升级和卸载时，宿主负责清理 APP 窗口。

APP 窗口创建后默认隐藏。页面完成自身初始化（例如订阅事件、加载首屏数据）后，应调用 `window.ready`；宿主只显示发起该 RPC 的 APP session。未声明 `window.ready` 或未调用该方法的 APP 窗口会保持隐藏，避免页面加载期间出现白色或空白闪屏。

由于 APP 窗口通常使用无边框模式且不会显示任务栏按钮，frontend 应提供清晰的窗口内关闭按钮，并调用 `window.close` 关闭当前窗口。该操作只关闭窗口，不会停用 APP service；再次由 Agent 或宿主触发打开时可以复用同一 service。

`placement` 省略时不保证相对桌宠定位。`pet-relative` 和 `pet-top-center` 仅适用于 `type: app`，其他插件类型使用这些值会被 manifest 校验拒绝。

## 配置隔离

宿主的 `pet.yml` 使用 `pluginConfig` 按插件 ID 隔离配置：

```yaml
pluginConfig:
  com.acme.weather:
    endpoint: https://api.example.com
    timeoutMs: 5000
```

插件不得要求把自定义键写入顶层配置。sidecar/provider 的 `initialize.configuration` 就是对应插件的 JSON 值；没有配置时收到 `null`。

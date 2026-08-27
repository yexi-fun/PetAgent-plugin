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
| `type` | 是 | `mcp`、`sidecar`、`provider`、`native-dll` 或 `frontend`。 |
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

所有路径必须是相对路径，不能包含 `..`、反斜杠、盘符或前导 `/`。运行时还会检查解析后的路径没有逃出插件版本目录。

## 配置隔离

宿主的 `pet.yml` 使用 `pluginConfig` 按插件 ID 隔离配置：

```yaml
pluginConfig:
  com.acme.weather:
    endpoint: https://api.example.com
    timeoutMs: 5000
```

插件不得要求把自定义键写入顶层配置。sidecar/provider 的 `initialize.configuration` 就是对应插件的 JSON 值；没有配置时收到 `null`。

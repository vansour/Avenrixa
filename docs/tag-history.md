# Tag History

本文档用于说明 `Avenrixa` 改名前后的历史 tag、发布标题和镜像仓库对应关系。

## 基本原则

- 历史 Git tag 不改名，继续保留原始版本号，例如 `v0.1.1`、`v0.1.2-rc.1`。
- 项目当前正式品牌名统一为 `Avenrixa`。
- 改名前已发布的镜像仍保留在历史 GHCR 仓库 `ghcr.io/vansour/vansour-image`，不伪造为新仓库。
- 改名后的新发布流程默认使用 `ghcr.io/vansour/avenrixa`。

## 历史 Tag 对应

| Git tag | 当前展示名 | 历史镜像仓库 | 说明 |
| --- | --- | --- | --- |
| `v0.1.0` | `Avenrixa 0.1.0` | `ghcr.io/vansour/vansour-image:0.1.0` | 历史正式版 tag，品牌已统一，但镜像路径保留原发布事实。 |
| `v0.1.1-rc.1` | `Avenrixa 0.1.1-rc.1` | `ghcr.io/vansour/vansour-image:0.1.1-rc.1` | 历史 RC tag。 |
| `v0.1.1` | `Avenrixa 0.1.1` | `ghcr.io/vansour/vansour-image:0.1.1` | 当前仓库可见的历史正式版 Release。 |
| `v0.1.2-rc.1` | `Avenrixa 0.1.2-rc.1` | `ghcr.io/vansour/vansour-image:0.1.2-rc.1` | 改名前最后一批 RC 资产之一。 |

## 仓库内样例资产

- `dist/release/*` 下保留的是发布样例资产。
- 这些样例资产的文件名会统一到 `Avenrixa` 品牌，例如 `avenrixa-<version>-release-bundle.tar.gz`。
- 若样例资产内部记录的是历史镜像引用或历史 OCI label，会保留原始发布事实，避免篡改溯源信息。

## GitHub Release 标题

- 现有 GitHub Release 标题统一使用 `Avenrixa <version>`。
- Release 正文中的历史镜像引用继续保留旧 GHCR 路径，并明确标注为历史发布路径。

# Converter 设计笔记

> 此文件记录 LLM 游戏 UI 工作流中 Converter A/B 的设计决策，供迁移到新仓库时使用。
> 完整方案文档：`docs/LLM_GAME_UI_WORKFLOW.md`

## 概述

两个 Converter 解决 LLM 与游戏引擎资产之间两个方向的问题：

- **Converter A**（LLM 输出 → 引擎资产）：LLM 生成 HTML/CSS → 确定性转换为引擎资产骨架
- **Converter B**（引擎资产 → LLM 输入）：引擎资产太大太噪 → 提取结构化摘要给 LLM 阅读写代码

## Converter A 设计

### 技术选型
- **语言**：Rust（用户偏好，在公司推广 Rust）
- **HTML 解析**：html5ever
- **CSS 解析**：stylo 的 selectors crate（来自 servo/stylo: https://github.com/servo/stylo）
- **输出**：每个引擎一个 backend

### 架构：前后端分离

```
HTML/CSS → html5ever + stylo 解析 → UiNode 中间树（引擎无关）→ Backend trait → 引擎资产
```

核心数据结构：
```rust
struct UiNode {
    name: String,
    widget: WidgetKind,          // Button, Text, Image, ScrollView...
    children: Vec<UiNode>,
    attrs: HashMap<String, String>,  // data-* 属性、文本内容等
    css: CssProperties,          // 布局增强时用，最简版可为空
}

trait Backend {
    fn emit(&self, root: &UiNode) -> Result<Vec<u8>>;
}
```

### 分级策略

| Level | 输出内容 | 说明 |
|-------|---------|------|
| 0 | 结构 + 类型 + 命名 | 最简版，程序员可立即写代码 |
| 1 | + 尺寸信息 | |
| 2 | + 布局方向（横排/竖排）| |
| 3 | + 对齐、间距 | |
| 4 | + 锚点、百分比适配 | 接近可用的布局骨架 |

先做 Level 0，根据实际反馈逐级增强。

### HTML → 引擎组件映射表

```
div[flex-row]    →  Layout(HORIZONTAL)
div[flex-col]    →  Layout(VERTICAL)
button           →  Button
img              →  ImageView / Sprite
span / p         →  Text / Label
div[overflow]    →  ScrollView
ul / ol          →  ListView
input            →  TextField
```

### Backend 配置化

映射表可做成 TOML 配置，适配新引擎可能只需配置文件：
```toml
[widgets]
Button = "UIButton"
Text = "UILabel"
Image = "UISprite"
ScrollView = "UIScrollPanel"
```

序列化部分（各引擎资产格式的 XML/JSON/YAML 生成）差异大，每个 backend 需写代码。

### 多引擎后端
- 优先：Cocos2d-x (.csd)——覆盖面最广（多个 in-house 引擎嵌入 cocos 做 UI）
- 按需：Unreal、其他 in-house 引擎
- Converter A 始终全量、无状态、单向，增量插入由工程师在编辑器完成

### 关于 MCP 替代方案的结论
- MCP 操作编辑器逐步创建 UI：串行慢、缺乏全局视野、成本高（token 消耗可能 10x）
- 精心设计批量 MCP → 本质上就是自定义 DSL 藏在调用参数里，绕回 Converter 方案
- MCP 适合微调场景，不适合作为 UI 创建主路径
- LLM 一次性生成复杂 HTML 已有大量验证，MCP 逐步拼复杂 UI 未经验证

## Converter B 设计

### 提取的信息
- 节点树（层级关系）
- 组件类型（引擎原生类型名）
- 节点命名
- 数据绑定声明
- 事件声明

### 丢弃的信息
- 坐标、尺寸、锚点等布局细节
- 颜色、字号、字体等视觉属性
- 资源引用路径
- 引擎内部 ID / Tag

### 输出格式
JSON，直接作为 LLM 写 UI 业务代码的输入。

## 完整工作流

```
设计师 Figma 标注图 + 交互说明
  ↓ 多模态 LLM
HTML/CSS
  ↓ Converter A
引擎资产骨架
  ↓ 拼接工程师精调视觉
完整资产
  ↓ Converter B
简化描述（结构 + 类型 + 命名）
  ↓ LLM + 业务接口定义
UI 业务代码
```

### 关键设计决策
- **用 HTML/CSS 作为 IR**：LLM 零学习成本，自带浏览器预览，不自创 DSL
- **结构定义前移**：LLM 生成 HTML 时同时接收业务接口，结构兼顾视觉和代码需求
- **最简版即有价值**：结构/类型/命名确定后，程序员和拼接工程师可并行工作
- **零运行时改造**：不改引擎 UI 系统，只改资产生成方式

### 增量开发
- 现有资产 → Converter B → 简化结构（作为上下文）→ LLM 生成 HTML 片段 → Converter A → 资产子树 → 工程师手动挂入
- Converter A 始终全量无状态，不做增量插入

### 前提条件
- UI 层和业务逻辑层干净分离
- 需要明确的接口层：数据源、动作、事件
- 

## 实施优先级
1. Converter B 先行（对存量项目即时有价值）
2. Converter A 最简版（Cocos .csd backend）
3. Converter A 布局增强
4. LLM Prompt 优化

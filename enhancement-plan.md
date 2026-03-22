# Converter A 增强计划

> 基于 2026-03-22 的设计讨论，覆盖两个方向：补齐 CSS 属性解析 + 组件化分层。

## 一、补齐 CSS 通用属性解析

### 背景

当前 `style.rs` 只解析布局相关的 CSS 属性（尺寸、定位、flex、margin/padding/gap）。
Cocos 节点的多个通用属性在 `emit_base_properties` 中被硬编码（Scale 固定 1.0、CColor 固定白色、无 Rotation/Opacity/Visible 等）。

这些属性在 CSS 中有无歧义的对应，且与引擎无关——所有游戏 UI 引擎都有 Scale/Rotation/Opacity/Visibility/ZOrder。属于 HTML+CSS 基础层，应该做厚。

### 需要新增解析的 CSS 属性

| CSS 属性 | 目标字段 | Cocos 输出 | 说明 |
|---|---|---|---|
| `transform: scale(x, y)` | `scale_x: Option<f32>`, `scale_y: Option<f32>` | `<Scale ScaleX="x" ScaleY="y"/>` | 负值可推断 FlipX/FlipY |
| `transform: rotate(deg)` | `rotation: Option<f32>` | `RotationSkewX`, `RotationSkewY` | Cocos 用角度制，CSS 也是 |
| `opacity: 0-1` | `opacity: Option<f32>` | `Alpha="0-255"` 在 CColor 中 | 需 0-1 → 0-255 映射 |
| `visibility: hidden/visible` | `visible: Option<bool>` | `VisibleForFrame="False"` | — |
| `z-index: N` | `z_order: Option<i32>` | `ZOrder` 属性 | — |
| `color: #RRGGBB` | `color: Option<(u8,u8,u8)>` | `<CColor R G B/>` | 文本前景色 |
| `background-color: #RRGGBB` | `background_color: Option<(u8,u8,u8,u8)>` | Panel 的 `SingleColor` + `BackColorAlpha` | 仅容器适用 |

### 关于 transform 解析

CSS `transform` 是复合属性，可包含多个函数：`transform: scale(1.5) rotate(15deg)`。
建议实现一个简单的 transform 函数解析器，支持 `scale()`、`scaleX()`、`scaleY()`、`rotate()`，忽略其他函数（translateX/Y 不需要，定位用 left/top）。

### 修改范围

1. **`src/types.rs`** — `LayoutProps` 新增字段：
   ```rust
   pub scale_x: Option<f32>,
   pub scale_y: Option<f32>,
   pub rotation: Option<f32>,       // degrees
   pub opacity: Option<f32>,        // 0.0 - 1.0
   pub visible: Option<bool>,
   pub z_order: Option<i32>,
   pub color: Option<(u8, u8, u8)>,
   pub background_color: Option<(u8, u8, u8, u8)>, // RGBA
   ```

2. **`src/style.rs`** — 新增 `Decl` 变体 + 解析逻辑：
   - `Decl::Transform(Vec<TransformFn>)` — 需要新增 `TransformFn` 枚举
   - `Decl::Opacity(f32)`
   - `Decl::Visibility(bool)`
   - `Decl::ZIndex(i32)`
   - `Decl::Color(u8, u8, u8)`
   - `Decl::BackgroundColor(u8, u8, u8, u8)`

3. **`src/backend/cocos.rs`** — `emit_base_properties` 改为从字段读取：
   - Scale: 读 `scale_x`/`scale_y`，默认 1.0
   - Rotation: 输出 `RotationSkewX`/`RotationSkewY`
   - CColor: 从 `color`/`opacity` 合成 RGBA
   - VisibleForFrame: 从 `visible`
   - ZOrder: 从 `z_order`
   - FlipX/FlipY: 从 scale 负值推断

### 颜色解析：使用 cssparser-color

添加 `cssparser-color` 依赖（与 `cssparser` 同仓库 `servo/rust-cssparser`，同团队维护，版本兼容有保障）。

```toml
# Cargo.toml
cssparser-color = "0.5"
```

该 crate 提供统一的颜色解析入口，一个函数覆盖所有格式：

```rust
use cssparser_color::{parse_color_with, DefaultColorParser, Color};

// 在 LayoutDeclParser::parse_value 中：
"color" | "background-color" => {
    let color = parse_color_with(&DefaultColorParser, input)
        .map_err(|_| input.new_custom_error(()))?;
    // ...
}
```

覆盖范围：`#RGB`、`#RRGGBB`、`#RRGGBBAA`、`rgb()`、`rgba()`、`hsl()`、`hsla()`、148 个命名颜色——无需手写任何分支。

### 注意事项

- `transform-origin` 已被解析为 anchor_x/anchor_y，新增的 transform 解析要与之共存
- 这些属性全部是引擎无关的，未来新增 backend 直接可用

---

## 二、组件化分层

### 架构总览

```
┌──────────────────────────────────────────────────┐
│  HTML+CSS 基础层（引擎无关，共享）                  │
│                                                  │
│  parser.rs: div/img/span → Layout/Image/Text     │
│  style.rs:  所有 CSS 属性提取                      │
│  layout.rs: Taffy flexbox 布局                    │
│                                                  │
│  此层尽可能做厚，覆盖所有通用概念                    │
├──────────────────────────────────────────────────┤
│  组件层（per-backend）                             │
│                                                  │
│  每个引擎一个 Vue 组件包：                          │
│    @converter/cocos-components                   │
│    @converter/unity-components                   │
│                                                  │
│  组件通过 SSR 输出带 data-widget 的 HTML           │
│  → 进入共享的 parser 管线                          │
├──────────────────────────────────────────────────┤
│  Backend 层（per-backend，已有此架构）              │
│                                                  │
│  CocosBackend.emit()  →  .csd XML                │
│  UnityBackend.emit()  →  .prefab YAML            │
└──────────────────────────────────────────────────┘
```

### HTML 原生标签保留（引擎无关）

以下映射保留在 `classify_element()` 中，所有 backend 共享：

| HTML 标签 | WidgetKind | 理由 |
|---|---|---|
| `<div>` 等容器标签 | `Layout` | 容器是最基础的概念，所有引擎都有 |
| `<img>` | `Image` | 图片显示是通用概念 |
| `<span>`/`<p>`/`<label>` | `Text` | 文本显示是通用概念 |

这三类标签的 CSS 属性完全能覆盖它们在各引擎中的基础行为，不需要组件化。

### 需要组件化的类型（以 Cocos 为例）

这些类型有**引擎特有语义**——props 不同引擎不同，HTML 原生标签无法自然表达：

| Cocos 类型 | 为什么需要组件化 | 组件 Props 示例 |
|---|---|---|
| **Button** | 三态贴图（normal/pressed/disabled）是 Cocos 特有 | `normal`, `pressed`, `disabled`, `text`, `font-size` |
| **CheckBox** | 五态贴图模型（background/cross × normal/pressed/disabled） | `bg-normal`, `bg-pressed`, `cross-normal`, ... |
| **Slider** | 三贴图（bar/ball/progress） | `bar`, `ball`, `progress`, `percent` |
| **LoadingBar** | 方向 + 贴图 + 百分比 | `texture`, `direction`, `percent` |
| **TextField** | placeholder 样式、输入限制 | `placeholder`, `max-length`, `password` |
| **ScrollView** | 内容区域尺寸、滚动方向、弹性 | `direction`, `bounce`, `inner-width`, `inner-height` |
| **ListView** | item 模板、滚动方向、间距 | `direction`, `item-margin`, `gravity` |
| **PageView** | 无 HTML 对应 | `direction`, `page-count` |
| **TabControl** | 无 HTML 对应 | `header-height`, `selected-index` |
| **Sprite** | 与 ImageView 不同的渲染模式 | `texture`, `blend-mode` |
| **ProjectNode** | 嵌套引用 | `file-path` |
| **TextBMFont** | 位图字体 | `fnt-file`, `text` |
| **TextAtlas** | 图集数字 | `atlas-file`, `text`, `char-width` |

### 组件化对 parser 的影响

**影响很小。** 流程是：

1. Vue 组件 SSR → 输出 `<div data-widget="Button" data-pressed="btn_p.png">...</div>`
2. 现有 `parse_data_widget()` 识别 `data-widget` → 生成对应 `WidgetKind`
3. 所有 `data-*` 属性已经被收集到 `node.attrs` 中
4. Backend 从 `attrs` 读取引擎特有参数

需要改动的是：
- 补齐 `parse_data_widget()` 中缺失的类型（TextBMFont、TextAtlas 等）
- 对应 backend 的 `emit_node` 补齐这些类型的输出逻辑

### 组件化对 web 编写的影响

**基础写法跨引擎完全一致：**
```html
<!-- 这部分不管目标引擎是什么，写法一样 -->
<div style="display:flex; width:640px; height:960px;">
  <img src="bg.png" style="width:100%; height:100%;"/>
  <span style="font-size:24px; color:#fff;">Hello</span>
</div>
```

**差异仅在组件引用上：**
```vue
<!-- Cocos 项目 -->
<script setup>
import { CocosButton, CocosLoadingBar } from '@converter/cocos-components'
</script>
<template>
  <CocosButton normal="btn.png" pressed="btn_p.png">Start</CocosButton>
  <CocosLoadingBar texture="hp.png" direction="left" :percent="80"/>
</template>
```

```vue
<!-- Unity 项目（将来） -->
<script setup>
import { UnityButton, UnitySlider } from '@converter/unity-components'
</script>
<template>
  <UnityButton sprite="btn.png" transition="color">Start</UnityButton>
  <UnitySlider fill-image="hp.png" fill-origin="left" :value="0.8"/>
</template>
```

切换目标引擎 = 换一个 import 包 + 换组件名。div/img/span 的写法不变。

---

## 三、实施顺序

### Phase 1：补齐 CSS 通用属性 ✅ 已完成

- `types.rs`: LayoutProps 新增 scale_x/scale_y/rotation/opacity/visible/z_order/color/background_color
- `style.rs`: transform/opacity/visibility/z-index/color/background-color 解析 + cssparser-color 集成
- `cocos.rs`: emit_base_properties 从字段读取替代硬编码
- 单元测试全部通过

### Phase 2：补齐 Cocos backend 缺失类型 ✅ 已完成

- `parse_data_widget()` 已包含 TextBMFont/TextAtlas/Sprite/ProjectNode/Node/PageView/TabControl
- `ctype_for()` 映射完整
- `emit_widget_elements()` 补齐 Sprite/TextBMFont/TextAtlas/ProjectNode 的 FileData 输出
- ProjectNode 优先使用 `data-file` 属性

### Phase 3：消除 HTML 标签到引擎控件的直接映射 ✅ 已完成

`classify_element()` 中移除了：
- `<button>` → Button（改为 Layout）
- `<input>` → TextField/CheckBox/Slider（改为 Layout）
- `<progress>` → ProgressBar（改为 Layout）
- `<ul>`/`<ol>` → ListView（改为 Layout）
- `<a>` → Button（改为 Layout）

保留在基础层：
- `<div>` 等容器标签 → Layout（引擎无关）
- `<img>` → Image（引擎无关）
- `<span>`/`<p>`/`<label>` → Text（引擎无关）
- `overflow:scroll` → ScrollView（CSS 语义与引擎行为接近）

所有测试已更新为使用 `data-widget`。

### Phase 4：Vue 组件库 ✅ 已完成

`frontend/src/components/` 新增 11 个 Cocos 组件：
- CocosButton, CocosCheckBox, CocosSlider, CocosTextField
- CocosScrollView, CocosListView, CocosPageView
- CocosSprite, CocosTextBMFont, CocosTextAtlas, CocosProjectNode

每个组件输出 `data-widget` + `data-*` 属性的 HTML（供 converter SSR 消费），同时提供浏览器预览渲染。
组件已注册到 `index.ts`，TypeScript 类型检查和 Vite 构建均通过。

### 待做：Skill/Prompt 模板

从组件 props 定义自动生成 LLM 可用的组件清单——后续单独规划。

---

## 四、设计原则

1. **HTML+CSS 基础层做厚**——能从标准 CSS 提取的信息就在 parser 层提取，不留给组件或 backend
2. **组件层是 per-backend 的**——不同引擎的组件包不同，这是设计而非缺陷
3. **Web 端基础写法跨引擎一致**——div/img/span + CSS 不变，差异封装在组件 import 中
4. **data-widget 是组件到 parser 的桥梁**——Vue SSR 输出 data-widget 标记的 HTML，parser 不需要理解 Vue
5. **渐进可用**——Phase 1-2 独立有价值，不依赖 Phase 3-4

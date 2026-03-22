# 组件化方案设计讨论记录

> 此文件记录关于 Converter 输入格式、组件化方案、工具形态及行业背景的设计讨论。

## 项目核心目的

LLM 部分取代传统 **设计师 → UI 拼接工程师 → 程序员** 链路：

```
传统流程：
  设计师 (Figma) → UI 拼接工程师 (引擎编辑器) → 程序员 (写业务代码)
                    ↑                              ↑
目标：          Converter A 取代               Converter B 辅助
            LLM 生成 IR → 引擎资产         引擎资产 → 精简描述 → LLM 写代码
```

- **Converter A**：LLM 不能直接生成引擎 UI 资产，需要 IR + 转换。当前 IR 是 HTML+CSS
- **Converter B**：将引擎资产输出为精简可读的结构描述，让 LLM 能理解现有 UI 并编写业务代码。这部分相对简单且已有大量实践

### 关于传统分工的判断

设计师留在 Figma、工程师留在引擎——这个分工格局短期不会变：
- 设计师需要轻量、快速、可协作的工具来探索方案（Figma 的核心价值）
- 引擎编辑器是开发工具，不是设计工具——操作复杂、迭代慢、协作能力弱
- 行业试过让设计师直接在引擎工作，大多退回了分工模式
- 本工具不是消除分工，而是**自动化 handoff**——用 LLM + Converter 替代手动拼接还原

## Converter A 输入格式讨论

## 纯 HTML+CSS 的优势与不足

### 优势
- LLM 生成 HTML 的训练数据最充分，输出最可靠
- 浏览器直接预览，零构建步骤
- html5ever 成熟解析，Rust 端实现简单

### 不足
- **语义丢失**：游戏 UI 组件（进度条、装备槽等）需要用多个 HTML 元素 + CSS 定位模拟，组件语义隐含在结构模式中，converter 靠猜
- **无抽象能力**：重复结构（如 40 个背包格子）只能逐个写，token 线性增长
- **游戏概念用 CSS 表达是 hack**：锚点、九宫格、图集等无 CSS 原生对应，靠 data-* 凑合
- **标签集固定**：div/button/img/label 无法表达开放的游戏组件集

## 组件化是必要的

游戏开发积累了大量 UI 组件（ProgressBar、TabPanel、EquipSlot、ChatBubble 等），不同项目不同，不可能在 converter 中穷举。需要可扩展、可复用的组件模式。

## 架构设计：三层分离

### 1. Skill/Prompt 层（非确定性引导）

为不同 backend 提供组件信息，在 LLM 生成前注入：
- 可用组件列表
- 每个组件的 props schema
- 使用示例

让 LLM 知道能用什么、怎么用。

### 2. 组件库层（确定性保障）

预定义好的组件实现，LLM 直接引用而非重新拼装：
- LLM 写 `<ProgressBar value="850" max="1060" />` 而不是手动用两张 img 叠进度条
- 组件内部结构是确定性的，由组件库保证正确
- LLM 只决定用什么组件、填什么值

### 3. Backend 映射层（确定性转换）

组件展开和引擎资产映射的复杂度放在 backend 里处理。

### 信息流

```
Backend 组件协议（一份定义）
  ├→ 生成 Skill/Prompt（LLM 消费）
  ├→ 组件库实现（浏览器预览 + LLM 使用）
  └→ Converter backend 规则（转换消费）

LLM + Skill + 设计稿
  → 组件化 UI 描述
  → 浏览器预览验证 ✓
  → Converter 解析 → Backend 展开 → 引擎资产
```

## 框架选择：Vue

### 为什么不是 React

React 的 JSX 不是 HTML：
- `class` → `className`
- `style="width:420px"` → `style={{width: 420}}`
- 列表渲染用 JS 的 `.map()` 而非声明式指令
- 标记和逻辑混合

### 为什么是 Vue

Vue template 本质上是增强版 HTML，是从纯 HTML+CSS 到组件化的最自然过渡：
- `class`、`style` 字符串语法与标准 HTML 一致
- `<template>` 区域是纯标记，`<style>` 分离 —— 结构就是 HTML+CSS
- `v-for`、`v-if` 是 HTML 属性而非 JS 表达式，LLM 犯语法错误概率更低
- LLM 从纯 HTML 过渡到 Vue 的认知跳跃最小
- Vue 训练数据量充足

## 工具形态：CLI + 浏览器预览

### 为什么不做 Tauri 桌面应用

曾考虑 Tauri 桌面端（Rust 后端 + Vue 前端 webview），但评估后放弃：

- **预览问题浏览器本身就能解决**：Vite dev server 热更新预览、浏览器 DevTools device mode 模拟分辨率/横竖屏
- **Tauri 解决不了的问题它也解决不了**：Web 端的 CSS 布局 ≠ 引擎端的锚点布局，文字渲染、缩放行为、设计分辨率适配策略都不同。预览始终是"近似"的，真正的适配验证只能在引擎里做
- 加一个 Tauri 壳子的收益（合并 CLI + 浏览器到一个窗口）远不值工程成本

### 实际工具链

```
LLM → HTML/CSS (Vue) → 浏览器预览（Vite dev server，近似验证）→ Converter CLI → 引擎资产 → 引擎内验证（最终确认）
```

- **Converter 保持 CLI 工具**
- **预览靠 Vite dev server + 浏览器 DevTools**
- 精力放在 converter 转换质量上——缩小 web 预览与引擎实际效果的差距，比做桌面 app 有价值

## 对现有 Parser 的影响

**不需要大改。**

如果走 Vue SSR 路线，Vue 组件经过 SSR 后输出的是标准 HTML。现有 html5ever parser 仍然接收 HTML。

需要的改动：
1. 组件 render 输出的 HTML 需要带 `data-widget` 等标记保留组件语义
2. Parser 加一条规则：遇到 `data-widget` 走组件映射路径，而非按 div/img 逐元素解析
3. 组件展开的复杂度放在 backend 里处理

## 行业背景：游戏引擎与声明式 UI

### 现状

主流游戏引擎（Unity、Unreal、Cocos、Godot 及各 in-house 引擎）全部采用可视化编辑器 + 资产文件模式。代码优先的声明式 UI 框架在游戏引擎中约等于不存在。

唯一例外：Unity UI Toolkit（借鉴 web 的 UXML + USS），但普及率仍低。

### 根本原因

- **用户群体不同**：游戏 UI 制作者大比例是美术/设计师，可视化编辑器是刚需
- **布局模型不同**：游戏 UI 是绝对定位 + 锚点适配，不是文档流 + 响应式
- **非标准内容多**：3D 预览、粒子特效、Spine 动画等混在 UI 中，不适合 DOM 模型
- **存量和惯性**：大量现有项目、从业者技能栈，迁移成本极高

### 推动变化的力量

- Unity UI Toolkit 向 web 范式靠拢是明确信号
- **LLM/AI 工具链**：LLM 能生成代码不能操作编辑器——没有代码表示的 UI 系统无法接入 AI 工具链，这可能是最大的加速器
- 团队降本增效压力

### 对本项目的意义

- **短期**：引擎没有声明式 UI → converter 是必要的桥梁
- **中期**：引擎开始支持声明式 → backend 可输出声明式代码而非资产文件
- **长期**：引擎原生支持类 web 声明式 UI → LLM 直接生成引擎代码 → converter 使命完成
- 以行业惯性来看，这个窗口期足够长

## 未决事项

- [ ] Vue 组件库的具体组件清单和 props 设计
- [ ] Skill/Prompt 模板的格式和内容规范
- [ ] Backend 组件协议的定义格式（如何从一份定义派生 prompt、组件库、converter 规则）
- [ ] Web 预览与引擎实际效果的差距如何缩小
- [ ] 纯 HTML+CSS 模式是否保留作为降级方案
- [ ] Converter B 的输出格式规范（精简可读的引擎资产描述）

# SPIKE — gdi32full 原生 per-lookup 可行性（2026-09-06）

目标：判定能否对 **系统 usp10 的真实引擎（gdi32full.dll 内部）** 做原生逐-lookup
hook，产出 `final` 与 `stages` 同源的真逐-lookup trace（替代 HB 代理）。

## 已确认事实（objdump 静态反汇编，gdi32full 10.0.26100.x，x64）
- ImageBase = `0x180000000`；ScriptShapeOpenType 实现 @ RVA `0x5a180`（VA `0x18005a180`）。
- `0x5a180` 是一个**巨型同步函数**（栈参 0x108…0x140 对应 ScriptShapeOpenType 的超长
  签名，先做 cGlyphs/pwOutGlyphs 等合法性校验），在**调用线程**上直接 call 一批
  gdi32full 模块内助手——**驱动层看不到 threadpool 派活**。→ 修正早前"深层成型在
  worker 线程"的猜测：driver 在主线程同步跑。且代码 hook 本就按"谁执行谁触发"，
  线程亲和不是障碍。

## 候选地址判定（在 0x5a180 体内反复被调的小函数）
- `0x5b880`：**不是**——是引用计数 Release（`lock xadd 0x18(%rcx)` / `lock cmpxchg
  0x10(%rcx)` / jmp import）。
- `0x5b8fc`：**不是**——SIMD `paddq` 十路求和（小工具）。
- `0x5ec78`：**不是**——2 字节代码→glyph 映射 + 标点(0x2c,0x3b,0x3f)特判（cmap/
  charclass 层）。
- `0x5eb64`：每记录匹配/写回助手，区域被调 ~8 次；prologue 标准（`mov %rsp,%rax`+
  4 存储 + 4 push ≈ 13B，可 12B 内联 patch + trampoline）。但看调用点上下文（0x5c770
  附近对 `(%rsi,%rbx,4)` u16 记录做 0xC63E/0xC5E0/0xC11A/0xC0BC/0xC6FA/0xC69C 掩码、
  循环 92 次 `cmp $0x5c,%r14d`），这是**逐 glyph 状态机/属性置位**，不是 GSUB lookup 遍历。
- 结论：ScriptShapeOpenType 表面层 = 校验 + cmap/charclass + 逐 glyph 状态机；
  **"逐 GSUB lookup 应用"的循环还没在这段里直接现形**（被更深一层封装）。

## 还差什么（真正的难点）
1. 定位 gdi32full 里读取**解析后 GSUB LookupList**、按 feature 顺序逐个 lookup 应用
   的那段（应在 script-cache 布局对象里，特征：含 OT 表头偏移/feature tag 常量如
   'GSUB'/feature tag u32、lookup type 分派）。
2. 摸清内存里 glyph-buffer 记录对象（指针/计数/stride）以快照每 lookup。
3. 验证蒙古文路径（字体带 'mong' GSUB）确实命中该点 + hook 点稳定可迁移
   （selfhook.rs 已支持 RVA+prologue 的 relocate）。

## 判定
- **原则上可行**：引擎进程内、主线程同步、代码 hook 不挑线程；候选助手 prologue 均
  可 hook。
- **现实**：gdi32full 是高度优化无符号的手写单体，定位"LookupList 遍历 + glyph buffer
  布局"= DESIGN 早已标注的 **long RE effort**（pydwshape/DWriteCore 同款工作量），
  不是一次会话能可靠交付的改动，存在风险。

## 检查点 7（表驱动对象框架确认 + 逐-lookup 判否）
- 分派表 @ `.rdata 0x1800b4988`（槽成员：0x4eb50,0x53b90,0x4b430,0x7b950,0x7a10,
  0x29ab0,0x4b720,0x4e180,0x4bb50,0x57910,0x4bb80,0x4cfb0…），全局指针 `0x1800b4900`。
- `0x4e970` = **按记录类型 switch 的 size/compute 分派**（switch (%rcx) type：0xc、
  0x28–0xd0 区间；返回 0x8/0x10/0x18/0x20/0x100 等结构尺寸）；`0x4eb50` = **表头 info
  getter**（读 obj+0x5c/0x60/0x64/0x10/0x38）→ OT 引擎是**表驱动对象框架**，per-type
  处理器经表间接调用，无独立可 hook 的逐-lookup 分派器。
- **决定性实验**（callee 探针 hook 全部表成员，数一次 shape 的触发，边界 B 由 objdump
  指令边界保证 ≥5，只 hook 真函数入口——16B RIP 热桶不许 hook，会崩）：蒙古文
  `ᠰᠠᠢᠢᠨ` / `ᠰ` / `ᠰ`×8 / latin `saihan`：
  - `0x4b720` 与 `0x4e180` 永远同数（成对触发）：**4/item + 9/蒙古字形**
    （4+9·6=58 ✓、4+9·8=76 ✓、latin 仅 item 基础 4，0/字形）。
  - `0x4cfb0`：**3/item + 5/字形**。`0x53b90` ≈ /字形。0x4bb50/0x57910 常量 x3。
    0x4eb50/0x7b950/0x7a10/0x29ab0/0x4b430 从不触发（解析/其他路径处理器）。
  - **结论：无任何表成员是逐-lookup 边界**——全是逐字形扫描原语（latin 0/字形 =
    不走 OT 上下文路径）。逐-lookup apply 是内联 driver 循环（lookup list 迭代），
    不是表成员函数 → 无法靠 hook 单个表成员拿 per-lookup 快照。
- 含义：若继续，下一杠杆 = 用 RIP 桶收窄到指令级定位 **lookup-list driver 循环**
  （迭代 feature/lookup 并调 table[type] 的那段），再对 glyph-buffer 对象做内联 hook
  每轮快照——代价高于预期，无干净边界函数可挂。
- 建议：若 babelsoft 端当前瓶颈只是"stages 必须逐-lookup 且 final==usp10"，则先落地
  **wineusp 真逐-lookup（已全语料 final 逐位 == usp10）** 作为该路径的 stages 来源
  （彻底去掉 HB），原生 gdi32full hook 留作独立、可后置的长期 RE 项。

---

# 会话 3 追加（2026-09-06，进程内 hook 实证 → 修正线程模型）

## 关键修正：gdi32full 成型**不建线程**，引擎在主线程 0x5a180 内
- 进程内（src/worker_hook.rs，无 frida 冲突、可靠）hook 了 ntdll/kernel32 全套线程
  原语：TpSimpleTryPost/TpPostWork/TpAllocWork/CreateThreadpoolWork/
  TrySubmitThreadpoolCallback/CreateThread —— shape saihan 期间 **events=0**（全不触发）
  → 早前"引擎在 worker 线程"是**坏 Stalker 的假象**（frida_probe_calls 的 ~5 个 call
  是主线程 0x5a180 前 ~0x500B 校验区的真实调用，被误读）。
- 修正结论：**逐-lookup 引擎就在主线程的 0x5a180 巨型函数里**。为何 frida inline 探针
  只见 4-5 个内部调用？因它只 hook 了我列出的候选 RVA，且 0x5a180 对 saihan 走的路径
  未到达那些 helper（0x5b880 等）——引擎调用的是**别的内部函数**（0x41760/0x42990/
  0x8555c/0x9a8c0/0x1da24/0x98504… 这些 0x5a180 的"远调用"）。

## 已建的进程内 hook 设施（src/worker_hook.rs + lib.rs env 开关）
- `PYUSP_WORKER_PROBE=1`：hook 线程/线程池原语并报 gdi32full 回调（结论：0 触发）。
- `PYUSP_CALLEE_PROBE=1`：对 0x5a180 远调用目标上计数（找高频 OT 引擎）。
- **hook_one 有 trampoline 指令边界 bug**（把 12 字节当指令边界重放→0xC0000005）：
  例如 0x41760 prologue= `sub rsp,0x28`(4B)+`cmpl`(3B)+`je`(6B)，12 字节切在 je 中间。
  修法：E9 rel32 5B patch + 用 objdump 预先验证的"指令边界 ≥ patch 长度"的 prologue
  长度 B（trampoline 重放 orig[0..B] 再 jmp target+B），否则跳过不 hook。

## 0x5a180 远调用目标 prologue 实测（tools/native_spike/far_prologues.txt，均可 hook）
- 干净入口：0x41760(`sub rsp,0x28`)、0x8555c、0x9a8c0、0x1da24、0x1da84、0x98504、
  0x306a8、0x2b750、0x538b0、0x8b2d8、0x23590、0xae5c0、0xae524、0xb069c、0xcb80、
  0xc890、0xab40、0x8850、0xcd40、0xb480、0x5b864、0x5b880、0x5b8c0、0x5b8fc、
  0x5b93c、0x5eb64、0x5ec78、0x42990(`mov ecx,eax`)。
- import 跳板（跳过）：0x87db8/0xb0a1c/0xb0a28/0xb2010（jmp *IAT）。
- 下一个决定性动作：修 hook_one 边界后对以上目标计数 → 哪几个高频（每 lookup 一次）
  = OT 逐-lookup 分发器；拿到后 inline hook 它快照 glyph buffer（selfhook 的 GlyphRec
  布局待按对象摸）。

## 备忘
- objdump 批量反汇编/入口验证脚本：python/_fp.py（可复用）。
- worker_hook.rs 的 CALLBACKS/EVENTS 用 Mutex<Vec>，线程安全；Guard Drop 还原。
- 0x5a180 对"无 feature 参数(cranges=0)"的调用也会出正确 GSUB 结果(303/471)，
  说明系统在无显式 feature 时仍应用字体默认 feature——这点与 wine 不同，注意别被误导。

---

# 会话 6 追加（2026-09-06，定位 Uniscribe 分派表）
- 0x10380 最热其实是**堆/分配器释放例程**(链表头 -0xc/-0x10 + 全局 0x109538)，OT 高频
  分配所致，非 OT 逻辑。
- `0x4cfb0`(读 OT 解析数据 0x87c u16 + 返回 +0x880 ptr 的 getter)全 .text **无静态
  调用** → 地址存在数据表里。DLL 里搜 0x18004cfb0 8B → 落在 **.rdata 0x1800b49b0**，
  是一张**函数指针分派表**(基址 0x1800b4988)。邻槽全是 .text 函数：0x4eb50/0x53b90/
  0x4b430/0x7b950/0x7a10/0x4cfb0/0x29ab0/0x4b720/0x4e180/0x4bb50/0x57910…
  **与 RIP 热区高度重合**(0x4cfb0/0x4e180/0x53b90 蒙古热、0x4bb50/0x57910 latin 热)
  → 这就是 Uniscribe OT 引擎组件分派表(0x4cfb0 是第 5 槽)。
- 表基 0x1800b4988 被 ~9 个驱动函数 lea 引用：0x4ebd/0xac1c/0xad81/0xb408/0xb42b/
  0xb448/0xbe44/0x3e08b/0x40807/0x846d1（0x4a39d/0x4d037 引 0x1800b4900，全局指针）。
- 0x4d030 区代码：`mov 0x1800b4900 存全局指针; 按 glyph 高位取表; ...` = cmap/charclass
  经该表；0x4d037 读全局 0x1800b4900。

## 下一步
- 逐个看 0xb4988 表引用驱动(0x4ebd 最近，0x4eb50 是表首成员)谁在**循环按槽调用**
  (每 lookup/每 op 一次) → 那就是逐-lookup 驱动器；或扩大 RIP 采样定位到驱动函数的
  interior 循环指令 → interior-hook 抓每 lookup 的 glyph buffer。

---

# 会话 5 追加（2026-09-06，RIP 采样器交付 → OT 引擎定位）

## 进程内 RIP 采样器（worker_hook.rs：rip_start/rip_report + PYUSP_RIP_PROBE=1）
- 副线程对主(成型)线程 SuspendThread→GetThreadContext→ResumeThread 循环(200k 上限 +
  yield_now 防饿死)。windows-rs 的 CONTEXT 被 cfg 掉 → 用 GetProcAddress 原始调
  GetThreadContext + 手写 x64 CONTEXT(16B 对齐缓冲 0x4D0，ContextFlags@0x30=0x100001，
  Rip@0xF8)。已跑通：蒙古文长文本 10070 样本 / latin 7393 样本。
- **注意**：rva > .text 上限 0xc15ef(如 0x15ccf0 落在 .rsrc)的桶是噪声(部分上下文脏读)，
  只信 .text 内(<0xc2000)的桶。

## RIP diff 结果：蒙古 OT 引擎真实热区（.text 内，蒙古独有或远热于 latin）
- 蒙古独有：`0x10380`(x1945,最热)/`0x10200-0x10420` 带、`0x4cfb0/0x4cfc0`、`0x2d180`、
  `0x64c30`、`0x53bc0`；蒙古更热：`0x28a10`(1286 vs 77)、`0x4e180`。
- latin 独有(基线)：0x2b260/0x4bb80/0x4980/0xb0a20/0x57910/0x41760/0x42990…
- `0x4cfb0` 已反汇编 = 小 getter：读 cache `0x10(obj)->0x50` 再 `movzwl 0x87c(%r9)`
  存到出参 + 返回 `+0x880` 指针 → 像是取 OT 解析后数据(每 glyph 结构/表)的入口，
  per-glyph 高频调用。
- 反汇编 0x10000-0x10600 是多小函数带(调 0x42990/0x10480/0x110bc/0x105d8)，0x10380
  落在其中某函数 → 需细读定位真正的逐-lookup 循环。

## 下一步（继续长期路线）
逐函数细读上述蒙古独有热区，找"循环遍历 LookupList、按 lookup type 分派、改 glyph
buffer"的代码；或先扩大 RIP 采样到**16B 内逐条**(改桶粒度/加每函数边界)做更细定位，
再 interior-hook。

## 已备产物
- tools/native_spike/rip_{mong,latin}.txt(原始 UTF-16，_u8 已解码)、sso_full.txt、
  prologue_B.txt、hot_*.txt、SPIKE.md。
- worker_hook.rs: arm/arm_rvas/rip_start/rip_report；lib.rs env: PYUSP_WORKER_PROBE/
  PYUSP_CALLEE_PROBE/PYUSP_CALLEE_ONE/PYUSP_RIP_PROBE。

---

# 会话 4 追加（2026-09-06，决定性可行性结论）

## 进程内 hook 修好 + 决定性实验
- **修复 hook_one trampoline 指令边界 bug**：改 E9 rel32 5B patch + objdump 预验证的
  "安全 prologue 前缀 B(≥5 的指令边界)"，trampoline 重放 orig[0..B] 再 jmp target+B
  → PYUSP_CALLEE_PROBE 不再崩(此前 12B 整块重放切在指令中间→0xC0000005)。
  边界表/tools/native_spike/prologue_B.txt、_pb.py；lib.rs 内联 (rva,B) 表。
- **callee 探针实证（saihan vs latin 完全一致）**：shape 主线程只调用 4 个一次性
  小函数 0xab40/0xc890/0xcb80/0x41760（各×1），**0x5eb64 及其余候选全 0 命中**；
  latin 与 saihan 完全相同 → 蒙古文全部 GSUB(init/medi/fina/rclt) 没有引发任何额外
  函数调用。
- **0x5a180 真终点 0x5eb59(ret)**（前面只反汇编到 0x5d000 是误区）；0x5a180–0x5eb60
  区域是**大量小函数**（非单体）。0x5eb64 静态被 ~30 次调用但运行时 0 命中 → 那些
  调用在**我们走的路径未到达的其它函数**里。
- **结论（关键）**：系统 usp10 的 OT 引擎（per-lookup 应用）是**内联在 0x5a180 主体
  里的**——不存在一个"每 lookup 一次"的独立可 hook 分发函数（这正是早前找不到的
  根因）。要真逐-lookup 只能：(a) 静态细读 0x5a180 内联引擎定位 per-lookup 循环内部
  指令再 interior-hook（需摸 glyph buffer 布局，工作量大且不保证收敛）；或 (b) 走
  wineusp 真逐-lookup（已逐位==usp10 final，去掉 HB）满足 babelsoft 需要。

## 判定更新
- "找独立 per-lookup 分发器来 hook" 的路线**基本证伪**（引擎内联，无独立函数）。
- 原生 usp10-internal 真逐 lookup = 高成本、低确定性（内联引擎 interior RE）。
- 建议：babelsoft 需求（stages 逐-lookup 且 final==usp10）用 wineusp 真逐-lookup
  路线即可达成；gdi32full 内联引擎 RE 仅作长期可选课题。


---

# 会话 2 追加（2026-09-06，inline 探针证据）

## 决定性发现：引擎在 gdi32full **自建线程**上跑（不是主线程、不是 threadpool work）
- **gdi32full 静态导入里 threadpool 只有 timer 函数**（CreateThreadpoolTimer/
  SetThreadpoolTimer/CloseThreadpoolTimer/WaitForThreadpoolTimerCallbacks），**没有
  work item 导入**；但 **.rdata 含 "CreateThread" 字符串** + 导入 `GetProcAddress`、
  `WaitForSingleObject`/`WaitForSingleObjectEx` → gdi32full **用 GetProcAddress 动态
  解析 CreateThread** 自建成型线程，主线程用 WaitForSingleObject 等它。
- **inline Interceptor 探针**（tools/native_spike/inline_probe.py，0x5a180 onEnter
  清零 / onLeave 发结果，进程存活期抓取）：shape saihan 全程在主线程只命中 4 个
  **一次性设置调用**：RVA `0xcb80`×1、`0xc890`×1、`0x41760`×1、`0x86cc0`×1；深层候选
  （0x5eb64/0x5ec78/0x5b880/0x42990/…）**全部 0 命中**，但 final 正确
  `[675,281,303,471,281,351]` → 逐-lookup 引擎不在这些函数、不在主线程。
- 早前 frida_probe_calls（Stalker 全线程跟随）"只见 5 个一次性 call"与此一致：主线程
  只跑 0x5a180 的前 ~0x500B 校验 + 交线程 + 等待。

## 下一步（下一会话首选）
1. **抓 gdi32full 自建线程入口**：hook `kernel32!CreateThread`/`kernelbase!CreateThread`
   （frida Interceptor 即可，tools/native_spike/thread_creator.py 已备），start routine
   若落在 gdi32full → 得 worker 引擎入口 RVA；随后对该入口 inline hook / Stalker follow，
   得到引擎内部的 call 直方图 → 逐-lookup 分发器。
2. 或用进程内更稳的路线：在 pyusp 的 **selfhook.rs**（Rust 内联 hook，已编译）里先 hook
   CreateThread，规避 frida spawn 的竞态。
3. 拿到 worker 入口后：其内部按 call 频率挑出"每 lookup 一次"的函数 → 快照 glyph buffer
   → 组装真逐-lookup stages，与 wineusp trace 对拍。

## 探针工具（tools/native_spike/）
- `inline_probe.py`：Interceptor 内联计数一组候选 RVA（结果在 0x5a180 onLeave 发）。
- `thread_creator.py`：hook CreateThread 抓 worker 入口 RVA。
- 输出样例：counts_saihan.json / inline_saihan.json / thread_saihan.json。


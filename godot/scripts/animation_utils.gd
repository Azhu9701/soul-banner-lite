extends Node
## 通用动画工具——提供淡入、弹入、数值跳动、进度条平滑过渡等 UI 动效。
##
## 所有方法均为静态方法，可在任何 UI 组件中直接调用。
##
## 用法：
##   AnimUtils.fade_in(node, 0.3)
##   AnimUtils.pop_in(dialog_panel)
##   AnimUtils.button_press(btn)
##   AnimUtils.animate_number(cash_label, 10, 15, "$", "M")

class_name AnimUtils


# ═══════════════════════ 淡入/淡出 ═══════════════════════

## 淡入节点（modulate.a 从 0 到 1）
static func fade_in(node: Node, duration: float = 0.3) -> void:
	node.modulate.a = 0.0
	var t := node.create_tween()
	t.tween_property(node, "modulate:a", 1.0, duration).set_ease(Tween.EASE_OUT)


## 淡出节点（modulate.a 从当前到 0）
static func fade_out(node: Node, duration: float = 0.3) -> void:
	var t := node.create_tween()
	t.tween_property(node, "modulate:a", 0.0, duration).set_ease(Tween.EASE_IN)


# ═══════════════════════ 弹入/弹出 ═══════════════════════

## 弹入效果——缩放从 0.85 弹性回到 1.0，营造"弹出"感
static func pop_in(node: Node, duration: float = 0.35) -> void:
	node.scale = Vector2(0.85, 0.85)
	var t := node.create_tween()
	t.tween_property(node, "scale", Vector2(1.0, 1.0), duration) \
		.set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_BACK)


## 弹入效果（从指定缩放起点开始）
static func pop_in_from(node: Node, from_scale: Vector2 = Vector2(0.9, 0.9), duration: float = 0.3) -> void:
	node.scale = from_scale
	var t := node.create_tween()
	t.tween_property(node, "scale", Vector2(1.0, 1.0), duration) \
		.set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_BACK)


# ═══════════════════════ 数值动画 ═══════════════════════

## 数值跳动——Label 文本从 old_val 平滑变化到 new_val
## 可选 prefix/suffix 前缀后缀（如 "$" / "M"）
static func animate_number(label: Label, old_val: int, new_val: int, prefix: String = "", suffix: String = "", duration: float = 0.5) -> void:
	var t := label.create_tween()
	t.tween_method(
		func(v: int) -> void: label.text = prefix + str(v) + suffix,
		old_val,
		new_val,
		duration
	)


## 数值动画（float 版本），保留指定小数位
static func animate_number_float(label: Label, old_val: float, new_val: float, decimals: int = 2, prefix: String = "", suffix: String = "", duration: float = 0.5) -> void:
	var t := label.create_tween()
	t.tween_method(
		func(v: float) -> void: label.text = prefix + str(round(v * pow(10, decimals)) / pow(10, decimals)) + suffix,
		old_val,
		new_val,
		duration
	)


# ═══════════════════════ 进度条动画 ═══════════════════════

## 进度条平滑过渡——将 ColorRect 的 size.x 从当前值过渡到目标百分比对应宽度
## max_width: 进度条最大宽度（px）
## target_pct: 目标百分比（0.0 ~ 1.0）
static func animate_progress(bar: ColorRect, target_pct: float, max_width: float = -1.0, duration: float = 0.4) -> void:
	if max_width <= 0.0:
		max_width = bar.custom_minimum_size.x if bar.custom_minimum_size.x > 0.0 else bar.size.x
		if max_width <= 0.0:
			max_width = 130.0  # fallback
	
	var pct: float = clampf(target_pct, 0.0, 1.0)
	var t := bar.create_tween()
	t.tween_method(
		func(v: float) -> void: bar.size.x = v,
		bar.size.x,
		max_width * pct,
		duration
	).set_ease(Tween.EASE_OUT)


## 进度条动画（同时更新百分比文本），将 ColorRect 宽度从当前平滑过渡到目标
## pct_label: 显示百分比文字的 Label
static func animate_progress_with_label(bar: ColorRect, pct_label: Label, target_pct: float, max_width: float = -1.0, duration: float = 0.4) -> void:
	if max_width <= 0.0:
		max_width = bar.custom_minimum_size.x if bar.custom_minimum_size.x > 0.0 else bar.size.x
		if max_width <= 0.0:
			max_width = 130.0
	
	var pct: float = clampf(target_pct, 0.0, 1.0)
	var old_width: float = bar.size.x
	var t := bar.create_tween()
	t.tween_method(
		func(v: float) -> void:
			bar.size.x = v
			var current_pct: float = v / max_width
			pct_label.text = "%d%%" % int(current_pct * 100.0),
		old_width,
		max_width * pct,
		duration
	).set_ease(Tween.EASE_OUT)


# ═══════════════════════ 按钮动效 ═══════════════════════

## 按钮按下动效——快速缩放回弹，模拟物理按压感
static func button_press(btn: Button) -> void:
	var t := btn.create_tween()
	t.tween_property(btn, "scale", Vector2(0.92, 0.92), 0.07).set_ease(Tween.EASE_OUT)
	t.tween_property(btn, "scale", Vector2(1.0, 1.0), 0.10).set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_BACK)


# ═══════════════════════ 滑动/位移 ═══════════════════════

## 从下方滑入（适合弹窗、提示条）
static func slide_in_from_bottom(node: Node, target_y: float = -1.0, duration: float = 0.35) -> void:
	if target_y < 0.0 and node is Control:
		target_y = (node as Control).position.y
	
	var start_y: float = target_y + 40.0
	node.set("position:y", start_y)
	var t := node.create_tween()
	t.tween_property(node, "position:y", target_y, duration).set_ease(Tween.EASE_OUT)


## 从上方滑入
static func slide_in_from_top(node: Node, target_y: float = -1.0, duration: float = 0.35) -> void:
	if target_y < 0.0 and node is Control:
		target_y = (node as Control).position.y
	
	var start_y: float = target_y - 40.0
	node.set("position:y", start_y)
	var t := node.create_tween()
	t.tween_property(node, "position:y", target_y, duration).set_ease(Tween.EASE_OUT)


# ═══════════════════════ 列表项入场 ═══════════════════════

## 列表项逐项入场——每个子节点依次从左侧滑入并淡入
## stagger: 每个子节点之间的延迟（秒）
static func stagger_in(parent: Node, stagger: float = 0.05, duration: float = 0.25) -> void:
	var children: Array[Node] = []
	for c in parent.get_children():
		children.append(c)
	
	for i: int in children.size():
		var child: Node = children[i]
		child.modulate.a = 0.0
		child.position.x -= 20.0
		var t := child.create_tween()
		t.tween_property(child, "modulate:a", 1.0, duration).set_ease(Tween.EASE_OUT).set_delay(i * stagger)
		t.parallel().tween_property(child, "position:x", child.position.x + 20.0, duration) \
			.set_ease(Tween.EASE_OUT).set_delay(i * stagger)


# ═══════════════════════ 脉冲/强调 ═══════════════════════

## 脉冲效果——短暂放大再恢复，吸引注意力
static func pulse(node: Node, duration: float = 0.6) -> void:
	var t := node.create_tween()
	t.tween_property(node, "scale", Vector2(1.08, 1.08), duration * 0.5).set_ease(Tween.EASE_OUT)
	t.tween_property(node, "scale", Vector2(1.0, 1.0), duration * 0.5).set_ease(Tween.EASE_IN)


## 闪烁效果——modulate 快速闪烁
static func flash(node: Node, flash_color: Color = Color.WHITE, duration: float = 0.3) -> void:
	var original_modulate: Color = node.modulate
	var t := node.create_tween()
	t.tween_property(node, "modulate", flash_color, duration * 0.5)
	t.tween_property(node, "modulate", original_modulate, duration * 0.5)


# ═══════════════════════ StyleBox 工厂 ═══════════════════════

## 创建带圆角的卡片 StyleBoxFlat
static func card_style(bg_color: Color, radius: int = 8, shadow_enabled: bool = false) -> StyleBoxFlat:
	var sb := StyleBoxFlat.new()
	sb.bg_color = bg_color
	sb.set_corner_radius_all(radius)
	if shadow_enabled:
		sb.shadow_size = 4
		sb.shadow_offset = Vector2(0, 2)
		sb.shadow_color = Color(0.0, 0.0, 0.0, 0.12)
	return sb


## 创建圆角色条（Pill Badge）StyleBoxFlat
static func pill_style(bg_color: Color) -> StyleBoxFlat:
	var sb := StyleBoxFlat.new()
	sb.bg_color = bg_color
	sb.set_corner_radius_all(20)
	sb.content_margin_left = 10
	sb.content_margin_right = 10
	sb.content_margin_top = 3
	sb.content_margin_bottom = 3
	return sb


## 创建带圆角的按钮 StyleBoxFlat
static func button_style(bg_color: Color, radius: int = 6) -> StyleBoxFlat:
	var sb := StyleBoxFlat.new()
	sb.bg_color = bg_color
	sb.set_corner_radius_all(radius)
	sb.content_margin_left = 18
	sb.content_margin_right = 18
	sb.content_margin_top = 8
	sb.content_margin_bottom = 8
	return sb

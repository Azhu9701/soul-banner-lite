extends Control
## 生产线视窗——显示工厂和生产线状态。
##
## 通过 update(factories) 接收后端数据刷新显示。
## 使用 AnimUtils 实现进度条平滑过渡、状态色条、hover 效果等动画抛光。

class_name ProductionView

const T = preload("res://scripts/theme.gd")
const AU = preload("res://scripts/animation_utils.gd")

# ── Constants ──

const TYPE_LABELS: Dictionary = {
	"manual": "手工",
	"semi_auto": "半自动",
	"auto": "全自动",
}

const STATUS_LABELS: Dictionary = {
	"idle": "空闲",
	"producing": "生产中",
	"building": "建设中",
	"switching_to": "转产中",
}

# 布局常量
const ROW_HEIGHT: float = 44.0
const BAR_HEIGHT: float = 20.0
const BAR_WIDTH: float = 140.0
const ICON_SIZE: float = 24.0
const SECTION_GAP: float = 6.0
const CARD_RADIUS: int = 8
const ENTRY_STAGGER: float = 0.04
const PROGRESS_ANIM_DURATION: float = 0.45

# ── Private Vars ──

var _container: VBoxContainer

# 记录上一次每条产线的进度值，用于动画过渡
# key: "{factory_idx}_{line_id}" -> float
var _prev_progress: Dictionary = {}

# 记录每条产线的填充 ColorRect 引用，用于动画
# key: "{factory_idx}_{line_id}" -> ColorRect
var _fill_rects: Dictionary = {}

# 记录每条产线的百分比 Label 引用
var _pct_labels: Dictionary = {}


# ── Virtual Methods ──

func _ready() -> void:
	_container = VBoxContainer.new()
	_container.name = "Container"
	_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	add_child(_container)


# ── Public Methods ──

## 接收后端工厂数据并刷新 UI。
func update(factories: Array) -> void:
	# 清空旧内容
	for child in _container.get_children():
		_container.remove_child(child)
		child.queue_free()

	_fill_rects.clear()
	_pct_labels.clear()

	if factories.is_empty():
		var empty_label := Label.new()
		empty_label.text = "暂无工厂"
		empty_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		empty_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		empty_label.size_flags_vertical = Control.SIZE_EXPAND_FILL
		empty_label.add_theme_color_override("font_color", T.colors.text_muted)
		empty_label.add_theme_font_size_override("font_size", T.body)
		_container.add_child(empty_label)
		return

	for factory_idx: int in factories.size():
		var factory: Dictionary = factories[factory_idx]
		if factory_idx > 0:
			# 工厂之间添加细分隔
			var gap := Control.new()
			gap.custom_minimum_size = Vector2(0, SECTION_GAP)
			_container.add_child(gap)
		_add_factory_card(factory, factory_idx)

	# 列表项逐项入场动画
	await get_tree().process_frame
	AU.stagger_in(_container, ENTRY_STAGGER, 0.2)


# ── Private: Factory Section ──

func _add_factory_card(factory: Dictionary, factory_idx: int) -> void:
	# ── 工厂卡片容器（圆角 + 阴影） ──
	var card := Panel.new()
	card.name = "FactoryCard_%d" % factory_idx
	card.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	card.add_theme_stylebox_override("panel", AU.card_style(T.colors.card, CARD_RADIUS, true))
	_container.add_child(card)

	var card_vbox := VBoxContainer.new()
	card_vbox.name = "CardContent"
	card_vbox.add_theme_constant_override("separation", 4)
	card_vbox.add_theme_constant_override("margin_left", 12)
	card_vbox.add_theme_constant_override("margin_top", 10)
	card_vbox.add_theme_constant_override("margin_right", 12)
	card_vbox.add_theme_constant_override("margin_bottom", 10)
	card.add_child(card_vbox)

	# ── 工厂头部 ──
	var header := HBoxContainer.new()
	header.name = "FactoryHeader"
	card_vbox.add_child(header)

	var icon := Label.new()
	icon.text = "🏭"
	header.add_child(icon)

	var name_label := Label.new()
	name_label.text = factory.get("name", "未知工厂")
	name_label.add_theme_font_size_override("font_size", T.head)
	name_label.add_theme_color_override("font_color", T.colors.text_strong)
	header.add_child(name_label)

	var capacity: int = factory.get("capacity", 0)
	var lines: Array = factory.get("lines", [])
	var cap_label := Label.new()
	cap_label.text = " 容量:%d  产线:%d" % [capacity, lines.size()]
	cap_label.add_theme_font_size_override("font_size", T.small)
	cap_label.add_theme_color_override("font_color", T.colors.text_muted)
	header.add_child(cap_label)

	# 弹性空间
	var h_spacer := Control.new()
	h_spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(h_spacer)

	# ── 产线列表 ──
	if lines.is_empty():
		var empty_line := Label.new()
		empty_line.text = "  暂无产线"
		empty_line.add_theme_font_size_override("font_size", T.small)
		empty_line.add_theme_color_override("font_color", T.colors.text_muted)
		card_vbox.add_child(empty_line)
	else:
		for line in lines:
			_add_line_row(card_vbox, line, factory_idx)


# ── Line Row ──

func _add_line_row(parent: VBoxContainer, line: Dictionary, factory_idx: int) -> void:
	var row := HBoxContainer.new()
	row.name = "LineRow"
	row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.custom_minimum_size = Vector2(0, ROW_HEIGHT)
	row.mouse_filter = Control.MOUSE_FILTER_STOP

	# Hover 效果
	row.mouse_entered.connect(_on_row_hover.bind(row, true))
	row.mouse_exited.connect(_on_row_hover.bind(row, false))

	parent.add_child(row)

	var line_type: String = line.get("type", "manual")
	var status: String = line.get("status", "idle")
	var product: String = line.get("product", "")
	var line_id: int = line.get("id", 0)
	var progress_val: float = clampf(line.get("progress", 0.0), 0.0, 1.0)

	# ── 行号 + 类型标签 ──
	var name_label := Label.new()
	name_label.text = "#%d %s" % [line_id, TYPE_LABELS.get(line_type, line_type)]
	name_label.custom_minimum_size = Vector2(100, ROW_HEIGHT)
	name_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	name_label.add_theme_font_size_override("font_size", T.small)
	name_label.add_theme_color_override("font_color", T.colors.text_medium)
	row.add_child(name_label)

	# ── 产品图标（带颜色的圆角方块） ──
	row.add_child(_make_product_badge(product))

	# ── 进度条（ColorRect + 动画） ──
	row.add_child(_make_progress_bar(line, factory_idx, line_id))

	# ── 状态色条 ──
	row.add_child(_make_status_pill(status, line))

	# 弹性空间
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(spacer)


# ── Product Badge ──

func _make_product_badge(product_name: String) -> Control:
	var container := Control.new()
	container.custom_minimum_size = Vector2(ICON_SIZE + 12, ROW_HEIGHT)

	if product_name.is_empty():
		var empty_rect := ColorRect.new()
		empty_rect.size = Vector2(ICON_SIZE, ICON_SIZE)
		empty_rect.position = Vector2(6, (ROW_HEIGHT - ICON_SIZE) * 0.5)
		empty_rect.color = T.colors.border
		container.add_child(empty_rect)
		return container

	var rect := ColorRect.new()
	rect.size = Vector2(ICON_SIZE, ICON_SIZE)
	rect.position = Vector2(6, (ROW_HEIGHT - ICON_SIZE) * 0.5)

	var color: Color = T.product_color(product_name)
	if color == T.colors.idle:
		color = T.colors.text_muted
	rect.color = color

	# 微圆角
	var sb := AU.card_style(color, 4)
	rect.add_theme_stylebox_override("panel", sb)

	container.add_child(rect)

	# 产品名首字叠加在色块上
	var char_label := Label.new()
	char_label.text = T.product_icon(product_name)
	char_label.position = Vector2(8, (ROW_HEIGHT - 16) * 0.5)
	char_label.size = Vector2(20, 16)
	char_label.add_theme_font_size_override("font_size", T.tiny)
	char_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	char_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	container.add_child(char_label)

	return container


# ── Progress Bar ──

func _make_progress_bar(line: Dictionary, factory_idx: int, line_id: int) -> Control:
	var container := Control.new()
	container.custom_minimum_size = Vector2(BAR_WIDTH + 10, ROW_HEIGHT)
	container.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN

	var status: String = line.get("status", "idle")
	var progress_val: float = clampf(line.get("progress", 0.0), 0.0, 1.0)
	var anim_key: String = "%d_%d" % [factory_idx, line_id]

	# 背景条
	var bg := ColorRect.new()
	bg.color = T.colors.border_light
	bg.size = Vector2(BAR_WIDTH, BAR_HEIGHT)
	bg.position = Vector2(0, (ROW_HEIGHT - BAR_HEIGHT) * 0.5)
	container.add_child(bg)

	if status == "idle":
		# idle 状态：显示空进度条，不填充
		return container

	# 填充条
	var fill := ColorRect.new()
	fill.color = _get_status_color(status)
	fill.size = Vector2(BAR_WIDTH * progress_val, BAR_HEIGHT)
	fill.position = Vector2(0, (ROW_HEIGHT - BAR_HEIGHT) * 0.5)
	container.add_child(fill)

	# 记住引用，供动画使用
	_fill_rects[anim_key] = fill

	# 百分比文字
	var pct_label := Label.new()
	pct_label.text = "%d%%" % int(progress_val * 100.0)
	pct_label.add_theme_color_override("font_color", T.colors.white)
	pct_label.add_theme_font_size_override("font_size", 10)
	pct_label.position = Vector2(6, (ROW_HEIGHT - BAR_HEIGHT) * 0.5)
	pct_label.size = Vector2(BAR_WIDTH - 12, BAR_HEIGHT)
	pct_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
	pct_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	container.add_child(pct_label)
	_pct_labels[anim_key] = pct_label

	# 动画：从旧进度平滑过渡到新进度
	_animate_bar(anim_key, progress_val)

	return container


func _animate_bar(anim_key: String, target_pct: float) -> void:
	var fill: ColorRect = _fill_rects.get(anim_key)
	if not fill:
		return

	var old_pct: float = _prev_progress.get(anim_key, 0.0)
	_prev_progress[anim_key] = target_pct

	# 如果差值很小，跳过动画直接设置
	if abs(target_pct - old_pct) < 0.01:
		fill.size.x = BAR_WIDTH * target_pct
		return

	var pct_label: Label = _pct_labels.get(anim_key)
	if pct_label:
		AU.animate_progress_with_label(fill, pct_label, target_pct, BAR_WIDTH, PROGRESS_ANIM_DURATION)
	else:
		AU.animate_progress(fill, target_pct, BAR_WIDTH, PROGRESS_ANIM_DURATION)


# ── Status Pill ──

func _make_status_pill(status: String, line: Dictionary) -> Control:
	var container := Control.new()
	container.custom_minimum_size = Vector2(100, ROW_HEIGHT)

	var status_text: String = _get_status_text(line)
	var bg_color: Color = _get_status_color(status)

	# 使用圆角色条
	var pill_bg := ColorRect.new()
	pill_bg.color = bg_color
	pill_bg.size = Vector2(90, 24)
	pill_bg.position = Vector2(5, (ROW_HEIGHT - 24) * 0.5)

	var sb := AU.pill_style(bg_color)
	# ColorRect 不直接支持 StyleBox，通过 Panel 实现
	# 这里用 ColorRect + 近似圆角效果

	container.add_child(pill_bg)

	# 文字叠加
	var label := Label.new()
	label.text = status_text
	label.position = Vector2(5, (ROW_HEIGHT - 20) * 0.5)
	label.size = Vector2(90, 24)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.add_theme_font_size_override("font_size", T.small)
	# 根据背景亮度选择文字颜色
	if bg_color.get_luminance() > 0.5:
		label.add_theme_color_override("font_color", Color.BLACK)
	else:
		label.add_theme_color_override("font_color", T.colors.white)
	container.add_child(label)

	return container


# ── Hover Effect ──

func _on_row_hover(row: HBoxContainer, hover: bool) -> void:
	if hover:
		# 高亮背景
		var sb := StyleBoxFlat.new()
		sb.bg_color = T.colors.border_light
		sb.set_corner_radius_all(4)
		row.add_theme_stylebox_override("normal", sb)
	else:
		row.remove_theme_stylebox_override("normal")


# ── Helpers ──

func _get_status_text(line: Dictionary) -> String:
	var status: String = line.get("status", "idle")
	var product: String = line.get("product", "")
	var remaining: int = line.get("remaining", 0)
	var switch_target: String = line.get("switch_target", "")

	match status:
		"idle":
			return "空闲"
		"producing":
			if not product.is_empty():
				var pname: String = ""
				match product:
					"ben_ma": pname = "奔马"
					"meng_hu": pname = "猛虎"
					"fei_ying": pname = "飞鹰"
					"tian_long": pname = "天龙"
					_: pname = product
				return "生产 %s" % pname
			return "生产中"
		"building":
			if remaining > 0:
				return "建设 Q%d" % remaining
			return "建设中"
		"switching_to":
			if not switch_target.is_empty():
				return "转产→%s" % switch_target
			return "转产中"
		_:
			return status


func _get_status_color(status: String) -> Color:
	match status:
		"idle":
			return T.colors.idle
		"producing":
			return T.colors.producing
		"building":
			return T.colors.building
		"switching_to":
			return T.colors.switching
		_:
			return T.colors.idle

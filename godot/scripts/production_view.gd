extends Control
## 生产线视窗——显示工厂和生产线状态。
##
## 通过 update(factories) 接收后端数据刷新显示。
## 使用纯 ColorRect 做进度条，无需额外贴图资源。

class_name ProductionView

const T = preload("res://scripts/theme.gd")

# ── Constants ──

# 产线类型显示文本
const TYPE_LABELS: Dictionary = {
	"manual": "手工",
	"semi_auto": "半自动",
	"auto": "全自动",
}

# 布局常量
const ROW_HEIGHT: float = 40.0
const BAR_HEIGHT: float = 18.0
const BAR_WIDTH: float = 130.0
const ICON_SIZE: float = 22.0
const SECTION_GAP: float = 4.0

# ── Private Vars ──

var _container: VBoxContainer


# ── Virtual Methods ──

func _ready() -> void:
	_container = VBoxContainer.new()
	_container.name = "Container"
	_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	add_child(_container)


# ── Public Methods ──

## 接收后端工厂数据并刷新 UI。
## factories: Array[Dictionary] 格式：
##   { name, capacity, lines: [{ id, name, type, status, product, progress, remaining?, switch_target? }] }
func update(factories: Array) -> void:
	# 清空旧内容
	for child in _container.get_children():
		_container.remove_child(child)
		child.queue_free()

	if factories.is_empty():
		var empty_label := Label.new()
		empty_label.text = "暂无工厂"
		empty_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		empty_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		empty_label.size_flags_vertical = Control.SIZE_EXPAND_FILL
		empty_label.add_theme_color_override("font_color", T.colors.text_muted)
		_container.add_child(empty_label)
		return

	for factory_idx: int in factories.size():
		var factory: Dictionary = factories[factory_idx]
		if factory_idx > 0:
			var sep := HSeparator.new()
			_container.add_child(sep)
		_add_factory_section(factory)


# ── Private: Factory Section ──

func _add_factory_section(factory: Dictionary) -> void:
	# ── Factory header ──
	var header := HBoxContainer.new()
	_container.add_child(header)

	var icon := Label.new()
	icon.text = "🏭"
	header.add_child(icon)

	var name_label := Label.new()
	name_label.text = factory.get("name", "未知工厂")
	name_label.add_theme_font_size_override("font_size", 14)
	name_label.add_theme_color_override("font_color", T.colors.text)
	header.add_child(name_label)

	var cap_label := Label.new()
	var capacity: int = factory.get("capacity", 0)
	cap_label.text = " (容量:%d)" % capacity
	cap_label.add_theme_color_override("font_color", T.colors.text_muted)
	header.add_child(cap_label)

	# 弹性空间
	var h_spacer := Control.new()
	h_spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(h_spacer)

	# ── Production lines ──
	var lines: Array = factory.get("lines", [])
	for line in lines:
		_add_line_row(line)


func _add_line_row(line: Dictionary) -> void:
	var row := HBoxContainer.new()
	row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_container.add_child(row)

	var line_type: String = line.get("type", "manual")
	var status: String = line.get("status", "idle")
	var product: String = line.get("product", "")
	var line_id: int = line.get("id", 0)

	# ── 行号 + 类型标签 ──
	var name_label := Label.new()
	name_label.text = "#%d %s" % [line_id, TYPE_LABELS.get(line_type, line_type)]
	name_label.custom_minimum_size = Vector2(100, ROW_HEIGHT)
	name_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	name_label.add_theme_font_size_override("font_size", 12)
	row.add_child(name_label)

	# ── 进度条（ColorRect 实现） ──
	row.add_child(_make_progress_bar(line))

	# ── 产品图标 ──
	row.add_child(_make_product_icon(product))

	# ── 状态文字 ──
	var status_label := Label.new()
	status_label.text = _get_status_text(line)
	status_label.add_theme_color_override("font_color", _get_status_color(status))
	status_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	status_label.custom_minimum_size = Vector2(110, ROW_HEIGHT)
	status_label.add_theme_font_size_override("font_size", 12)
	row.add_child(status_label)

	# 弹性空间
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(spacer)


# ── Progress Bar ──

func _make_progress_bar(line: Dictionary) -> Control:
	var container := Control.new()
	container.custom_minimum_size = Vector2(BAR_WIDTH, ROW_HEIGHT)
	container.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN

	var status: String = line.get("status", "idle")

	# 背景条
	var bg := ColorRect.new()
	bg.color = T.colors.border
	bg.size = Vector2(BAR_WIDTH, BAR_HEIGHT)
	bg.position = Vector2(0, (ROW_HEIGHT - BAR_HEIGHT) * 0.5)
	container.add_child(bg)

	if status == "idle":
		# idle 状态不显示填充条
		pass
	else:
		# 填充条
		var progress_val: float = clampf(line.get("progress", 0.0), 0.0, 1.0)
		var fill := ColorRect.new()
		fill.color = _get_status_color(status)
		fill.size = Vector2(BAR_WIDTH * progress_val, BAR_HEIGHT)
		fill.position = Vector2(0, (ROW_HEIGHT - BAR_HEIGHT) * 0.5)
		container.add_child(fill)

		# 百分比文字
		var pct_label := Label.new()
		pct_label.text = "%d%%" % (progress_val * 100.0)
		pct_label.add_theme_color_override("font_color", T.colors.white)
		pct_label.add_theme_font_size_override("font_size", 10)
		pct_label.position = Vector2(4, (ROW_HEIGHT - BAR_HEIGHT) * 0.5)
		pct_label.size = Vector2(BAR_WIDTH - 8, BAR_HEIGHT)
		container.add_child(pct_label)

	return container


# ── Product Icon ──

func _make_product_icon(product_name: String) -> Control:
	var container := Control.new()
	container.custom_minimum_size = Vector2(ICON_SIZE + 8, ROW_HEIGHT)

	if product_name.is_empty():
		return container

	var rect := ColorRect.new()
	rect.size = Vector2(ICON_SIZE, ICON_SIZE)
	rect.position = Vector2(4, (ROW_HEIGHT - ICON_SIZE) * 0.5)

	var color: Color = T.product_color(product_name)
	if color == T.colors.idle:
		color = T.colors.text_muted
	rect.color = color
	container.add_child(rect)

	return container


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
				return "生产 %s" % product
			return "生产中"
		"building":
			if remaining > 0:
				return "建设中(Q%d)" % remaining
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

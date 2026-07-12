extends Control
## 库存与财务看板——显示原料、在制品、成品、应收款等核心数据。
##
## 通过 update(state) 接收完整 game_state 刷新显示。

class_name InventoryDashboard

const T = preload("res://scripts/theme.gd")

# ── Constants ──

# 颜色（引用 Theme 常量）
var COLOR_HEADER: Color = T.colors.text_strong
var COLOR_LABEL: Color = T.colors.text_muted
var COLOR_VALUE: Color = T.colors.text_strong
var COLOR_SECTION: Color = T.colors.brand
var COLOR_AR: Color = T.colors.warning
var COLOR_PROGRESS_BG: Color = T.colors.border
var COLOR_PROGRESS_FILL: Color = T.colors.success
var COLOR_SEPARATOR: Color = T.colors.border

# 产品颜色（引用 Theme 常量）
var PRODUCT_COLORS: Dictionary = {
	"奔马": T.colors.benma,
	"猛虎": T.colors.menghu,
	"雄鹰": T.colors.feiying,
	"飞龙": T.colors.tianlong,
}

# 布局常量
const ROW_HEIGHT: float = 22.0
const SECTION_INDENT: float = 8.0
const BAR_WIDTH: float = 80.0
const BAR_HEIGHT: float = 12.0

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

## 接收完整 game_state 并刷新看板。
func update(state: Dictionary) -> void:
	# 清空旧内容
	for child in _container.get_children():
		_container.remove_child(child)
		child.queue_free()

	_build_header()
	_build_separator()
	_build_raw_materials(state)
	_build_separator()
	_build_work_in_progress(state)
	_build_separator()
	_build_finished_goods(state)
	_build_separator()
	_build_accounts_receivable(state)


# ── Private: Section Builders ──

func _build_header() -> void:
	var label := Label.new()
	label.text = "📦 库存与财务"
	label.add_theme_font_size_override("font_size", 14)
	label.add_theme_color_override("font_color", COLOR_HEADER)
	label.custom_minimum_size = Vector2(0, ROW_HEIGHT + 4)
	_container.add_child(label)


func _build_separator() -> void:
	var sep := HSeparator.new()
	sep.modulate = COLOR_SEPARATOR
	_container.add_child(sep)


func _build_raw_materials(state: Dictionary) -> void:
	_add_section_label("原料库存")

	var raw: Variant = state.get("raw_material_inventory", {})
	var raw_units: int = 0

	if raw is Dictionary:
		raw_units = raw.get("quantity", raw.get("units", 0))
	elif raw is int:
		raw_units = raw
	elif raw is float:
		raw_units = int(raw)

	if raw_units > 0:
		_add_value_row("原料库存: %d 个单位" % raw_units)
	else:
		_add_empty_row()


func _build_work_in_progress(state: Dictionary) -> void:
	_add_section_label("在制品")

	var wip: Array = state.get("work_in_progress", [])
	if wip.is_empty():
		_add_empty_row()
		return

	for item in wip:
		_add_wip_row(item)


func _build_finished_goods(state: Dictionary) -> void:
	_add_section_label("成品库存")

	var fg: Variant = state.get("finished_goods", {})
	var items: Dictionary = _normalize_finished_goods(fg)

	if items.is_empty():
		_add_empty_row()
		return

	# 按产品分组的行
	var hb := HBoxContainer.new()
	hb.custom_minimum_size = Vector2(0, ROW_HEIGHT + 4)
	_container.add_child(hb)

	var indent := Control.new()
	indent.custom_minimum_size = Vector2(SECTION_INDENT * 2, 0)
	hb.add_child(indent)

	for product_name: String in items:
		var qty: int = items[product_name]
		var color: Color = PRODUCT_COLORS.get(product_name, T.colors.text_muted)

		var plabel := Label.new()
		plabel.text = "%s x%d" % [product_name, qty]
		plabel.add_theme_color_override("font_color", color)
		plabel.add_theme_font_size_override("font_size", 11)
		plabel.custom_minimum_size = Vector2(70, ROW_HEIGHT)
		hb.add_child(plabel)


func _build_accounts_receivable(state: Dictionary) -> void:
	_add_section_label("应收账款")

	var ar: Array = state.get("accounts_receivable", [])
	if ar.is_empty():
		_add_empty_row()
		return

	for item in ar:
		_add_ar_row(item)


# ── Private: Row Helpers ──

func _add_section_label(text: String) -> void:
	var label := Label.new()
	label.text = "  %s" % text
	label.add_theme_font_size_override("font_size", 12)
	label.add_theme_color_override("font_color", COLOR_SECTION)
	label.custom_minimum_size = Vector2(0, ROW_HEIGHT)
	_container.add_child(label)


func _add_empty_row() -> void:
	var hb := HBoxContainer.new()
	hb.custom_minimum_size = Vector2(0, ROW_HEIGHT)
	_container.add_child(hb)

	var indent := Control.new()
	indent.custom_minimum_size = Vector2(SECTION_INDENT * 2, 0)
	hb.add_child(indent)

	var label := Label.new()
	label.text = "  (无)"
	label.add_theme_color_override("font_color", COLOR_LABEL)
	label.add_theme_font_size_override("font_size", 11)
	hb.add_child(label)


func _add_value_row(text: String) -> void:
	var hb := HBoxContainer.new()
	hb.custom_minimum_size = Vector2(0, ROW_HEIGHT)
	_container.add_child(hb)

	var indent := Control.new()
	indent.custom_minimum_size = Vector2(SECTION_INDENT * 2, 0)
	hb.add_child(indent)

	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", 11)
	label.add_theme_color_override("font_color", COLOR_VALUE)
	hb.add_child(label)


func _add_wip_row(item: Dictionary) -> void:
	var hb := HBoxContainer.new()
	hb.custom_minimum_size = Vector2(0, ROW_HEIGHT + 4)
	_container.add_child(hb)

	var indent := Control.new()
	indent.custom_minimum_size = Vector2(SECTION_INDENT * 2, 0)
	hb.add_child(indent)

	var line_id: int = item.get("line_id", item.get("id", 0))
	var product: String = item.get("product", "")
	var progress: float = clampf(item.get("progress", 0.0), 0.0, 1.0)

	# 产线标签: "#N 产品名"
	var line_label := Label.new()
	line_label.text = "#%d %s" % [line_id, product]
	line_label.custom_minimum_size = Vector2(80, ROW_HEIGHT)
	line_label.add_theme_font_size_override("font_size", 11)
	line_label.add_theme_color_override("font_color", COLOR_LABEL)
	hb.add_child(line_label)

	# 进度条
	hb.add_child(_make_progress_bar(progress))

	# 百分比
	var pct_label := Label.new()
	pct_label.text = "%d%%" % (progress * 100.0)
	pct_label.add_theme_font_size_override("font_size", 10)
	pct_label.add_theme_color_override("font_color", COLOR_VALUE)
	pct_label.custom_minimum_size = Vector2(30, ROW_HEIGHT)
	hb.add_child(pct_label)

	# 弹性空间
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hb.add_child(spacer)


func _make_progress_bar(progress: float) -> Control:
	var c := Control.new()
	c.custom_minimum_size = Vector2(BAR_WIDTH, ROW_HEIGHT)

	var bg := ColorRect.new()
	bg.color = COLOR_PROGRESS_BG
	bg.size = Vector2(BAR_WIDTH, BAR_HEIGHT)
	bg.position = Vector2(0, (ROW_HEIGHT - BAR_HEIGHT) * 0.5)
	c.add_child(bg)

	if progress > 0.0:
		var fill := ColorRect.new()
		fill.color = COLOR_PROGRESS_FILL
		fill.size = Vector2(BAR_WIDTH * progress, BAR_HEIGHT)
		fill.position = Vector2(0, (ROW_HEIGHT - BAR_HEIGHT) * 0.5)
		c.add_child(fill)

	return c


func _normalize_finished_goods(fg: Variant) -> Dictionary:
	var items: Dictionary = {}

	if fg is Dictionary:
		items = fg.duplicate()
	elif fg is Array:
		for entry in fg:
			if entry is Dictionary:
				var name: String = entry.get("product", entry.get("name", ""))
				var qty: int = entry.get("quantity", entry.get("qty", 1))
				items[name] = items.get(name, 0) + qty

	return items


func _add_ar_row(item: Dictionary) -> void:
	var hb := HBoxContainer.new()
	hb.custom_minimum_size = Vector2(0, ROW_HEIGHT)
	_container.add_child(hb)

	var indent := Control.new()
	indent.custom_minimum_size = Vector2(SECTION_INDENT * 2, 0)
	hb.add_child(indent)

	var amount: int = item.get("amount", 0)
	var remaining: int = item.get("remaining", item.get("remaining_quarters", 0))

	var ar_label := Label.new()
	ar_label.text = "%dM (剩余 %d季度)" % [amount, remaining]
	ar_label.add_theme_color_override("font_color", COLOR_AR)
	ar_label.add_theme_font_size_override("font_size", 11)
	hb.add_child(ar_label)

	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hb.add_child(spacer)

extends Panel
## 年度财务报告弹窗——全屏半透明遮罩上的多标签页财务报告。
##
## 通过 show_report(report) 传入年度报告数据并在四个标签页中展示：
## - 📊 损益表：收入、成本、利润等
## - 📋 资产负债表：资产、负债、权益
## - 💰 费用明细：各项费用支出
## - 🎯 关键指标：毛利率、净利润率、资产负债率、流动比率

class_name AnnualReportPopup

const T = preload("res://scripts/theme.gd")

# ── Colors ──

var OVERLAY_COLOR: Color = Color(0.0, 0.0, 0.0, 0.65)
var PANEL_COLOR: Color = T.colors.card
var COLOR_TITLE: Color = T.colors.text_strong
var COLOR_SECTION: Color = T.colors.brand
var COLOR_LABEL: Color = T.colors.text_muted
var COLOR_VALUE: Color = T.colors.text_strong
var COLOR_TOTAL: Color = T.colors.success
var COLOR_NEGATIVE: Color = T.colors.danger
var COLOR_SEPARATOR: Color = T.colors.border
var COLOR_CLOSE_BG: Color = T.colors.danger
var COLOR_CLOSE_HOVER: Color = T.colors.danger_bg
var COLOR_BG_ALT: Color = T.colors.border_light
var COLOR_TAB_BG: Color = T.colors.border
var COLOR_TAB_FG: Color = T.colors.card
var COLOR_INDICATOR_LABEL: Color = T.colors.text
var COLOR_INDICATOR_VAL: Color = T.colors.success

# ── Layout Constants ──

const PANEL_WIDTH: float = 720.0
const PANEL_HEIGHT: float = 560.0
const ROW_HEIGHT: float = 28.0
const LABEL_WIDTH: float = 200.0
const VALUE_WIDTH: float = 120.0
const FONT_SIZE_TITLE: int = 20
const FONT_SIZE_TAB: int = 13
const FONT_SIZE_ROW: int = 12
const FONT_SIZE_TOTAL: int = 14
const INDENT_WIDTH: float = 10.0

# ── Tab Titles ──

const TAB_TITLES: PackedStringArray = [
	"📊 损益表",
	"📋 资产负债表",
	"💰 费用明细",
	"🎯 关键指标",
]

# ── Tab 1: 损益表 rows ──

const INCOME_ROWS: Array[Dictionary] = [
	{key = "revenue", label = "营业收入", is_total = false},
	{key = "cost_of_goods_sold", label = "营业成本", is_total = false},
	{key = "gross_profit", label = "毛利润", is_total = true},
	{separator = true},
	{key = "sales_expense", label = "销售费用", is_total = false},
	{key = "admin_expense", label = "管理费用", is_total = false},
	{key = "rd_expense", label = "研发费用", is_total = false},
	{key = "depreciation", label = "折旧费用", is_total = false},
	{key = "operating_profit", label = "营业利润", is_total = true},
	{separator = true},
	{key = "interest_expense", label = "利息支出", is_total = false},
	{key = "discount_fee", label = "贴现费用", is_total = false},
	{key = "tax", label = "所得税", is_total = false},
	{separator = true},
	{key = "net_profit", label = "净利润", is_total = true},
]

# ── Tab 2: 资产负债表 rows ──

const BALANCE_ROWS: Array[Dictionary] = [
	{section = "🌊 流动资产"},
	{key = "cash", label = "现金", is_total = false},
	{key = "accounts_receivable", label = "应收账款", is_total = false},
	{key = "raw_material", label = "原材料", is_total = false},
	{key = "work_in_process", label = "在制品", is_total = false},
	{key = "finished_goods", label = "成品", is_total = false},
	{key = "total_current_assets", label = "流动资产合计", is_total = true},
	{separator = true},
	{section = "🏭 固定资产"},
	{key = "factory", label = "厂房", is_total = false},
	{key = "production_lines", label = "生产线", is_total = false},
	{key = "total_fixed_assets", label = "固定资产合计", is_total = true},
	{separator = true},
	{key = "total_assets", label = "资产总计", is_total = true, is_major = true},
	{separator = true},
	{section = "📋 负债"},
	{key = "long_term_loans", label = "长期借款", is_total = false},
	{key = "short_term_loans", label = "短期借款", is_total = false},
	{key = "total_liabilities", label = "负债合计", is_total = true},
	{separator = true},
	{section = "💎 所有者权益"},
	{key = "equity", label = "所有者权益", is_total = true},
]

# ── Tab 3: 费用明细 rows ──

const EXPENSE_ROWS: Array[Dictionary] = [
	{key = "new_market_investment", label = "新市场开拓投资"},
	{key = "product_rd_investment", label = "产品研发投资"},
	{key = "quarterly_admin_fees", label = "行政管理费"},
	{key = "factory_rent", label = "厂房租金"},
	{key = "line_maintenance", label = "设备维护费"},
	{key = "product_switch_fees", label = "转产费用"},
	{key = "sales_expenses", label = "销售费用"},
	{key = "depreciation", label = "折旧"},
	{key = "interest_and_discount", label = "利息与贴现"},
	{key = "tax", label = "税费"},
]

# ── Nodes (created in _build_ui) ──

var _overlay: ColorRect
var _dialog: Panel
var _title_label: Label
var _tab_container: TabContainer
var _close_btn: Button

# Holds the VBoxContainer for each tab (index 0-3)
var _tab_contents: Array[VBoxContainer] = []

var _report: Dictionary = {}


# ── Virtual Methods ──

func _ready() -> void:
	_build_ui()
	visible = false


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		# Re-center dialog when parent resizes
		if _dialog and _overlay:
			_center_dialog()


# ── Public Methods ──

## 接收年度报告数据并刷新所有标签页内容，然后显示弹窗。
## report 结构：{ income_statement: {}, balance_sheet: {}, expense_sheet: {}, year: int }
func show_report(report: Dictionary) -> void:
	_report = report
	var year: int = report.get("year", 0)
	_title_label.text = "📊 年度财务报告 — 第 %d 年" % year

	_populate_tabs()
	visible = true
	_center_dialog()


# ── UI Building ──

func _build_ui() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE

	# ── Full-screen overlay (click to close) ──
	_overlay = ColorRect.new()
	_overlay.name = "Overlay"
	_overlay.color = OVERLAY_COLOR
	_overlay.mouse_filter = Control.MOUSE_FILTER_STOP
	_overlay.connect("gui_input", _on_overlay_gui_input)
	add_child(_overlay)
	# Anchor overlay to full parent
	_overlay.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)

	# ── Dialog panel ──
	_dialog = Panel.new()
	_dialog.name = "Dialog"
	_dialog.custom_minimum_size = Vector2(PANEL_WIDTH, PANEL_HEIGHT)
	add_child(_dialog)

	# ── Title ──
	_title_label = Label.new()
	_title_label.name = "TitleLabel"
	_title_label.text = "📊 年度财务报告"
	_title_label.add_theme_font_size_override("font_size", FONT_SIZE_TITLE)
	_title_label.add_theme_color_override("font_color", COLOR_TITLE)
	_title_label.position = Vector2(20, 16)
	_title_label.size = Vector2(PANEL_WIDTH - 80, 32)
	_dialog.add_child(_title_label)

	# ── Close button ──
	_close_btn = Button.new()
	_close_btn.name = "CloseBtn"
	_close_btn.text = "✕ 关闭"
	_close_btn.position = Vector2(PANEL_WIDTH - 100, 16)
	_close_btn.size = Vector2(80, 30)
	_close_btn.add_theme_color_override("font_color", Color(1.0, 0.8, 0.8))
	_close_btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_CLOSE_BG))
	_close_btn.add_theme_stylebox_override("hover", _make_stylebox(COLOR_CLOSE_HOVER))
	_close_btn.connect("pressed", _on_close_pressed)
	_dialog.add_child(_close_btn)

	# ── TabContainer ──
	_tab_container = TabContainer.new()
	_tab_container.name = "TabContainer"
	_tab_container.position = Vector2(10, 56)
	_tab_container.size = Vector2(PANEL_WIDTH - 20, PANEL_HEIGHT - 72)
	_tab_container.add_theme_color_override("font_color", T.colors.text)
	_tab_container.add_theme_color_override("font_selected_color", T.colors.text_strong)
	_tab_container.add_theme_stylebox_override("panel", _make_stylebox(COLOR_TAB_BG))
	_tab_container.add_theme_stylebox_override("tab_fg", _make_stylebox(COLOR_TAB_FG))
	_tab_container.tab_alignment = TabBar.ALIGNMENT_LEFT
	_dialog.add_child(_tab_container)

	# Build 4 tabs
	for i in range(4):
		var scroll := ScrollContainer.new()
		scroll.name = "Tab%d" % i
		scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
		_tab_container.add_child(scroll)

		var vbox := VBoxContainer.new()
		vbox.name = "Content"
		vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		vbox.add_theme_constant_override("separation", 0)
		scroll.add_child(vbox)

		_tab_contents.append(vbox)
		_tab_container.set_tab_title(i, TAB_TITLES[i])


# ── Tab Population ──

func _populate_tabs() -> void:
	_populate_income_tab()
	_populate_balance_tab()
	_populate_expense_tab()
	_populate_indicators_tab()


func _populate_income_tab() -> void:
	var container: VBoxContainer = _tab_contents[0]
	_clear_container(container)

	var data: Dictionary = _report.get("income_statement", {})
	for row_info: Dictionary in INCOME_ROWS:
		if row_info.has("separator") and row_info.separator:
			container.add_child(_make_separator())
		elif row_info.has("key"):
			var key: String = row_info.key
			var label: String = row_info.label
			var is_total: bool = row_info.get("is_total", false)
			var is_major: bool = row_info.get("is_major", false)
			var val: Variant = data.get(key, 0)
			container.add_child(_make_data_row(label, val, is_total, is_major))


func _populate_balance_tab() -> void:
	var container: VBoxContainer = _tab_contents[1]
	_clear_container(container)

	var data: Dictionary = _report.get("balance_sheet", {})
	for row_info: Dictionary in BALANCE_ROWS:
		if row_info.has("separator") and row_info.separator:
			container.add_child(_make_separator())
		elif row_info.has("section"):
			container.add_child(_make_section_label(row_info.section))
		elif row_info.has("key"):
			var key: String = row_info.key
			var label: String = row_info.label
			var is_total: bool = row_info.get("is_total", false)
			var is_major: bool = row_info.get("is_major", false)
			var val: Variant = data.get(key, 0)
			container.add_child(_make_data_row(label, val, is_total, is_major))


func _populate_expense_tab() -> void:
	var container: VBoxContainer = _tab_contents[2]
	_clear_container(container)

	var data: Dictionary = _report.get("expense_sheet", {})
	for row_info: Dictionary in EXPENSE_ROWS:
		if row_info.has("key"):
			var key: String = row_info.key
			var label: String = row_info.label
			var val: Variant = data.get(key, 0)
			container.add_child(_make_data_row(label, val, false, false))


func _populate_indicators_tab() -> void:
	var container: VBoxContainer = _tab_contents[3]
	_clear_container(container)

	var income: Dictionary = _report.get("income_statement", {})
	var balance: Dictionary = _report.get("balance_sheet", {})

	var revenue: float = float(income.get("revenue", 0))
	var gross_profit: float = float(income.get("gross_profit", 0))
	var net_profit: float = float(income.get("net_profit", 0))
	var total_assets: float = float(balance.get("total_assets", 0))
	var total_liabilities: float = float(balance.get("total_liabilities", 0))
	var current_assets: float = float(balance.get("total_current_assets", 0))
	var short_loans: float = float(balance.get("short_term_loans", 0))

	# Section header
	container.add_child(_make_section_label("📈 盈利能力指标"))

	# 毛利率
	var gross_margin_pct: float = (gross_profit / revenue * 100.0) if revenue > 0 else 0.0
	container.add_child(_make_indicator_row(
		"毛利率",
		"%s%%" % _fmt_pct(gross_margin_pct),
		gross_margin_pct
	))

	# 净利润率
	var net_margin_pct: float = (net_profit / revenue * 100.0) if revenue > 0 else 0.0
	container.add_child(_make_indicator_row(
		"净利润率",
		"%s%%" % _fmt_pct(net_margin_pct),
		net_margin_pct
	))

	container.add_child(_make_separator())
	container.add_child(_make_section_label("⚖️ 偿债能力指标"))

	# 资产负债率
	var debt_ratio_pct: float = (total_liabilities / total_assets * 100.0) if total_assets > 0 else 0.0
	container.add_child(_make_indicator_row(
		"资产负债率",
		"%s%%" % _fmt_pct(debt_ratio_pct),
		debt_ratio_pct,
		true
	))

	# 流动比率
	var current_ratio: float = (current_assets / short_loans) if short_loans > 0 else 0.0
	var cr_text: String = "%.2f" % current_ratio if short_loans > 0 else "N/A (无短期借款)"
	container.add_child(_make_indicator_row(
		"流动比率",
		cr_text,
		current_ratio,
		false,
		short_loans <= 0
	))

	# Explanation note
	container.add_child(_make_separator())
	var note := Label.new()
	note.text = "  提示：资产负债率 < 50% 为健康，流动比率 > 1.5 为佳。"
	note.add_theme_font_size_override("font_size", 11)
	note.add_theme_color_override("font_color", T.colors.text_muted)
	note.custom_minimum_size = Vector2(0, ROW_HEIGHT)
	container.add_child(note)


# ── Row Builders ──

func _make_data_row(label_text: String, value: Variant, is_total: bool, is_major: bool) -> HBoxContainer:
	var hb := HBoxContainer.new()
	hb.custom_minimum_size = Vector2(0, ROW_HEIGHT + (4 if is_major else 0))
	hb.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# Indent label
	var indent := Control.new()
	indent.custom_minimum_size = Vector2(INDENT_WIDTH, 0)
	hb.add_child(indent)

	# Label
	var label := Label.new()
	label.text = label_text
	label.custom_minimum_size = Vector2(LABEL_WIDTH - INDENT_WIDTH, 0)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER

	if is_total:
		label.add_theme_font_size_override("font_size", FONT_SIZE_TOTAL)
		label.add_theme_color_override("font_color", COLOR_TOTAL)
		label.add_theme_font_weight_override("bold", true)
	else:
		label.add_theme_font_size_override("font_size", FONT_SIZE_ROW)
		label.add_theme_color_override("font_color", COLOR_LABEL)

	if is_major:
		label.add_theme_font_size_override("font_size", FONT_SIZE_TOTAL + 1)
		label.add_theme_color_override("font_color", T.colors.text_strong)

	hb.add_child(label)

	# Spacer
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hb.add_child(spacer)

	# Value
	var val_label := Label.new()
	val_label.text = _fmt_money(value)
	val_label.custom_minimum_size = Vector2(VALUE_WIDTH, 0)
	val_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	val_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	val_label.size_flags_horizontal = Control.SIZE_SHRINK_END

	var num_val: float = float(value) if value != null else 0.0
	if is_total or is_major:
		val_label.add_theme_font_size_override("font_size", FONT_SIZE_TOTAL)
		val_label.add_theme_font_weight_override("bold", true)
		if num_val >= 0.0:
			val_label.add_theme_color_override("font_color", COLOR_TOTAL)
		else:
			val_label.add_theme_color_override("font_color", COLOR_NEGATIVE)
	else:
		val_label.add_theme_font_size_override("font_size", FONT_SIZE_ROW)
		if num_val >= 0.0:
			val_label.add_theme_color_override("font_color", COLOR_VALUE)
		else:
			val_label.add_theme_color_override("font_color", COLOR_NEGATIVE)

	hb.add_child(val_label)

	return hb


func _make_indicator_row(label_text: String, value_text: String, raw_val: float, is_debt_ratio: bool = false, use_na: bool = false) -> HBoxContainer:
	var hb := HBoxContainer.new()
	hb.custom_minimum_size = Vector2(0, ROW_HEIGHT + 4)
	hb.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var indent := Control.new()
	indent.custom_minimum_size = Vector2(INDENT_WIDTH * 2, 0)
	hb.add_child(indent)

	# Label
	var label := Label.new()
	label.text = label_text
	label.custom_minimum_size = Vector2(LABEL_WIDTH - INDENT_WIDTH * 2, 0)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.add_theme_font_size_override("font_size", FONT_SIZE_TOTAL)
	label.add_theme_color_override("font_color", COLOR_INDICATOR_LABEL)
	hb.add_child(label)

	# Spacer
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hb.add_child(spacer)

	# Value
	var val_label := Label.new()
	val_label.text = value_text
	val_label.custom_minimum_size = Vector2(VALUE_WIDTH + 40, 0)
	val_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	val_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	val_label.add_theme_font_size_override("font_size", FONT_SIZE_TOTAL + 2)
	val_label.add_theme_font_weight_override("bold", true)
	val_label.add_theme_color_override("font_color", COLOR_INDICATOR_VAL)

	if not use_na:
		if is_debt_ratio:
			# Color-code debt ratio: green < 50%, yellow 50-70%, red > 70%
			if raw_val < 50.0:
				val_label.add_theme_color_override("font_color", COLOR_TOTAL)
			elif raw_val < 70.0:
				val_label.add_theme_color_override("font_color", T.colors.warning)
			else:
				val_label.add_theme_color_override("font_color", COLOR_NEGATIVE)
		else:
			# Color-code profitability: green for positive
			if raw_val >= 0.0:
				val_label.add_theme_color_override("font_color", COLOR_TOTAL)
			else:
				val_label.add_theme_color_override("font_color", COLOR_NEGATIVE)

	hb.add_child(val_label)

	return hb


func _make_section_label(text: String) -> Label:
	var label := Label.new()
	label.text = "  %s" % text
	label.custom_minimum_size = Vector2(0, ROW_HEIGHT + 4)
	label.add_theme_font_size_override("font_size", FONT_SIZE_ROW + 1)
	label.add_theme_color_override("font_color", COLOR_SECTION)
	label.add_theme_font_weight_override("bold", true)
	return label


func _make_separator() -> HSeparator:
	var sep := HSeparator.new()
	sep.modulate = COLOR_SEPARATOR
	sep.custom_minimum_size = Vector2(0, 4)
	return sep


# ── Helpers ──

static func _make_stylebox(bg_color: Color) -> StyleBoxFlat:
	var sb := StyleBoxFlat.new()
	sb.bg_color = bg_color
	sb.corner_radius_top_left = 4
	sb.corner_radius_top_right = 4
	sb.corner_radius_bottom_left = 4
	sb.corner_radius_bottom_right = 4
	return sb


static func _fmt_money(value: Variant) -> String:
	var v: float = float(value) if value != null else 0.0
	if v == int(v):
		return "%dM" % int(v)
	return "%.1fM" % v


static func _fmt_pct(value: float) -> String:
	if value == int(value):
		return "%d" % int(value)
	return "%.1f" % value


static func _clear_container(container: VBoxContainer) -> void:
	for child in container.get_children():
		container.remove_child(child)
		child.queue_free()


func _center_dialog() -> void:
	if not _dialog:
		return
	var parent_size: Vector2 = size if size != Vector2.ZERO else Vector2(1280, 720)
	_dialog.position = Vector2(
		(parent_size.x - PANEL_WIDTH) * 0.5,
		(parent_size.y - PANEL_HEIGHT) * 0.5
	)


# ── Handlers ──

func _on_close_pressed() -> void:
	visible = false


func _on_overlay_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
		visible = false

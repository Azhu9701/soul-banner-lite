extends Panel
## 工厂与融资弹窗——管理工厂、产线、贷款和贴现。
##
## 展示当前工厂信息、新建产线选项、贷款选择和贴现确认。
## 通过 show(state) 传入游戏状态数据刷新 UI。
##
## 信号：
## - factory_ordered(line_type): 用户确认订购新产线
## - loan_requested(loan_type, amount): 用户确认贷款
## - discount_confirmed(amount): 用户确认贴现

class_name FactoryDialog

# ── Colors ──

const OVERLAY_COLOR: Color = Color(0.0, 0.0, 0.0, 0.65)
const PANEL_BG: Color = Color(0.12, 0.12, 0.15)
const COLOR_TITLE: Color = Color(1.0, 1.0, 1.0)
const COLOR_SECTION: Color = Color(0.55, 0.80, 0.95)
const COLOR_LABEL: Color = Color(0.70, 0.70, 0.75)
const COLOR_VALUE: Color = Color(0.95, 0.85, 0.30)
const COLOR_ACCENT: Color = Color(0.30, 0.90, 0.40)
const COLOR_WARN: Color = Color(0.95, 0.70, 0.20)
const COLOR_SEPARATOR: Color = Color(0.30, 0.30, 0.30)
const COLOR_BTN_BG: Color = Color(0.18, 0.55, 0.28)
const COLOR_BTN_HOVER: Color = Color(0.22, 0.65, 0.32)
const COLOR_BTN_DISABLED: Color = Color(0.15, 0.15, 0.18)
const COLOR_CLOSE_BG: Color = Color(0.35, 0.20, 0.20)
const COLOR_CLOSE_HOVER: Color = Color(0.50, 0.25, 0.25)
const COLOR_LINE_BTN: Color = Color(0.18, 0.30, 0.45)
const COLOR_LINE_BTN_HOVER: Color = Color(0.22, 0.35, 0.55)
const COLOR_LINE_BTN_SELECTED: Color = Color(0.25, 0.50, 0.65)
const COLOR_LOAN_BTN: Color = Color(0.30, 0.20, 0.15)
const COLOR_LOAN_BTN_HOVER: Color = Color(0.40, 0.25, 0.20)
const COLOR_LOAN_BTN_SELECTED: Color = Color(0.50, 0.30, 0.20)

# ── Layout Constants ──

const PANEL_WIDTH: float = 420.0
const PANEL_HEIGHT: float = 540.0
const FONT_SIZE_TITLE: int = 18
const FONT_SIZE_SECTION: int = 14
const FONT_SIZE_BODY: int = 12
const FONT_SIZE_SMALL: int = 11
const ROW_HEIGHT: float = 26.0
const SECTION_HEADER_HEIGHT: float = 24.0
const SEPARATOR_HEIGHT: float = 8.0

# ── Production Line Data ──

const LINE_TYPES: Dictionary = {
	"manual": {"label": "手工线", "cost": 4, "time": 2, "desc": "2Q 建成"},
	"semi_auto": {"label": "半自动", "cost": 8, "time": 3, "desc": "3Q 建成"},
	"auto": {"label": "全自动", "cost": 16, "time": 4, "desc": "4Q 建成"},
}

# ── Nodes ──

var _overlay: ColorRect
var _dialog: Panel
var _title_label: Label
var _close_btn: Button

# Factory info nodes
var _factory_info_label: Label
var _factory_used_label: Label

# New line nodes
var _selected_line_type: String = "manual"
var _line_type_btns: Dictionary = {}  # type -> Button
var _line_desc_label: Label

# Loan nodes
var _selected_loan_type: String = "short"
var _loan_type_btns: Dictionary = {}  # type -> Button
var _loan_amount_input: LineEdit
var _loan_rate_label: Label

# Discount nodes
var _ar_amount_label: Label
var _ar_fee_label: Label
var _discount_confirm_btn: Button

# State
var _game_state: Dictionary = {}

# ── Signals ──

signal factory_ordered(line_type: String)
signal loan_requested(loan_type: String, amount: int)
signal discount_confirmed(amount: int)

# ── Virtual Methods ──

func _ready() -> void:
	_build_ui()
	visible = false


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		if _dialog and _overlay:
			_center_dialog()


# ── Public Methods ──

## 接收游戏状态数据并刷新弹窗内容。
func show_dialog(state: Dictionary) -> void:
	_game_state = state
	_refresh_factory_info()
	_refresh_discount_info()
	visible = true
	_center_dialog()


# ── UI Building ──

func _build_ui() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE

	# ── Full-screen overlay ──
	_overlay = ColorRect.new()
	_overlay.name = "Overlay"
	_overlay.color = OVERLAY_COLOR
	_overlay.mouse_filter = Control.MOUSE_FILTER_STOP
	_overlay.connect("gui_input", _on_overlay_gui_input)
	add_child(_overlay)
	_overlay.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)

	# ── Dialog panel ──
	_dialog = Panel.new()
	_dialog.name = "Dialog"
	_dialog.custom_minimum_size = Vector2(PANEL_WIDTH, PANEL_HEIGHT)
	_dialog.add_theme_stylebox_override("panel", _make_stylebox(PANEL_BG))
	add_child(_dialog)

	var y_pos: float = 0.0

	# ── Title bar ──
	_title_label = Label.new()
	_title_label.text = "🏗️ 工厂与融资"
	_title_label.add_theme_font_size_override("font_size", FONT_SIZE_TITLE)
	_title_label.add_theme_color_override("font_color", COLOR_TITLE)
	_title_label.position = Vector2(16, 12)
	_title_label.size = Vector2(PANEL_WIDTH - 100, 32)
	_dialog.add_child(_title_label)

	_close_btn = Button.new()
	_close_btn.text = "✕ 关闭"
	_close_btn.position = Vector2(PANEL_WIDTH - 90, 12)
	_close_btn.size = Vector2(76, 28)
	_close_btn.add_theme_color_override("font_color", Color(1.0, 0.8, 0.8))
	_close_btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_CLOSE_BG))
	_close_btn.add_theme_stylebox_override("hover", _make_stylebox(COLOR_CLOSE_HOVER))
	_close_btn.connect("pressed", _on_close_pressed)
	_dialog.add_child(_close_btn)

	y_pos = 48.0

	# ════════════════════════════════════════════
	# Section: 工厂信息
	# ════════════════════════════════════════════
	y_pos = _add_section_header("🏭 工厂信息", y_pos)

	_factory_info_label = Label.new()
	_factory_info_label.position = Vector2(24, y_pos)
	_factory_info_label.size = Vector2(PANEL_WIDTH - 40, ROW_HEIGHT)
	_factory_info_label.add_theme_font_size_override("font_size", FONT_SIZE_BODY)
	_factory_info_label.add_theme_color_override("font_color", COLOR_LABEL)
	_dialog.add_child(_factory_info_label)
	y_pos += ROW_HEIGHT

	_factory_used_label = Label.new()
	_factory_used_label.position = Vector2(24, y_pos)
	_factory_used_label.size = Vector2(PANEL_WIDTH - 40, ROW_HEIGHT)
	_factory_used_label.add_theme_font_size_override("font_size", FONT_SIZE_BODY)
	_factory_used_label.add_theme_color_override("font_color", COLOR_VALUE)
	_dialog.add_child(_factory_used_label)
	y_pos += ROW_HEIGHT + 4.0

	y_pos = _add_separator(y_pos)

	# ════════════════════════════════════════════
	# Section: 新建产线
	# ════════════════════════════════════════════
	y_pos = _add_section_header("🔧 新建产线", y_pos)

	# Line type selection buttons
	var line_btn_x: float = 16.0
	for lt: String in ["manual", "semi_auto", "auto"]:
		var line_data: Dictionary = LINE_TYPES[lt]
		var btn := Button.new()
		btn.text = "%s\n%dM %s" % [line_data["label"], line_data["cost"], line_data["desc"]]
		btn.position = Vector2(line_btn_x, y_pos)
		btn.size = Vector2(124, 44)
		btn.add_theme_font_size_override("font_size", FONT_SIZE_SMALL)
		btn.add_theme_color_override("font_color", Color(1.0, 1.0, 1.0))
		btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_LINE_BTN))
		btn.add_theme_stylebox_override("hover", _make_stylebox(COLOR_LINE_BTN_HOVER))
		btn.connect("pressed", _on_line_type_selected.bind(lt))
		_dialog.add_child(btn)
		_line_type_btns[lt] = btn
		line_btn_x += 132.0

	y_pos += 50.0

	_line_desc_label = Label.new()
	_line_desc_label.position = Vector2(24, y_pos)
	_line_desc_label.size = Vector2(PANEL_WIDTH - 40, ROW_HEIGHT)
	_line_desc_label.add_theme_font_size_override("font_size", FONT_SIZE_SMALL)
	_line_desc_label.add_theme_color_override("font_color", COLOR_VALUE)
	_dialog.add_child(_line_desc_label)
	y_pos += ROW_HEIGHT

	# Confirm line order button
	var confirm_line_btn := Button.new()
	confirm_line_btn.text = "✅ 确认订购产线"
	confirm_line_btn.position = Vector2(120, y_pos)
	confirm_line_btn.size = Vector2(180, 30)
	confirm_line_btn.add_theme_color_override("font_color", Color(1.0, 1.0, 1.0))
	confirm_line_btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_BTN_BG))
	confirm_line_btn.add_theme_stylebox_override("hover", _make_stylebox(COLOR_BTN_HOVER))
	confirm_line_btn.connect("pressed", _on_confirm_line_pressed)
	_dialog.add_child(confirm_line_btn)
	y_pos += 38.0

	y_pos = _add_separator(y_pos)

	# ════════════════════════════════════════════
	# Section: 贷款
	# ════════════════════════════════════════════
	y_pos = _add_section_header("🏦 贷款", y_pos)

	# Loan type buttons
	var loan_btn_x: float = 16.0
	for lt: String in ["short", "long"]:
		var btn := Button.new()
		var label: String = "短贷" if lt == "short" else "长贷"
		btn.text = label
		btn.position = Vector2(loan_btn_x, y_pos)
		btn.size = Vector2(64, 30)
		btn.add_theme_font_size_override("font_size", FONT_SIZE_BODY)
		btn.add_theme_color_override("font_color", Color(1.0, 1.0, 1.0))
		btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_LOAN_BTN))
		btn.add_theme_stylebox_override("hover", _make_stylebox(COLOR_LOAN_BTN_HOVER))
		btn.connect("pressed", _on_loan_type_selected.bind(lt))
		_dialog.add_child(btn)
		_loan_type_btns[lt] = btn
		loan_btn_x += 72.0

	# Amount label + input
	var amount_label := Label.new()
	amount_label.text = "金额:"
	amount_label.position = Vector2(loan_btn_x + 8, y_pos)
	amount_label.size = Vector2(40, ROW_HEIGHT)
	amount_label.add_theme_font_size_override("font_size", FONT_SIZE_BODY)
	amount_label.add_theme_color_override("font_color", COLOR_LABEL)
	_dialog.add_child(amount_label)

	_loan_amount_input = LineEdit.new()
	_loan_amount_input.position = Vector2(loan_btn_x + 46, y_pos)
	_loan_amount_input.size = Vector2(60, 30)
	_loan_amount_input.placeholder_text = "20"
	_loan_amount_input.text = "20"
	_dialog.add_child(_loan_amount_input)

	var m_label := Label.new()
	m_label.text = "M"
	m_label.position = Vector2(loan_btn_x + 110, y_pos)
	m_label.size = Vector2(20, ROW_HEIGHT)
	m_label.add_theme_font_size_override("font_size", FONT_SIZE_BODY)
	m_label.add_theme_color_override("font_color", COLOR_LABEL)
	_dialog.add_child(m_label)

	y_pos += 36.0

	# Interest rate info
	_loan_rate_label = Label.new()
	_loan_rate_label.position = Vector2(24, y_pos)
	_loan_rate_label.size = Vector2(PANEL_WIDTH - 40, ROW_HEIGHT)
	_loan_rate_label.add_theme_font_size_override("font_size", FONT_SIZE_SMALL)
	_loan_rate_label.add_theme_color_override("font_color", COLOR_VALUE)
	_dialog.add_child(_loan_rate_label)
	y_pos += ROW_HEIGHT

	# Confirm loan button
	var confirm_loan_btn := Button.new()
	confirm_loan_btn.text = "✅ 确认贷款"
	confirm_loan_btn.position = Vector2(140, y_pos)
	confirm_loan_btn.size = Vector2(140, 30)
	confirm_loan_btn.add_theme_color_override("font_color", Color(1.0, 1.0, 1.0))
	confirm_loan_btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_BTN_BG))
	confirm_loan_btn.add_theme_stylebox_override("hover", _make_stylebox(COLOR_BTN_HOVER))
	confirm_loan_btn.connect("pressed", _on_confirm_loan_pressed)
	_dialog.add_child(confirm_loan_btn)
	y_pos += 38.0

	y_pos = _add_separator(y_pos)

	# ════════════════════════════════════════════
	# Section: 贴现
	# ════════════════════════════════════════════
	y_pos = _add_section_header("💳 贴现", y_pos)

	_ar_amount_label = Label.new()
	_ar_amount_label.position = Vector2(24, y_pos)
	_ar_amount_label.size = Vector2(PANEL_WIDTH - 40, ROW_HEIGHT)
	_ar_amount_label.add_theme_font_size_override("font_size", FONT_SIZE_BODY)
	_ar_amount_label.add_theme_color_override("font_color", COLOR_LABEL)
	_dialog.add_child(_ar_amount_label)
	y_pos += ROW_HEIGHT

	_ar_fee_label = Label.new()
	_ar_fee_label.position = Vector2(24, y_pos)
	_ar_fee_label.size = Vector2(PANEL_WIDTH - 40, ROW_HEIGHT)
	_ar_fee_label.add_theme_font_size_override("font_size", FONT_SIZE_BODY)
	_ar_fee_label.add_theme_color_override("font_color", COLOR_VALUE)
	_dialog.add_child(_ar_fee_label)
	y_pos += ROW_HEIGHT

	_discount_confirm_btn = Button.new()
	_discount_confirm_btn.text = "✅ 确认贴现"
	_discount_confirm_btn.position = Vector2(140, y_pos)
	_discount_confirm_btn.size = Vector2(140, 30)
	_discount_confirm_btn.add_theme_color_override("font_color", Color(1.0, 1.0, 1.0))
	_discount_confirm_btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_BTN_BG))
	_discount_confirm_btn.add_theme_stylebox_override("hover", _make_stylebox(COLOR_BTN_HOVER))
	_discount_confirm_btn.connect("pressed", _on_confirm_discount_pressed)
	_dialog.add_child(_discount_confirm_btn)

	# Initialize selection states
	_update_line_selection("manual")
	_update_loan_selection("short")


# ── Content Refresh ──

func _refresh_factory_info() -> void:
	var factories: Array = _game_state.get("factories", [])
	if factories.is_empty():
		_factory_info_label.text = "当前工厂: 暂无"
		_factory_used_label.text = ""
		return

	var factory: Dictionary = factories[0]
	var name_str: String = factory.get("name", "未知工厂")
	var capacity: int = factory.get("capacity", 0)
	var lines: Array = factory.get("lines", [])
	var lines_used: int = 0
	var total_value: int = 0

	for line: Dictionary in lines:
		if not line.get("status", "") in ["idle", ""]:
			lines_used += 1
		# Estimate value: base factory 20M + line cost
		total_value += _get_line_value(line)

	var factory_value: int = 20 + total_value
	_factory_info_label.text = "当前工厂: %s (容量%d) 价值%dM" % [name_str, capacity, factory_value]
	_factory_used_label.text = "已用产线: %d/%d" % [lines.size(), capacity]


func _refresh_discount_info() -> void:
	var ar: Array = _game_state.get("accounts_receivable", [])
	if ar.is_empty():
		_ar_amount_label.text = "应收款: 无"
		_ar_fee_label.text = "手续费: -"
		_discount_confirm_btn.disabled = true
		return

	var amount: int = ar[0].get("amount", 0)
	var fee: int = ceili(float(amount) / 14.0)
	_ar_amount_label.text = "应收款: %dM" % amount
	_ar_fee_label.text = "手续费: %dM (1/14)" % fee
	_discount_confirm_btn.disabled = false


func _update_line_selection(line_type: String) -> void:
	_selected_line_type = line_type
	for lt: String in _line_type_btns.keys():
		var btn: Button = _line_type_btns[lt]
		if lt == line_type:
			btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_LINE_BTN_SELECTED))
		else:
			btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_LINE_BTN))

	# Update description
	var data: Dictionary = LINE_TYPES[line_type]
	_line_desc_label.text = "已选: %s — 费用 %dM, %s" % [data["label"], data["cost"], data["desc"]]


func _update_loan_selection(loan_type: String) -> void:
	_selected_loan_type = loan_type
	for lt: String in _loan_type_btns.keys():
		var btn: Button = _loan_type_btns[lt]
		if lt == loan_type:
			btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_LOAN_BTN_SELECTED))
		else:
			btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_LOAN_BTN))

	# Update rate info
	match loan_type:
		"short":
			_loan_rate_label.text = "利率: 短贷 10% / 年"
		"long":
			_loan_rate_label.text = "利率: 长贷 5% / 年"


static func _get_line_value(line: Dictionary) -> int:
	var type_str: String = line.get("type", "manual")
	match type_str:
		"semi_auto":
			return 8
		"auto":
			return 16
		_:
			return 4


# ── Layout Helpers ──

func _add_section_header(text: String, y: float) -> float:
	var label := Label.new()
	label.text = "  %s" % text
	label.position = Vector2(8, y)
	label.size = Vector2(PANEL_WIDTH - 16, SECTION_HEADER_HEIGHT)
	label.add_theme_font_size_override("font_size", FONT_SIZE_SECTION)
	label.add_theme_color_override("font_color", COLOR_SECTION)
	_dialog.add_child(label)
	return y + SECTION_HEADER_HEIGHT + 2.0


func _add_separator(y: float) -> float:
	var sep := HSeparator.new()
	sep.position = Vector2(12, y)
	sep.size = Vector2(PANEL_WIDTH - 24, 2)
	sep.modulate = COLOR_SEPARATOR
	_dialog.add_child(sep)
	return y + SEPARATOR_HEIGHT


static func _make_stylebox(bg_color: Color) -> StyleBoxFlat:
	var sb := StyleBoxFlat.new()
	sb.bg_color = bg_color
	sb.corner_radius_top_left = 4
	sb.corner_radius_top_right = 4
	sb.corner_radius_bottom_left = 4
	sb.corner_radius_bottom_right = 4
	return sb


func _center_dialog() -> void:
	if not _dialog:
		return
	var parent_size: Vector2 = size if size != Vector2.ZERO else Vector2(1280, 720)
	_dialog.position = Vector2(
		int((parent_size.x - PANEL_WIDTH) * 0.5),
		int((parent_size.y - PANEL_HEIGHT) * 0.5)
	)


# ── Handlers ──

func _on_line_type_selected(line_type: String) -> void:
	_update_line_selection(line_type)


func _on_confirm_line_pressed() -> void:
	factory_ordered.emit(_selected_line_type)
	visible = false


func _on_loan_type_selected(loan_type: String) -> void:
	_update_loan_selection(loan_type)


func _on_confirm_loan_pressed() -> void:
	var amount_str: String = _loan_amount_input.text.strip_edges()
	var amount: int = int(amount_str) if amount_str.is_valid_int() else 0
	if amount <= 0:
		return
	loan_requested.emit(_selected_loan_type, amount)
	visible = false


func _on_confirm_discount_pressed() -> void:
	var ar: Array = _game_state.get("accounts_receivable", [])
	if ar.is_empty():
		return
	var amount: int = ar[0].get("amount", 0)
	discount_confirmed.emit(amount)
	visible = false


func _on_close_pressed() -> void:
	visible = false


func _on_overlay_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
		visible = false

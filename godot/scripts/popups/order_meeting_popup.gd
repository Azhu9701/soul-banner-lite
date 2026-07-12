extends Panel
## 订货会弹窗——展示市场预测和可选订单卡片，支持多选后提交。
##
## 通过 show_orders() 传入市场名、订单列表和年份，弹窗显示市场预测柱状图
## 和订单卡片列表。玩家点击卡片选中/取消，点击"确认提交选中订单"触发回调。
##
## 用法：
##   var popup := OrderMeetingPopup.new()
##   add_child(popup)
##   popup.show_orders("平城市场", orders_array, 2)
##   var selected: Array = popup.get_selected_order_ids()

class_name OrderMeetingPopup

# ── Colors ──

const OVERLAY_COLOR: Color = Color(0.0, 0.0, 0.0, 0.65)
const PANEL_BG: Color = Color(0.12, 0.12, 0.15)
const SECTION_BG: Color = Color(0.10, 0.10, 0.14)
const CARD_BG: Color = Color(0.16, 0.16, 0.20)
const CARD_BG_SELECTED: Color = Color(0.20, 0.30, 0.24)
const CARD_BORDER: Color = Color(0.30, 0.30, 0.35)
const CARD_BORDER_SELECTED: Color = Color(0.30, 1.00, 0.50)
const CARD_BORDER_HOVER: Color = Color(0.45, 0.45, 0.50)
const COLOR_TITLE: Color = Color(1.0, 1.0, 1.0)
const COLOR_LABEL: Color = Color(0.70, 0.70, 0.75)
const COLOR_VALUE: Color = Color(0.95, 0.85, 0.30)
const COLOR_SECTION: Color = Color(0.55, 0.80, 0.95)
const COLOR_URGENT: Color = Color(1.00, 0.25, 0.25)
const COLOR_BAR: Color = Color(0.30, 0.70, 0.95)
const COLOR_BAR_LABEL: Color = Color(0.85, 0.85, 0.90)
const COLOR_SEPARATOR: Color = Color(0.30, 0.30, 0.30)
const COLOR_BTN_BG: Color = Color(0.18, 0.55, 0.28)
const COLOR_BTN_HOVER: Color = Color(0.22, 0.65, 0.32)
const COLOR_BTN_DISABLED: Color = Color(0.15, 0.15, 0.18)
const COLOR_CLOSE_BG: Color = Color(0.35, 0.20, 0.20)
const COLOR_CLOSE_HOVER: Color = Color(0.50, 0.25, 0.25)
const COLOR_ACCENT: Color = Color(0.55, 0.80, 0.95)

# ── Layout Constants ──

const POPUP_WIDTH: float = 720.0
const POPUP_HEIGHT: float = 620.0
const TITLE_HEIGHT: float = 40.0
const TOP_BAR_HEIGHT: float = 36.0
const CHART_SECTION_HEIGHT: float = 200.0
const CARD_WIDTH: float = 310.0
const CARD_HEIGHT: float = 82.0
const CARD_GAP: float = 8.0
const FONT_SIZE_TITLE: int = 18
const FONT_SIZE_SECTION: int = 14
const FONT_SIZE_CARD_TITLE: int = 13
const FONT_SIZE_CARD_DETAIL: int = 11
const MAX_VISIBLE_CARDS: int = 6  # max before scroll

# Product visual metadata
const PRODUCT_MAP: Dictionary = {
	"ben_ma": {"emoji": "🐴", "name": "奔马", "color": Color(0.76, 0.42, 0.22)},
	"meng_hu": {"emoji": "🐯", "name": "猛虎", "color": Color(0.90, 0.60, 0.20)},
	"fei_ying": {"emoji": "🦅", "name": "飞鹰", "color": Color(0.25, 0.60, 0.90)},
	"xiang_long": {"emoji": "🐉", "name": "翔龙", "color": Color(0.80, 0.20, 0.20)},
}

# ── Nodes ──

var _overlay: ColorRect
var _dialog: Panel
var _title_label: Label
var _submit_btn: Button
var _close_btn: Button
var _chart_scroll: ScrollContainer
var _chart_vbox: VBoxContainer
var _orders_scroll: ScrollContainer
var _orders_grid: VBoxContainer
var _status_label: Label
var _selected_count_label: Label

# ── State ──

var _orders: Array = []
var _selected_ids: Dictionary = {}  # id -> true
var _card_nodes: Dictionary = {}  # order_id -> Panel (card)
var _market_name: String = ""
var _current_year: int = 1

# Signal emitted when user confirms selections
signal orders_confirmed(selected_ids: Array)


# ── Virtual Methods ──

func _ready() -> void:
	_build_ui()
	visible = false


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		if _dialog and _overlay:
			_center_dialog()


# ── Public Methods ──

## 显示订货会弹窗。
## market_name: 市场名称，如"平城市场"
## orders: 订单数组，每项含 id, product, quantity, unit_price, account_period, delivered, urgent
## year: 当前游戏年
## predictions: 可选的市场价格预测数据，格式 [{year: int, price: float}, ...]
func show_orders(market_name: String, orders: Array, year: int, predictions: Array = []) -> void:
	_market_name = market_name
	_orders = orders.duplicate()
	_current_year = year
	_selected_ids.clear()
	_card_nodes.clear()

	_title_label.text = "📋 订货会 — %s  第 %d 年" % [market_name, year]

	# Build chart
	_build_chart(predictions)

	# Build order cards
	_build_order_cards()

	# Update UI state
	_update_submit_button()
	_selected_count_label.text = "已选: 0 张"

	visible = true
	_center_dialog()


## 返回当前选中的订单 ID 数组。
func get_selected_order_ids() -> Array:
	var ids: Array = []
	for oid: Variant in _selected_ids.keys():
		ids.append(oid)
	return ids


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
	_dialog.custom_minimum_size = Vector2(POPUP_WIDTH, POPUP_HEIGHT)
	_dialog.add_theme_stylebox_override("panel", _make_stylebox(PANEL_BG))
	add_child(_dialog)

	# ── Title bar ──
	_title_label = Label.new()
	_title_label.name = "TitleLabel"
	_title_label.text = "📋 订货会"
	_title_label.add_theme_font_size_override("font_size", FONT_SIZE_TITLE)
	_title_label.add_theme_color_override("font_color", COLOR_TITLE)
	_title_label.position = Vector2(20, 12)
	_title_label.size = Vector2(POPUP_WIDTH - 120, TOP_BAR_HEIGHT)
	_dialog.add_child(_title_label)

	# ── Close button ──
	_close_btn = Button.new()
	_close_btn.name = "CloseBtn"
	_close_btn.text = "✕ 关闭"
	_close_btn.position = Vector2(POPUP_WIDTH - 100, 10)
	_close_btn.size = Vector2(80, 28)
	_close_btn.add_theme_color_override("font_color", Color(1.0, 0.8, 0.8))
	_close_btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_CLOSE_BG))
	_close_btn.add_theme_stylebox_override("hover", _make_stylebox(COLOR_CLOSE_HOVER))
	_close_btn.connect("pressed", _on_close_pressed)
	_dialog.add_child(_close_btn)

	# ── Section: 市场预测 ──
	var chart_section := _make_section_header("📈 市场预测")
	chart_section.position = Vector2(10, TOP_BAR_HEIGHT + 4)
	_dialog.add_child(chart_section)

	_chart_scroll = ScrollContainer.new()
	_chart_scroll.name = "ChartScroll"
	_chart_scroll.position = Vector2(10, TOP_BAR_HEIGHT + 28)
	_chart_scroll.size = Vector2(POPUP_WIDTH - 20, CHART_SECTION_HEIGHT - 32)
	_chart_scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_dialog.add_child(_chart_scroll)

	_chart_vbox = VBoxContainer.new()
	_chart_vbox.name = "ChartContent"
	_chart_vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_chart_vbox.add_theme_constant_override("separation", 2)
	_chart_scroll.add_child(_chart_vbox)

	# Separator
	var sep := HSeparator.new()
	sep.position = Vector2(10, TOP_BAR_HEIGHT + CHART_SECTION_HEIGHT)
	sep.size = Vector2(POPUP_WIDTH - 20, 2)
	sep.modulate = COLOR_SEPARATOR
	_dialog.add_child(sep)

	# ── Section: 可选订单 ──
	var orders_section := _make_section_header("📋 可选订单")
	orders_section.position = Vector2(10, TOP_BAR_HEIGHT + CHART_SECTION_HEIGHT + 6)
	_dialog.add_child(orders_section)

	# Selected count label
	_selected_count_label = Label.new()
	_selected_count_label.name = "SelectedCount"
	_selected_count_label.text = "已选: 0 张"
	_selected_count_label.add_theme_font_size_override("font_size", FONT_SIZE_CARD_DETAIL)
	_selected_count_label.add_theme_color_override("font_color", COLOR_VALUE)
	_selected_count_label.position = Vector2(POPUP_WIDTH - 140, TOP_BAR_HEIGHT + CHART_SECTION_HEIGHT + 8)
	_selected_count_label.size = Vector2(120, 20)
	_dialog.add_child(_selected_count_label)

	# Orders scroll area
	_orders_scroll = ScrollContainer.new()
	_orders_scroll.name = "OrdersScroll"
	var orders_y: float = TOP_BAR_HEIGHT + CHART_SECTION_HEIGHT + 32
	var orders_h: float = POPUP_HEIGHT - orders_y - 56.0
	_orders_scroll.position = Vector2(10, orders_y)
	_orders_scroll.size = Vector2(POPUP_WIDTH - 20, orders_h)
	_orders_scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_orders_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_dialog.add_child(_orders_scroll)

	_orders_grid = VBoxContainer.new()
	_orders_grid.name = "OrdersGrid"
	_orders_grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_orders_grid.add_theme_constant_override("separation", CARD_GAP)
	_orders_scroll.add_child(_orders_grid)

	# ── Submit button ──
	_submit_btn = Button.new()
	_submit_btn.name = "SubmitBtn"
	_submit_btn.text = "✅ 确认提交选中订单"
	_submit_btn.position = Vector2(
		(POPUP_WIDTH - 220) * 0.5,
		POPUP_HEIGHT - 44
	)
	_submit_btn.size = Vector2(220, 34)
	_submit_btn.add_theme_color_override("font_color", Color(1.0, 1.0, 1.0))
	_submit_btn.add_theme_stylebox_override("normal", _make_stylebox(COLOR_BTN_BG))
	_submit_btn.add_theme_stylebox_override("hover", _make_stylebox(COLOR_BTN_HOVER))
	_submit_btn.add_theme_stylebox_override("disabled", _make_stylebox(COLOR_BTN_DISABLED))
	_submit_btn.connect("pressed", _on_submit_pressed)
	_dialog.add_child(_submit_btn)


func _build_chart(predictions: Array) -> void:
	# Clear previous chart content
	for child in _chart_vbox.get_children():
		_chart_vbox.remove_child(child)
		child.queue_free()

	# If no predictions provided, generate sample data for demonstration
	var display_data: Array = predictions.duplicate()
	if display_data.is_empty():
		display_data = _generate_sample_predictions()

	# Build bar chart
	var max_price: float = 1.0
	for entry: Dictionary in display_data:
		var price: float = float(entry.get("price", 0))
		if price > max_price:
			max_price = price

	var bar_area_width: float = POPUP_WIDTH - 60.0  # leave room for labels

	for entry: Dictionary in display_data:
		var year_label_val: int = entry.get("year", _current_year + 1)
		var price: float = float(entry.get("price", 0))
		var bar_width: float = (price / max_price) * bar_area_width

		var row := HBoxContainer.new()
		row.custom_minimum_size = Vector2(0, 24)
		row.size_flags_horizontal = Control.SIZE_EXPAND_FILL

		# Year label
		var ylabel := Label.new()
		ylabel.text = "第%d年" % year_label_val
		ylabel.custom_minimum_size = Vector2(60, 0)
		ylabel.add_theme_font_size_override("font_size", FONT_SIZE_CARD_DETAIL)
		ylabel.add_theme_color_override("font_color", COLOR_BAR_LABEL)
		ylabel.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		row.add_child(ylabel)

		# Bar
		var bar := ColorRect.new()
		bar.color = _get_bar_color(display_data.find(entry), len(display_data))
		bar.custom_minimum_size = Vector2(max(bar_width, 4.0), 18)
		bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		row.add_child(bar)

		# Price label
		var plabel := Label.new()
		plabel.text = "%dM" % price
		plabel.custom_minimum_size = Vector2(60, 0)
		plabel.add_theme_font_size_override("font_size", FONT_SIZE_CARD_DETAIL)
		plabel.add_theme_color_override("font_color", COLOR_VALUE)
		plabel.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		row.add_child(plabel)

		_chart_vbox.add_child(row)


func _build_order_cards() -> void:
	# Clear previous cards
	for child in _orders_grid.get_children():
		_orders_grid.remove_child(child)
		child.queue_free()

	_card_nodes.clear()

	for order: Dictionary in _orders:
		var card := _make_order_card(order)
		_orders_grid.add_child(card)
		_card_nodes[order.get("id", "")] = card


func _make_order_card(order: Dictionary) -> Panel:
	var order_id: String = order.get("id", "")
	var product: String = order.get("product", "")
	var quantity: int = order.get("quantity", 1)
	var unit_price: int = order.get("unit_price", 0)
	var account_period: int = order.get("account_period", 0)
	var urgent: bool = order.get("urgent", false)

	# Get product visual info
	var product_info: Dictionary = PRODUCT_MAP.get(product, {"emoji": "📦", "name": product, "color": Color(0.50, 0.50, 0.55)})

	# Card panel
	var card := Panel.new()
	card.name = "Card_%s" % order_id
	card.custom_minimum_size = Vector2(CARD_WIDTH, CARD_HEIGHT)
	card.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	card.mouse_filter = Control.MOUSE_FILTER_STOP
	card.connect("gui_input", _on_card_gui_input.bind(order_id))

	# Default style (unselected)
	_update_card_style(card, false)

	# HBox layout inside card
	var hb := HBoxContainer.new()
	hb.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hb.size_flags_vertical = Control.SIZE_EXPAND_FILL
	hb.anchor_right = 1.0
	hb.anchor_bottom = 1.0
	card.add_child(hb)

	# Product color swatch
	var swatch := ColorRect.new()
	swatch.color = product_info.get("color", Color(0.5, 0.5, 0.5))
	swatch.custom_minimum_size = Vector2(8, CARD_HEIGHT - 8)
	swatch.size_flags_vertical = Control.SIZE_EXPAND_FILL
	hb.add_child(swatch)

	# Spacer between swatch and content
	var swatch_spacer := Control.new()
	swatch_spacer.custom_minimum_size = Vector2(8, 0)
	hb.add_child(swatch_spacer)

	# Content VBox
	var content := VBoxContainer.new()
	content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.size_flags_vertical = Control.SIZE_EXPAND_FILL
	content.add_theme_constant_override("separation", 2)
	hb.add_child(content)

	# Row 1: Product name + quantity + urgent badge
	var row1 := HBoxContainer.new()
	row1.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var name_label := Label.new()
	name_label.text = "%s %s x%d" % [product_info.get("emoji", "📦"), product_info.get("name", product), quantity]
	name_label.add_theme_font_size_override("font_size", FONT_SIZE_CARD_TITLE)
	name_label.add_theme_color_override("font_color", COLOR_TITLE)
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row1.add_child(name_label)

	# Urgent badge
	if urgent:
		var urgent_label := Label.new()
		urgent_label.text = "🔴 紧急"
		urgent_label.add_theme_font_size_override("font_size", FONT_SIZE_CARD_DETAIL)
		urgent_label.add_theme_color_override("font_color", COLOR_URGENT)
		row1.add_child(urgent_label)

	content.add_child(row1)

	# Row 2: Details
	var row2 := HBoxContainer.new()
	row2.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var price_label := Label.new()
	price_label.text = "单价 %dM" % unit_price
	price_label.add_theme_font_size_override("font_size", FONT_SIZE_CARD_DETAIL)
	price_label.add_theme_color_override("font_color", COLOR_VALUE)
	row2.add_child(price_label)

	var period_label := Label.new()
	period_label.text = "  账期 %d 季" % account_period
	period_label.add_theme_font_size_override("font_size", FONT_SIZE_CARD_DETAIL)
	period_label.add_theme_color_override("font_color", COLOR_LABEL)
	row2.add_child(period_label)

	if urgent:
		var urgent_detail := Label.new()
		urgent_detail.text = "  ⚡ 交货期紧"
		urgent_detail.add_theme_font_size_override("font_size", FONT_SIZE_CARD_DETAIL)
		urgent_detail.add_theme_color_override("font_color", COLOR_URGENT)
		row2.add_child(urgent_detail)

	content.add_child(row2)

	# Row 3: Selection status
	var row3 := HBoxContainer.new()
	row3.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var status_label := Label.new()
	status_label.name = "StatusLabel"
	status_label.text = "  [ 点击选中 ]"
	status_label.add_theme_font_size_override("font_size", FONT_SIZE_CARD_DETAIL - 1)
	status_label.add_theme_color_override("font_color", Color(0.45, 0.45, 0.50))
	row3.add_child(status_label)

	content.add_child(row3)

	# Spacer to fill remaining space
	var fill := Control.new()
	fill.size_flags_vertical = Control.SIZE_EXPAND_FILL
	content.add_child(fill)

	return card


func _update_card_style(card: Panel, selected: bool) -> void:
	var bg: StyleBoxFlat = null
	if selected:
		bg = _make_stylebox(CARD_BG_SELECTED)
		bg.border_color = CARD_BORDER_SELECTED
		bg.border_width_top = 2
		bg.border_width_bottom = 2
		bg.border_width_left = 2
		bg.border_width_right = 2
	else:
		bg = _make_stylebox(CARD_BG)
		bg.border_color = CARD_BORDER
		bg.border_width_top = 1
		bg.border_width_bottom = 1
		bg.border_width_left = 1
		bg.border_width_right = 1

	card.add_theme_stylebox_override("panel", bg)


func _update_selection_status_label(order_id: String) -> void:
	var card: Panel = _card_nodes.get(order_id) as Panel
	if not card:
		return

	# Find StatusLabel inside the card
	var hb: HBoxContainer = card.get_child(0) as HBoxContainer
	if not hb:
		return
	var content: VBoxContainer = null
	for child in hb.get_children():
		if child is VBoxContainer:
			content = child
			break
	if not content:
		return

	# StatusLabel is in row3 (index 2 for the 3rd row)
	var rows: Array = content.get_children()
	if rows.size() < 3:
		return
	var row3: HBoxContainer = rows[2] as HBoxContainer
	if not row3:
		return

	var status_label: Label = row3.get_child(0) as Label
	if not status_label:
		return

	var is_selected: bool = _selected_ids.has(order_id)
	if is_selected:
		status_label.text = "  ✅ [ 已选中 ]"
		status_label.add_theme_color_override("font_color", CARD_BORDER_SELECTED)
	else:
		status_label.text = "  [ 点击选中 ]"
		status_label.add_theme_color_override("font_color", Color(0.45, 0.45, 0.50))


func _update_submit_button() -> void:
	var count: int = _selected_ids.size()
	_submit_btn.disabled = count == 0
	_selected_count_label.text = "已选: %d 张" % count


# ── Helpers ──

func _generate_sample_predictions() -> Array:
	# Generate synthetic price predictions for demonstration
	var samples: Array = []
	for i in range(1, 5):
		var year: int = _current_year + i
		var base_price: int = 8 + (i * 4) + randi() % 3
		samples.append({"year": year, "price": base_price})
	return samples


static func _get_bar_color(index: int, total: int) -> Color:
	# Gradient from blue to green
	var t: float = float(index) / float(max(total - 1, 1))
	return Color(
		0.30 + t * 0.20,
		0.60 + t * 0.30,
		0.95 - t * 0.30
	)


static func _make_stylebox(bg_color: Color) -> StyleBoxFlat:
	var sb := StyleBoxFlat.new()
	sb.bg_color = bg_color
	sb.corner_radius_top_left = 4
	sb.corner_radius_top_right = 4
	sb.corner_radius_bottom_left = 4
	sb.corner_radius_bottom_right = 4
	return sb


static func _make_section_header(text: String) -> Label:
	var label := Label.new()
	label.text = "  %s" % text
	label.custom_minimum_size = Vector2(0, 24)
	label.add_theme_font_size_override("font_size", FONT_SIZE_SECTION)
	label.add_theme_color_override("font_color", COLOR_SECTION)
	return label


func _center_dialog() -> void:
	if not _dialog:
		return
	var parent_size: Vector2 = size if size != Vector2.ZERO else Vector2(1280, 720)
	_dialog.position = Vector2(
		int((parent_size.x - POPUP_WIDTH) * 0.5),
		int((parent_size.y - POPUP_HEIGHT) * 0.5)
	)


# ── Handlers ──

func _on_card_gui_input(event: InputEvent, order_id: String) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
		# Toggle selection
		if _selected_ids.has(order_id):
			_selected_ids.erase(order_id)
		else:
			_selected_ids[order_id] = true

		# Update card visuals
		var card: Panel = _card_nodes.get(order_id) as Panel
		if card:
			var is_selected: bool = _selected_ids.has(order_id)
			_update_card_style(card, is_selected)
			_update_selection_status_label(order_id)

		_update_submit_button()


func _on_submit_pressed() -> void:
	var ids: Array = get_selected_order_ids()
	orders_confirmed.emit(ids)
	_log_selection(ids)
	visible = false


func _log_selection(ids: Array) -> void:
	var names: PackedStringArray = []
	for oid: Variant in ids:
		var oid_str: String = str(oid)
		# Find matching order
		for order: Dictionary in _orders:
			if str(order.get("id", "")) == oid_str:
				var prod: String = order.get("product", "")
				var info: Dictionary = PRODUCT_MAP.get(prod, {"name": prod})
				names.append("%s x%d" % [info.get("name", prod), order.get("quantity", 0)])
				break
	var detail: String = ", ".join(names)
	print("📋 提交订单 [%s]: %s" % [_market_name, detail])


func _on_close_pressed() -> void:
	visible = false


func _on_overlay_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
		visible = false

extends Control
## 创业沙盘主游戏场景——管理 UI 布局、决策弹窗和服务端通信。
##
## 生命周期：
## 1. _ready() 构建 UI → 连接 WS → 发送 start_game
## 2. 服务端推送 state_update → _update_ui() 刷新
## 3. 服务端推送 ask_decision → 弹出决策面板
## 4. 用户决策 → send_action() → 下一轮

class_name MainGame

# ── Preloads ──

const AnnualReportPopupScript := preload("res://scripts/popups/annual_report_popup.gd")
const OrderMeetingPopupScript := preload("res://scripts/popups/order_meeting_popup.gd")
const MarketPanelScript := preload("res://scripts/market_panel.gd")
const FactoryDialogScript := preload("res://scripts/popups/factory_dialog.gd")


# ── Constants ──

const INITIAL_WINDOW_SIZE: Vector2 = Vector2(800, 600)
const PANEL_MARGIN: float = 10.0
const LOG_HEIGHT: float = 380.0
const BOTTOM_HEIGHT: float = 40.0

# ── @onready Vars ──

@onready var _ws_manager: Node = $/root/WebSocketManager
@onready var _production_area: VBoxContainer = $ProductionArea

# ── Private Vars ──

var _game_state: Dictionary = {}
var _decision_type: String = ""
var _production_view: ProductionView
var _inventory_dashboard: InventoryDashboard
var _market_panel: Variant

# Lazy-init UI nodes (created in _build_ui)
var _year_label: Label
var _quarter_label: Label
var _cash_label: Label
var _phase_label: Label
var _msg_log: RichTextLabel
var _action_btn: Button
var _popup_panel: Panel
var _popup_title: Label
var _popup_input: LineEdit
var _popup_btn: Button
var _annual_report_popup: Variant
var _order_meeting_popup: Variant
var _factory_dialog: Variant


# ── Virtual Methods ──

func _ready() -> void:
	custom_minimum_size = INITIAL_WINDOW_SIZE
	_build_ui()
	_ws_manager.message_received.connect(_on_message_received)
	_ws_manager.connected.connect(_on_ws_connected)
	_ws_manager.connection_failed.connect(_on_ws_failed)
	_ws_manager.connect_to_server("game_001")


# ── UI Building ──

func _build_ui() -> void:
	_build_top_bar()
	_build_message_log()
	_build_bottom_bar()
	_build_market_panel()
	_build_production_view()
	_build_inventory_dashboard()
	_build_popup()
	_build_annual_report_popup()
	_build_order_meeting_popup()
	_build_factory_dialog()


func _build_top_bar() -> void:
	var hb := HBoxContainer.new()
	hb.position = Vector2(PANEL_MARGIN, PANEL_MARGIN)
	hb.size = Vector2(780, 36)
	add_child(hb)

	var title := Label.new()
	title.text = "🏢 创业沙盘"
	title.add_theme_font_size_override("font_size", 18)
	hb.add_child(title)

	hb.add_child(_make_spacer())

	_year_label = Label.new()
	_year_label.text = "📅 第 1 年"
	hb.add_child(_year_label)

	_quarter_label = Label.new()
	_quarter_label.text = " 第 1 季度"
	hb.add_child(_quarter_label)

	_cash_label = Label.new()
	_cash_label.text = " 💰 12M"
	hb.add_child(_cash_label)

	_phase_label = Label.new()
	_phase_label.text = " 阶段 1"
	hb.add_child(_phase_label)


func _build_message_log() -> void:
	_msg_log = RichTextLabel.new()
	_msg_log.position = Vector2(PANEL_MARGIN, 56.0)
	_msg_log.size = Vector2(780.0, LOG_HEIGHT)
	_msg_log.bbcode_enabled = true
	_msg_log.scroll_active = true
	_msg_log.text = "[b]欢迎来到创业沙盘！[/b]\n正在连接服务器...\n"
	add_child(_msg_log)


func _build_bottom_bar() -> void:
	var hb := HBoxContainer.new()
	hb.position = Vector2(PANEL_MARGIN, 56.0 + LOG_HEIGHT + PANEL_MARGIN)
	hb.size = Vector2(780.0, BOTTOM_HEIGHT)
	add_child(hb)

	_action_btn = Button.new()
	_action_btn.text = "等待服务器..."
	_action_btn.disabled = true
	_action_btn.pressed.connect(_on_action_clicked)
	hb.add_child(_action_btn)

	var discount_btn := Button.new()
	discount_btn.text = "💳 贴现"
	discount_btn.pressed.connect(_on_discount_clicked)
	hb.add_child(discount_btn)

	var loan_btn := Button.new()
	loan_btn.text = "🏦 贷款 20M"
	loan_btn.pressed.connect(_on_loan_clicked)
	hb.add_child(loan_btn)


func _build_production_view() -> void:
	var prod_view := ProductionView.new()
	prod_view.name = "ProductionView"
	prod_view.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	prod_view.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_production_area.add_child(prod_view)
	_production_view = prod_view


func _build_inventory_dashboard() -> void:
	var panel := Panel.new()
	panel.position = Vector2(470.0, 56.0)
	panel.size = Vector2(310.0, 380.0)
	add_child(panel)

	_inventory_dashboard = InventoryDashboard.new()
	_inventory_dashboard.name = "InventoryDashboard"
	_inventory_dashboard.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_inventory_dashboard.size_flags_vertical = Control.SIZE_EXPAND_FILL
	panel.add_child(_inventory_dashboard)


func _build_market_panel() -> void:
	_market_panel = MarketPanelScript.new()
	_market_panel.name = "MarketPanel"
	_market_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_market_panel.submit_bidding.connect(_on_market_panel_submit)
	_production_area.add_child(_market_panel)


func _build_popup() -> void:
	_popup_panel = Panel.new()
	_popup_panel.position = Vector2(150.0, 100.0)
	_popup_panel.size = Vector2(500.0, 200.0)
	_popup_panel.visible = false
	add_child(_popup_panel)

	_popup_title = Label.new()
	_popup_title.position = Vector2(20.0, 20.0)
	_popup_title.size = Vector2(460.0, 30.0)
	_popup_title.add_theme_font_size_override("font_size", 16)
	_popup_panel.add_child(_popup_title)

	var desc := Label.new()
	desc.position = Vector2(20.0, 55.0)
	desc.size = Vector2(460.0, 20.0)
	desc.text = "输入营销投入金额（M），决定能激活多少张订单："
	_popup_panel.add_child(desc)

	_popup_input = LineEdit.new()
	_popup_input.position = Vector2(20.0, 80.0)
	_popup_input.size = Vector2(200.0, 30.0)
	_popup_input.placeholder_text = "例如: 2"
	_popup_input.text = "2"
	_popup_panel.add_child(_popup_input)

	_popup_btn = Button.new()
	_popup_btn.position = Vector2(20.0, 120.0)
	_popup_btn.size = Vector2(120.0, 30.0)
	_popup_btn.text = "✅ 提交"
	_popup_btn.pressed.connect(_on_popup_confirmed)
	_popup_panel.add_child(_popup_btn)

	var cancel_btn := Button.new()
	cancel_btn.position = Vector2(150.0, 120.0)
	cancel_btn.size = Vector2(80.0, 30.0)
	cancel_btn.text = "取消"
	cancel_btn.pressed.connect(_hide_popup)
	_popup_panel.add_child(cancel_btn)


func _build_annual_report_popup() -> void:
	var popup: Variant = AnnualReportPopupScript.new()
	popup.name = "AnnualReportPopup"
	popup.visible = false
	popup.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	popup.size_flags_vertical = Control.SIZE_EXPAND_FILL
	var popup_layer: CanvasLayer = $PopupLayer
	popup_layer.add_child(popup)
	_annual_report_popup = popup


func _build_order_meeting_popup() -> void:
	var popup: Variant = OrderMeetingPopupScript.new()
	popup.name = "OrderMeetingPopup"
	popup.visible = false
	popup.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	popup.size_flags_vertical = Control.SIZE_EXPAND_FILL
	var popup_layer: CanvasLayer = $PopupLayer
	popup_layer.add_child(popup)
	popup.orders_confirmed.connect(_on_order_meeting_confirmed)
	_order_meeting_popup = popup


func _build_factory_dialog() -> void:
	var dialog: Variant = FactoryDialogScript.new()
	dialog.name = "FactoryDialog"
	dialog.visible = false
	dialog.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	dialog.size_flags_vertical = Control.SIZE_EXPAND_FILL
	var popup_layer: CanvasLayer = $PopupLayer
	popup_layer.add_child(dialog)
	dialog.factory_ordered.connect(_on_factory_ordered)
	dialog.loan_requested.connect(_on_factory_loan_requested)
	dialog.discount_confirmed.connect(_on_factory_discount_confirmed)
	_factory_dialog = dialog


# ── Helpers ──

static func _make_spacer() -> Control:
	var s := Control.new()
	s.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	return s


func _log(msg: String) -> void:
	_msg_log.text += msg + "\n"
	_msg_log.scroll_to_line(_msg_log.get_line_count() - 1)


func _hide_popup() -> void:
	_popup_panel.visible = false


# ── WS Callbacks ──

func _on_ws_connected() -> void:
	_log("[color=green]✅ 已连接到游戏服务器[/color]")
	_ws_manager.send_action({"action": "start_game"})


func _on_ws_failed() -> void:
	_log("[color=red]❌ 无法连接到游戏服务器\n请确认 Rust API 已在 localhost:3096 运行[/color]")


func _on_message_received(data: Dictionary) -> void:
	var event_type: String = data.get("event", "")
	match event_type:
		"state_update":
			_game_state = data.get("data", {})
			_update_ui()
		"ask_decision":
			_show_decision(data.get("data", {}))
		"message":
			_log("[color=yellow]📢 %s[/color]" % data.get("data", ""))
		"game_over":
			var reason: String = data.get("data", {}).get("reason", "未知原因")
			_log("[color=red]💀 游戏结束: %s[/color]" % reason)
			_action_btn.disabled = true
			_action_btn.text = "游戏结束"
		"phase_change":
			var pd: Dictionary = data.get("data", {})
			_log("[color=cyan]📅 第 %d 年第 %d 季度 — %s[/color]" % [
				pd.get("year", 1), pd.get("quarter", 0), pd.get("phase", "")
			])
		"annual_report":
			var report_data: Dictionary = data.get("data", {})
			_log("[color=green]📊 收到年度报告 — 第 %d 年[/color]" % report_data.get("year", 0))
			if _annual_report_popup:
				_annual_report_popup.show_report(report_data)
		"order_meeting":
			var order_data: Dictionary = data.get("data", {})
			var market_name: String = order_data.get("market_name", "")
			var orders: Array = order_data.get("orders", [])
			var year: int = order_data.get("year", 1)
			var predictions: Array = order_data.get("predictions", [])
			_log("[color=cyan]📋 收到订货会数据 — %s  第 %d 年 (%d 张订单)[/color]" % [market_name, year, orders.size()])
			if _order_meeting_popup:
				_order_meeting_popup.show_orders(market_name, orders, year, predictions)


# ── UI Updates ──

func _update_ui() -> void:
	var gs: Dictionary = _game_state
	_year_label.text = "📅 第 %d 年" % gs.get("game_year", 1)
	_quarter_label.text = " 第 %d 季度" % gs.get("game_quarter", 1)
	_cash_label.text = " 💰 %dM" % gs.get("cash", 0)
	_phase_label.text = " 阶段 %d" % gs.get("phase", 1)

	if _production_view:
		_production_view.update(gs.get("factories", []))

	if _inventory_dashboard:
		_inventory_dashboard.update(gs)

	if _market_panel:
		_market_panel.update(gs.get("markets", []))


func _show_decision(data: Dictionary) -> void:
	_decision_type = data.get("decision_type", "")
	_popup_title.text = "📋 %s" % _decision_type

	match _decision_type:
		"bidding":
			_popup_input.placeholder_text = "输入营销投入 M"
			_popup_input.text = "2"
			_popup_btn.text = "✅ 提交竞标"
		_:
			_popup_input.placeholder_text = "输入..."
			_popup_btn.text = "确认"

	_popup_panel.visible = true


# ── Button Handlers ──

func _on_popup_confirmed() -> void:
	var input_val: String = _popup_input.text.strip_edges()
	_hide_popup()

	match _decision_type:
		"bidding":
			var spend: int = int(input_val) if input_val.is_valid_int() else 2
			_log("[color=green]📋 提交竞标: 营销投入 %dM[/color]" % spend)
			_ws_manager.send_action({
				"action": "submit_bidding",
				"strategies": [{"market_name": "平城", "marketing_spend": spend}]
			})
			_action_btn.text = "▶ 执行下一季度"
			_action_btn.disabled = false
	_popup_input.text = ""


func _on_action_clicked() -> void:
	_action_btn.disabled = true
	_action_btn.text = "执行中..."
	_log("[color=cyan]▶ 执行下一季度...[/color]")
	_ws_manager.send_action({"action": "next_quarter"})


func _on_discount_clicked() -> void:
	if _factory_dialog:
		_factory_dialog.show_dialog(_game_state)


func _on_order_meeting_confirmed(selected_ids: Array) -> void:
	if selected_ids.is_empty():
		return
	_log("[color=green]📋 提交 %d 张订单[/color]" % selected_ids.size())
	_ws_manager.send_action({
		"action": "submit_orders",
		"order_ids": selected_ids
	})


func _on_loan_clicked() -> void:
	if _factory_dialog:
		_factory_dialog.show_dialog(_game_state)


# ── Factory Dialog Handlers ──

func _on_factory_ordered(line_type: String) -> void:
	var type_label: String = "手工线"
	match line_type:
		"semi_auto":
			type_label = "半自动"
		"auto":
			type_label = "全自动"
	_log("[color=green]🔧 订购新产线: %s[/color]" % type_label)
	_ws_manager.send_action({
		"action": "order_production_line",
		"line_type": line_type
	})


func _on_factory_loan_requested(loan_type: String, amount: int) -> void:
	var type_label: String = "短期" if loan_type == "short" else "长期"
	_log("[color=yellow]🏦 申请%s贷款 %dM...[/color]" % [type_label, amount])
	_ws_manager.send_action({
		"action": "take_loan",
		"loan_type": loan_type,
		"amount": amount
	})


func _on_factory_discount_confirmed(amount: int) -> void:
	_log("[color=yellow]💰 贴现 %dM 应收款...[/color]" % amount)
	_ws_manager.send_action({
		"action": "discount_receivable",
		"amount": amount
	})


func _on_market_panel_submit(strategies: Array) -> void:
	if strategies.is_empty():
		return
	_log("[color=green]📋 提交竞标策略: %d 个市场[/color]" % strategies.size())
	_ws_manager.send_action({
		"action": "submit_bidding",
		"strategies": strategies
	})
	_action_btn.text = "▶ 执行下一季度"
	_action_btn.disabled = false

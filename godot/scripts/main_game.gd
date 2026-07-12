extends Control
class_name MainGame

const T = preload("res://scripts/theme.gd")
const AnnualReportPopupScript = preload("res://scripts/popups/annual_report_popup.gd")
const OrderMeetingPopupScript = preload("res://scripts/popups/order_meeting_popup.gd")
const FactoryDialogScript = preload("res://scripts/popups/factory_dialog.gd")

@onready var _ws: Node = $/root/WebSocketManager

# ── Core state ──
var _state: Dictionary = {}
var _decision_type: String = ""

# ── Top bar widgets ──
var _year_lbl: Label
var _quarter_lbl: Label
var _cash_lbl: Label
var _asset_lbl: Label
var _rank_lbl: Label
var _phase_lbl: Label
var _phase_lbl2: Label

# ── Center widgets ──
var _prod_view: ProductionView
var _inv_dashboard: InventoryDashboard
var _market_container: VBoxContainer
var _rd_container: VBoxContainer
var _loan_container: VBoxContainer

# ── Action button ──
var _action_btn: Button

# ── Popups (stored for later show/hide) ──
var _bid_popup: Panel
var _bid_input: LineEdit
var _bid_confirm: Button
var _annual_popup: Variant
var _order_popup: Variant
var _factory_dialog: Variant


# ═══════════════ _ready ═══════════════

func _ready() -> void:
	custom_minimum_size = Vector2(960, 640)
	_ws.message_received.connect(_on_msg)
	_ws.connected.connect(func(): _ws.send_action({"action": "start_game"}))
	_build_layout()
	_ws.connect_to_server("game_001")


# ═══════════════ LAYOUT BUILD ═══════════════

func _build_layout() -> void:
	_build_topbar()
	_build_body()
	_build_bottombar()
	_build_bid_popup()


# ── TOP BAR (960x44 + 3px brand line) ──

func _build_topbar() -> void:
	var bar := _hbox(Vector2(0, 0), Vector2(960, 44), T.colors.white)
	add_child(bar)

	var logo := _label("🏢 创业沙盘", T.h1, T.colors.text_strong)
	bar.add_child(logo); bar.add_child(_sep())

	_year_lbl = _add(bar, "📅 第 1 年")
	_quarter_lbl = _add(bar, "Q1")
	_asset_lbl = _add(bar, "📊 资产 70M")

	var g := _gap(); bar.add_child(g)

	_rank_lbl = _add(bar, "🏆 排名 #1")
	_rank_lbl.add_theme_color_override("font_color", T.colors.text_medium)

	_cash_lbl = _add(bar, "💰 12M")
	_cash_lbl.add_theme_color_override("font_color", T.colors.white)
	var bg := _flat_bg(T.colors.brand, 14)
	bg.content_margin_left = 14; bg.content_margin_right = 14
	bg.content_margin_top = 5; bg.content_margin_bottom = 5
	_cash_lbl.add_theme_stylebox_override("normal", bg)

	var line := ColorRect.new(); line.color = T.colors.brand
	line.position = Vector2(0, 44); line.size = Vector2(960, 3)
	add_child(line)


# ── BODY (960x546) — three columns ──

func _build_body() -> void:
	var row := _hbox(Vector2(0, 47), Vector2(960, 542), T.colors.bg)
	add_child(row)

	row.add_child(_build_left())
	row.add_child(_vline())
	row.add_child(_build_center())
	row.add_child(_vline())
	row.add_child(_build_right())


func _build_left() -> Control:
	var c := VBoxContainer.new()
	c.custom_minimum_size = Vector2(200, 0)
	c.add_theme_color_override("background_color", T.colors.white)
	c.add_theme_constant_override("margin_left", 14)
	c.add_theme_constant_override("margin_top", 14)
	c.add_theme_constant_override("margin_right", 14)

	c.add_child(_section_head("📈 市场"))
	_market_container = VBoxContainer.new()
	c.add_child(_market_container)

	c.add_child(_spacer(12))
	c.add_child(_section_head("💳 贷款"))
	_loan_container = VBoxContainer.new()
	c.add_child(_loan_container)

	# marketing input row
	c.add_child(_spacer(14))
	var row := HBoxContainer.new()
	c.add_child(row)
	var inp := LineEdit.new(); inp.placeholder_text = "营销投入 M"; inp.text = "2"
	inp.custom_minimum_size = Vector2(60, 30); row.add_child(inp)
	_bid_input = inp
	var btn := _btn("提交竞标", T.colors.brand, T.colors.white); btn.pressed.connect(_on_bid_ok)
	btn.custom_minimum_size = Vector2(80, 30); row.add_child(btn)

	return c


func _build_center() -> Control:
	var c := VBoxContainer.new()
	c.size_flags_horizontal = SIZE_EXPAND_FILL
	c.add_theme_constant_override("separation", 12)
	c.add_theme_constant_override("margin_left", 16)
	c.add_theme_constant_override("margin_right", 16)
	c.add_theme_constant_override("margin_top", 14)
	c.add_theme_constant_override("margin_bottom", 14)

	# Factory card
	var fc := Panel.new(); fc.add_theme_stylebox_override("panel", _card_bg())
	fc.size_flags_horizontal = SIZE_EXPAND_FILL; fc.size_flags_vertical = SIZE_EXPAND_FILL
	c.add_child(fc)

	var fv := VBoxContainer.new()
	fv.add_theme_constant_override("margin_left", 16); fv.add_theme_constant_override("margin_top", 14)
	fc.add_child(fv)
	fv.add_child(_section_head("🏭 老工厂 (4条产线)"))
	_prod_view = ProductionView.new()
	fv.add_child(_prod_view)

	# Financial report card
	var rc := Panel.new(); rc.add_theme_stylebox_override("panel", _card_bg())
	rc.size_flags_horizontal = SIZE_EXPAND_FILL; rc.size_flags_vertical = SIZE_EXPAND_FILL
	c.add_child(rc)
	var rv := VBoxContainer.new()
	rv.add_theme_constant_override("margin_left", 16); rv.add_theme_constant_override("margin_top", 14)
	rc.add_child(rv)
	rv.add_child(_section_head("📦 库存"))
	_inv_dashboard = InventoryDashboard.new()
	rv.add_child(_inv_dashboard)

	return c


func _build_right() -> Control:
	var c := VBoxContainer.new()
	c.custom_minimum_size = Vector2(200, 0)
	c.add_theme_color_override("background_color", T.colors.white)
	c.add_theme_constant_override("margin_left", 14)
	c.add_theme_constant_override("margin_top", 14)
	c.add_theme_constant_override("margin_right", 14)

	c.add_child(_section_head("🔬 研发"))
	_rd_container = VBoxContainer.new()
	c.add_child(_rd_container)

	c.add_child(_spacer(12))
	c.add_child(_section_head("🤖 AI 对手"))
	# Placeholder info until real AI data comes
	c.add_child(_label("CEO    · 激进", T.small, T.colors.text_medium))
	c.add_child(_label("CFO    · 稳健", T.small, T.colors.text_medium))
	c.add_child(_label("COO    · 激进", T.small, T.colors.text_medium))
	c.add_child(_label("CMO    · 待定", T.small, T.colors.text_muted))

	return c


# ── BOTTOM BAR ──

func _build_bottombar() -> void:
	var bar := _hbox(Vector2(0, 589), Vector2(960, 51), T.colors.white)
	add_child(bar)

	var line := ColorRect.new(); line.color = T.colors.brand
	line.position = Vector2(0, 589); line.size = Vector2(960, 3)
	add_child(line)

	_action_btn = _btn("等待服务器...", T.colors.brand, T.colors.white)
	_action_btn.disabled = true; _action_btn.pressed.connect(_on_next_quarter)
	bar.add_child(_action_btn)

	bar.add_child(_btn_light("📊 年报")).pressed.connect(func():
		if _annual_popup: _annual_popup.show_report(_state))
	bar.add_child(_btn_light("📋 订货会"))
	bar.add_child(_btn_light("🏗️ 工厂")).pressed.connect(func():
		if _factory_dialog: _factory_dialog.show_dialog(_state))

	var g := _gap(); bar.add_child(g)

	_phase_lbl2 = _label("", T.small, T.colors.text_muted)
	bar.add_child(_phase_lbl2)


# ── BID POPUP ──

func _build_bid_popup() -> void:
	_bid_popup = Panel.new()
	_bid_popup.position = Vector2(230, 180); _bid_popup.size = Vector2(500, 180)
	_bid_popup.add_theme_stylebox_override("panel", _card_bg())
	_bid_popup.visible = false
	add_child(_bid_popup)

	var t := _label("📋 营销竞标", T.h1, T.colors.text_strong)
	t.position = Vector2(20, 20); _bid_popup.add_child(t)

	_bid_input = LineEdit.new()
	_bid_input.position = Vector2(20, 60); _bid_input.size = Vector2(180, 32)
	_bid_input.text = "2"
	_bid_popup.add_child(_bid_input)

	_bid_confirm = _btn("✅ 提交竞标", T.colors.brand, T.colors.white)
	_bid_confirm.position = Vector2(20, 110); _bid_confirm.pressed.connect(_on_bid_ok)
	_bid_popup.add_child(_bid_confirm)

	var cancel := _btn_light("取消"); cancel.position = Vector2(130, 110)
	cancel.pressed.connect(func(): _bid_popup.visible = false)
	_bid_popup.add_child(cancel)

	# Also init other popups into popup layer
	if has_node("PopupLayer"):
		_annual_popup = AnnualReportPopupScript.new(); _annual_popup.visible = false
		$PopupLayer.add_child(_annual_popup)
		_order_popup = OrderMeetingPopupScript.new(); _order_popup.visible = false
		$PopupLayer.add_child(_order_popup)
		_factory_dialog = FactoryDialogScript.new(); _factory_dialog.visible = false
		$PopupLayer.add_child(_factory_dialog)
		_factory_dialog.factory_ordered.connect(func(lt): _ws.send_action({"action": "order_production_line","line_type": lt}))
		_factory_dialog.loan_requested.connect(func(lt, amt): _ws.send_action({"action":"take_loan","loan_type":lt,"amount":amt}))
		_factory_dialog.discount_confirmed.connect(func(amt): _ws.send_action({"action":"discount_receivable","amount":amt}))


# ═══════════════ WS HANDLERS ═══════════════

func _on_msg(data: Dictionary) -> void:
	match data.get("event", ""):
		"state_update":
			_state = data.get("data", {}); _update_ui()
		"ask_decision":
			_bid_popup.visible = true
		"message":
			print("📢 ", data.get("data", ""))
		"game_over":
			print("💀 游戏结束: ", data.get("data", {}).get("reason", ""))
			_action_btn.disabled = true; _action_btn.text = "游戏结束"
		"phase_change":
			print("📅 ", data.get("data", {}).get("phase", ""))
		"annual_report":
			if _annual_popup: _annual_popup.show_report(data.get("data", {}))
		"order_meeting":
			if _order_popup:
				var od := data.get("data", {})
				_order_popup.show_orders(od.get("market",""), od.get("orders",[]), od.get("year",1))


func _update_ui() -> void:
	var s := _state
	_year_lbl.text = "📅 第 %d 年" % s.get("game_year", 1)
	_quarter_lbl.text = "Q%d" % s.get("game_quarter", 1)
	_cash_lbl.text = "💰 %dM" % s.get("cash", 0)

	var total_assets := s.get("cash", 0)
	for f in s.get("factories", []): total_assets += f.get("value", 0)
	_asset_lbl.text = "📊 资产 %dM" % total_assets

	var ph := s.get("phase", 1)
	var names := ["", "适应期", "市场竞争", "高阶经营", "复盘"]
	_phase_lbl2.text = "第%d节 · %s (%d-%d年)" % [ph, names[min(ph, 4)], _yr(ph)[0], _yr(ph)[1]]

	if _prod_view: _prod_view.update(s.get("factories", []))
	if _inv_dashboard: _inv_dashboard.update(s)

	# Update left market list
	for c in _market_container.get_children(): c.queue_free()
	for m in s.get("markets", []):
		if m.get("developed", false):
			_market_container.add_child(_label("✅ " + m.get("name","") + "  #" + str(m.get("rank",1)), T.body, T.colors.success))
		else:
			_market_container.add_child(_label("◦  " + m.get("name",""), T.body, T.colors.text_muted))

	# Update loan slots
	for c in _loan_container.get_children(): c.queue_free()
	var ll := s.get("long_term_loans", [])
	var sl := s.get("short_term_loans", [])
	_loan_container.add_child(_label("长贷 5%/年 | 已贷: %dM" % _sum_loans(ll), T.body, T.colors.text_medium))
	_loan_container.add_child(_label("短贷 10%/年 | 已贷: %dM" % _sum_loans(sl), T.body, T.colors.text_medium))

	# Update R&D
	for c in _rd_container.get_children(): c.queue_free()
	for rd in s.get("products_rd", []):
		var pname := "?"
		match str(rd.get("product","")):
			"ben_ma": pname = "🐴 奔马"
			"meng_hu": pname = "🐯 猛虎"
			"fei_ying": pname = "🦅 飞鹰"
			"tian_long": pname = "🐉 天龙"
		if rd.get("completed", false):
			_rd_container.add_child(_label(pname + " ✓", T.body, T.colors.success))
		else:
			_rd_container.add_child(_label("%s  %d/%d" % [pname, rd.get("progress",0), rd.get("total",0)], T.body, T.colors.text_medium))


# ═══════════════ BUTTON HANDLERS ═══════════════

func _on_bid_ok() -> void:
	_bid_popup.visible = false
	var val := int(_bid_input.text) if _bid_input.text.is_valid_int() else 2
	_ws.send_action({"action": "submit_bidding", "strategies": [{"market_name":"平城","marketing_spend":val}]})
	_action_btn.text = "▶ 执行下一季度"; _action_btn.disabled = false

func _on_next_quarter() -> void:
	_action_btn.disabled = true; _action_btn.text = "执行中..."
	_ws.send_action({"action": "next_quarter"})


# ═══════════════ PRIVATE HELPERS ═══════════════

func _hbox(pos: Vector2, sz: Vector2, bg: Color) -> HBoxContainer:
	var b := HBoxContainer.new(); b.position = pos; b.size = sz; b.custom_minimum_size = sz
	b.add_theme_color_override("background_color", bg); return b

func _label(txt: String, fs: int, cl: Color) -> Label:
	var l := Label.new(); l.text = txt
	l.add_theme_font_size_override("font_size", fs)
	l.add_theme_color_override("font_color", cl); return l

func _add(parent: Control, txt: String) -> Label:
	return _label(txt, T.body, T.colors.text_medium)

func _sep() -> Label:
	return _label("  |  ", T.body, T.colors.border)

func _gap() -> Control:
	var g := Control.new(); g.size_flags_horizontal = SIZE_EXPAND_FILL; return g

func _vline() -> ColorRect:
	var c := ColorRect.new(); c.color = T.colors.border; c.size.x = 1; return c

func _spacer(h: int) -> Control:
	var c := Control.new(); c.custom_minimum_size = Vector2(0, h); return c

func _card_bg() -> StyleBoxFlat:
	var sb := StyleBoxFlat.new(); sb.bg_color = T.colors.card; sb.set_corner_radius_all(8)
	sb.content_margin_left = 0; sb.content_margin_top = 0; return sb

func _flat_bg(cl: Color, r: int) -> StyleBoxFlat:
	var sb := StyleBoxFlat.new(); sb.bg_color = cl; sb.set_corner_radius_all(r); return sb

func _btn(txt: String, bg: Color, fg: Color) -> Button:
	var b := Button.new(); b.text = txt
	b.add_theme_font_size_override("font_size", T.body)
	b.add_theme_color_override("font_color", fg)
	var sb := StyleBoxFlat.new(); sb.bg_color = bg; sb.set_corner_radius_all(6)
	sb.content_margin_left = 18; sb.content_margin_right = 18
	sb.content_margin_top = 8; sb.content_margin_bottom = 8
	b.add_theme_stylebox_override("normal", sb); return b

func _btn_light(txt: String) -> Button:
	return _btn(txt, T.colors.white, T.colors.text_medium)

func _section_head(txt: String) -> Label:
	return _label(txt, T.h2, T.colors.text_strong)

func _yr(phase: int) -> Array[int]:
	match phase:
		1: return [1, 4]
		2: return [5, 7]
		3: return [8, 11]
		_: return [12, 12]

func _sum_loans(loans: Array) -> int:
	var s := 0
	for l in loans:
		s += l.get("amount", 0)
	return s

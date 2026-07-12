extends Control
## 市场状态面板——显示所有市场列表、开发状态、排名和上年度销售额。
##
## 通过 update(markets) 接收后端数据刷新显示。
## 顶部提供营销投入输入框用于提交竞标。

class_name MarketPanel

# ── Signals ──

## 用户点击"提交竞标"时触发，参数为竞标策略数组。
## 每个策略格式: { "market_name": String, "marketing_spend": int }
signal submit_bidding(strategies: Array)


# ── Constants ──

const COLOR_HEADER: Color = Color(0.90, 0.90, 0.90)
const COLOR_LABEL: Color = Color(0.70, 0.70, 0.70)
const COLOR_VALUE: Color = Color(0.90, 0.90, 0.30)
const COLOR_DEVELOPED_TAG: Color = Color(0.15, 0.75, 0.25)
const COLOR_UNDEVELOPED_TAG: Color = Color(0.70, 0.30, 0.30)
const COLOR_BORDER: Color = Color(0.30, 0.30, 0.30)

const ROW_HEIGHT: float = 24.0
const INPUT_WIDTH: float = 60.0
const PANEL_PADDING: float = 4.0


# ── Private Vars ──

var _market_container: VBoxContainer
var _marketing_input: LineEdit
var _submit_btn: Button


# ── Virtual Methods ──

func _ready() -> void:
	_build_ui()


# ── Public Methods ──

## 接收市场列表数据并刷新面板。
## markets: Array[Dictionary]
## 每个元素格式:
##   { "name": String, "developed": bool, "rank": int, "last_year_sales": int }
func update(markets: Array) -> void:
	# 清空旧列表
	for child in _market_container.get_children():
		_market_container.remove_child(child)
		child.queue_free()

	if markets.is_empty():
		_show_default_markets()
		return

	for market: Dictionary in markets:
		_add_market_row(market)


# ── Private Methods ──

func _build_ui() -> void:
	# 标题
	var header := Label.new()
	header.text = "📊 市场状态"
	header.add_theme_font_size_override("font_size", 14)
	header.add_theme_color_override("font_color", COLOR_HEADER)
	add_child(header)

	add_child(_make_separator())

	# 营销投入行
	var input_row := HBoxContainer.new()
	input_row.custom_minimum_size = Vector2(0, ROW_HEIGHT + 8)
	add_child(input_row)

	var input_label := Label.new()
	input_label.text = "营销投入:"
	input_label.add_theme_color_override("font_color", COLOR_LABEL)
	input_label.add_theme_font_size_override("font_size", 12)
	input_label.custom_minimum_size = Vector2(65, 0)
	input_row.add_child(input_label)

	_marketing_input = LineEdit.new()
	_marketing_input.placeholder_text = "2"
	_marketing_input.text = "2"
	_marketing_input.custom_minimum_size = Vector2(INPUT_WIDTH, 0)
	_marketing_input.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	input_row.add_child(_marketing_input)

	var unit_label := Label.new()
	unit_label.text = " M"
	unit_label.add_theme_color_override("font_color", COLOR_LABEL)
	unit_label.add_theme_font_size_override("font_size", 12)
	input_row.add_child(unit_label)

	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	input_row.add_child(spacer)

	_submit_btn = Button.new()
	_submit_btn.text = "提交竞标"
	_submit_btn.pressed.connect(_on_submit_bidding)
	input_row.add_child(_submit_btn)

	add_child(_make_separator())

	# 市场列表容器
	_market_container = VBoxContainer.new()
	_market_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_market_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	add_child(_market_container)

	# 初始显示默认市场（未开发状态）
	_show_default_markets()


static func _make_separator() -> HSeparator:
	var sep := HSeparator.new()
	sep.modulate = COLOR_BORDER
	return sep


func _show_default_markets() -> void:
	var default_markets: Array[Dictionary] = [
		{"name": "平城", "developed": false, "rank": 0, "last_year_sales": 0},
		{"name": "南城", "developed": false, "rank": 0, "last_year_sales": 0},
		{"name": "北城", "developed": false, "rank": 0, "last_year_sales": 0},
		{"name": "东城", "developed": false, "rank": 0, "last_year_sales": 0},
		{"name": "西城", "developed": false, "rank": 0, "last_year_sales": 0},
	]
	for market: Dictionary in default_markets:
		_add_market_row(market)


func _add_market_row(market: Dictionary) -> void:
	var row := HBoxContainer.new()
	row.custom_minimum_size = Vector2(0, ROW_HEIGHT)
	_market_container.add_child(row)

	var developed: bool = market.get("developed", false)

	var status_icon := Label.new()
	status_icon.text = "✅" if developed else "❌"
	status_icon.custom_minimum_size = Vector2(22, 0)
	row.add_child(status_icon)

	var name_label := Label.new()
	name_label.text = market.get("name", "未知")
	name_label.add_theme_color_override("font_color", COLOR_HEADER if developed else COLOR_LABEL)
	name_label.add_theme_font_size_override("font_size", 12)
	name_label.custom_minimum_size = Vector2(50, 0)
	row.add_child(name_label)

	if developed:
		var rank: int = market.get("rank", 0)
		var rank_label := Label.new()
		rank_label.text = "排名 #%d" % rank
		rank_label.add_theme_color_override("font_color", COLOR_VALUE)
		rank_label.add_theme_font_size_override("font_size", 11)
		rank_label.custom_minimum_size = Vector2(55, 0)
		row.add_child(rank_label)

		var sales: int = market.get("last_year_sales", 0)
		var sales_label := Label.new()
		sales_label.text = "去年销售 %dM" % sales
		sales_label.add_theme_color_override(
			"font_color", COLOR_DEVELOPED_TAG if sales > 0 else COLOR_LABEL
		)
		sales_label.add_theme_font_size_override("font_size", 11)
		sales_label.custom_minimum_size = Vector2(85, 0)
		row.add_child(sales_label)
	else:
		var undev_label := Label.new()
		undev_label.text = "未开发"
		undev_label.add_theme_color_override("font_color", COLOR_UNDEVELOPED_TAG)
		undev_label.add_theme_font_size_override("font_size", 11)
		undev_label.custom_minimum_size = Vector2(140, 0)
		row.add_child(undev_label)

	var row_spacer := Control.new()
	row_spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(row_spacer)


func _on_submit_bidding() -> void:
	var input_val: String = _marketing_input.text.strip_edges()
	var spend: int = int(input_val) if input_val.is_valid_int() else 2
	var strategies: Array[Dictionary] = [
		{"market_name": "平城", "marketing_spend": spend}
	]
	submit_bidding.emit(strategies)

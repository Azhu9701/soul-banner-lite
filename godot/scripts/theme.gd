extends Node
## OpenDesign 设计主题常量——供所有 UI 组件引用。
##
## 使用方式：T.colors.brand / T.fonts.head 等（通过 preload 引用）

class_name AppTheme

# ── Colors (OpenDesign a.light tokens) ──

const colors := {
	# Brand
	"brand": Color("#c7000b"),
	"brand_light": Color("#fecaca"),
	"brand_hover": Color("#a80009"),

	# Greys
	"white": Color("#ffffff"),
	"bg": Color("#edf0f5"),
	"card": Color("#ffffff"),
	"border": Color("#d5dae0"),
	"border_light": Color("#eef0f4"),

	# Text
	"text": Color("#1a1a1a"),
	"text_strong": Color("#000000"),
	"text_medium": Color("#2d3748"),
	"text_muted": Color("#64748b"),

	# Semantic
	"success": Color("#16a34a"),
	"success_bg": Color("#dcfce7"),
	"success_text": Color("#14532d"),
	"warning": Color("#f59e0b"),
	"warning_bg": Color("#fef3c7"),
	"warning_text": Color("#854d0e"),
	"danger": Color("#dc2626"),
	"danger_bg": Color("#fecaca"),
	"danger_text": Color("#7f1d1d"),

	# Product
	"benma": Color("#c7000b"),
	"menghu": Color("#1e6fe0"),
	"feiying": Color("#c28600"),
	"tianlong": Color("#7c3aed"),

	# Status
	"producing": Color("#c7000b"),
	"building": Color("#1e6fe0"),
	"switching": Color("#c28600"),
	"idle": Color("#c4cad4"),
}

# ── Fonts ──

const fonts := {
	"head": 14,
	"body": 13,
	"small": 12,
	"tiny": 10,
	"h1": 18,
	"h2": 16,
}

# ── Spacing ──

const space := {
	"xs": 4,
	"sm": 8,
	"md": 12,
	"lg": 16,
	"xl": 24,
}

# ── Helpers ──

## 给 Control 节点设置基础样式
static func style_card(node: Control) -> void:
	node.add_theme_color_override("background_color", colors.card)
	node.add_theme_stylebox_override("border", _make_stylebox(colors.border, 8))

## 创建圆角边框
static func _make_stylebox(color: Color, radius: int) -> StyleBoxFlat:
	var sb := StyleBoxFlat.new()
	sb.bg_color = color
	sb.set_corner_radius_all(radius)
	return sb

## 获取产品对应颜色
static func product_color(product_name: String) -> Color:
	match product_name.to_lower():
		"ben_ma", "奔马": return colors.benma
		"meng_hu", "猛虎": return colors.menghu
		"fei_ying", "飞鹰": return colors.feiying
		"tian_long", "天龙": return colors.tianlong
	return colors.idle

## 产品对应图标
static func product_icon(product_name: String) -> String:
	match product_name.to_lower():
		"ben_ma", "奔马": return "🐴"
		"meng_hu", "猛虎": return "🐯"
		"fei_ying", "飞鹰": return "🦅"
		"tian_long", "天龙": return "🐉"
	return "⬜"

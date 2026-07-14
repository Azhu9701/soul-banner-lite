extends Node
## OpenDesign a.light 设计主题常量。
##
## 使用：const T = preload("res://scripts/theme.gd")
## 访问：T.colors.brand / T.h1 / T.body 等

class_name AppTheme

# ── Colors (OpenDesign tokens) ──

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

	# Products
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

# ── Font sizes ──

const fonts := {
	"h1": 18,
	"h2": 16,
	"head": 14,
	"body": 13,
	"small": 12,
	"tiny": 10,
}

# ── Shorthand constants (T.h1, T.body, etc.) ──

const h1: int = 18
const h2: int = 16
const head: int = 14
const body: int = 13
const small: int = 12
const tiny: int = 10

# ── Spacing ──

const xs: int = 4
const sm: int = 8
const md: int = 12
const lg: int = 16
const xl: int = 24

# ── Static helpers ──

static func style_card(node: Control) -> void:
	node.add_theme_color_override("background_color", colors.card)

static func product_color(name: String) -> Color:
	match name.to_lower():
		"ben_ma", "奔马": return colors.benma
		"meng_hu", "猛虎": return colors.menghu
		"fei_ying", "飞鹰": return colors.feiying
		"tian_long", "天龙": return colors.tianlong
	return colors.idle

static func product_icon(name: String) -> String:
	match name.to_lower():
		"ben_ma", "奔马": return "🐴"
		"meng_hu", "猛虎": return "🐯"
		"fei_ying", "飞鹰": return "🦅"
		"tian_long", "天龙": return "🐉"
	return "⬜"

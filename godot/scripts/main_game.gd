extends Control
## 主游戏场景，管理顶部状态栏/中部产线/底部面板/弹窗系统

@onready var top_bar: Control = $TopBar
@onready var production_area: Control = $ProductionArea
@onready var bottom_panel: Control = $BottomPanel
@onready var popup_layer: Control = $PopupLayer
@onready var ws_manager: Node = $/root/WebSocketManager

var game_state: Dictionary = {}

func _ready() -> void:
	ws_manager.message_received.connect(_on_ws_message)
	ws_manager.connected.connect(_on_ws_connected)
	# 连接到游戏服务器
	ws_manager.connect_to_server("game_001")

func _on_ws_connected() -> void:
	print("已连接到游戏服务器")
	# 发送开始游戏指令
	ws_manager.send_action({"action": "start_game"})

func _on_ws_message(data: Dictionary) -> void:
	var event_type: String = data.get("event", "")
	match event_type:
		"state_update":
			game_state = data.get("data", {})
			_update_ui()
		"ask_decision":
			_show_decision_prompt(data.get("data", {}))
		"message":
			_show_message(data.get("data", ""))
		"game_over":
			_show_game_over(data.get("data", {}).get("reason", "未知原因"))
		"annual_report":
			_show_annual_report(data.get("data", {}))
		"phase_change":
			_update_phase_display(data.get("data", {}))

func _update_ui() -> void:
	top_bar.update(
		game_state.get("game_year", 1),
		game_state.get("game_quarter", 1),
		game_state.get("cash", 0)
	)
	production_area.update(game_state.get("factories", []))
	bottom_panel.update(game_state)

func _show_decision_prompt(data: Dictionary) -> void:
	var decision_type: String = data.get("decision_type", "")
	if decision_type == "bidding":
		_open_bidding_dialog()

func _open_bidding_dialog() -> void:
	# 占位：后面实现竞标弹窗
	print("打开竞标策略弹窗")

func _show_message(text: String) -> void:
	print("系统消息: ", text)

func _show_game_over(reason: String) -> void:
	print("游戏结束: ", reason)

func _show_annual_report(report: Dictionary) -> void:
	print("年度报告: ", report)

func _update_phase_display(data: Dictionary) -> void:
	pass

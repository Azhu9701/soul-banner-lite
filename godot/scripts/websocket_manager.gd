extends Node
## WebSocket 连接管理器，负责与 Rust 后端通信。

var socket: WebSocketPeer = WebSocketPeer.new()
var is_connected: bool = false
var server_url: String = "ws://127.0.0.1:3097/ws/game/"
var game_id: String = ""

signal connected()
signal disconnected()
signal message_received(data: Dictionary)
signal connection_error()

func connect_to_server(g_id: String) -> void:
	game_id = g_id
	var url: String = server_url + g_id
	var err: Error = socket.connect_to_url(url)
	if err != OK:
		push_error("WebSocket 连接失败: ", err)

func send_action(action: Dictionary) -> void:
	if is_connected:
		var json_str: String = JSON.stringify(action)
		socket.send_text(json_str)

func _process(delta: float) -> void:
	socket.poll()
	if socket.get_ready_state() == WebSocketPeer.STATE_OPEN and not is_connected:
		is_connected = true
		connected.emit()
	elif socket.get_ready_state() == WebSocketPeer.STATE_CLOSED and is_connected:
		is_connected = false
		disconnected.emit()

	while socket.get_available_packet_count() > 0:
		var packet: PackedByteArray = socket.get_packet()
		var json_str: String = packet.get_string_from_utf8()
		var data: Dictionary = JSON.parse_string(json_str) as Dictionary
		if data.is_empty():
			push_warning("无法解析WS消息: ", json_str)
			continue
		message_received.emit(data)

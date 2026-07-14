extends Node
## WebSocket 连接管理器——负责与 Rust 游戏后端通信。
##
## 作为 Autoload 单例运行，生命周期覆盖整个游戏过程。
## 所有游戏模块通过信号接收服务端事件。

# ── Signals ──

## 成功连接到游戏服务器
signal connected()
## 与服务器的连接断开
signal disconnected()
## 收到服务端的 JSON 消息（已解析为 Dictionary）
signal message_received(data: Dictionary)
## 连接失败
signal connection_failed()

# ── Constants ──

const DEFAULT_SERVER_URL: String = "ws://127.0.0.1:3096/ws/game/"
const RECONNECT_DELAY_SEC: float = 3.0
const MAX_RECONNECT_ATTEMPTS: int = 5

# ── Public vars ──

var server_url: String = DEFAULT_SERVER_URL
var game_id: String = ""
var is_connected: bool = false

# ── Private vars ─-

var _socket: WebSocketPeer = WebSocketPeer.new()
var _reconnect_attempts: int = 0
var _reconnect_timer: float = 0.0


# ── Virtual Methods ──

func _process(delta: float) -> void:
	_socket.poll()
	_update_connection_state()
	_receive_messages()
	_try_reconnect(delta)


# ── Public Methods ──

## 连接到指定游戏 ID 的服务器会话。
func connect_to_server(g_id: String) -> void:
	game_id = g_id
	var url: String = server_url + g_id
	var err: Error = _socket.connect_to_url(url)
	if err != OK:
		push_error("WebSocket 连接失败: ", err)
		connection_failed.emit()
		return
	_reconnect_attempts = 0


## 向服务端发送操作指令（JSON 格式）。
func send_action(action: Dictionary) -> void:
	if not is_connected:
		push_warning("未连接服务器，无法发送消息")
		return
	var json_str: String = JSON.stringify(action)
	_socket.send_text(json_str)


## 主动断开连接。
func disconnect_from_server() -> void:
	_socket.close()
	is_connected = false
	disconnected.emit()


# ── Private Methods ──

func _update_connection_state() -> void:
	var state: int = _socket.get_ready_state()
	match state:
		WebSocketPeer.STATE_OPEN:
			if not is_connected:
				is_connected = true
				_reconnect_attempts = 0
				connected.emit()
		WebSocketPeer.STATE_CLOSED, WebSocketPeer.STATE_CLOSING:
			if is_connected:
				is_connected = false
				disconnected.emit()
				_schedule_reconnect()


func _receive_messages() -> void:
	while _socket.get_available_packet_count() > 0:
		var packet: PackedByteArray = _socket.get_packet()
		var json_str: String = packet.get_string_from_utf8()
		var data: Dictionary = JSON.parse_string(json_str) as Dictionary
		if data.is_empty():
			push_warning("无法解析 WS 消息: ", json_str)
			continue
		message_received.emit(data)


func _schedule_reconnect() -> void:
	if _reconnect_attempts >= MAX_RECONNECT_ATTEMPTS:
		push_error("重连次数已达上限（%d 次）" % MAX_RECONNECT_ATTEMPTS)
		return
	_reconnect_timer = RECONNECT_DELAY_SEC


func _try_reconnect(delta: float) -> void:
	if _reconnect_timer <= 0.0:
		return
	_reconnect_timer -= delta
	if _reconnect_timer <= 0.0:
		_reconnect_attempts += 1
		print("正在重连（第 %d/%d 次）..." % [_reconnect_attempts, MAX_RECONNECT_ATTEMPTS])
		connect_to_server(game_id)

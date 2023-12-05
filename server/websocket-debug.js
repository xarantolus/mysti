// Basic websocket handler in javascript for debugging
var debug_socket = new WebSocket("ws://127.0.0.1:8080/ws");

debug_socket.onclose = function (event) {
	console.log("Socket closed");
};

debug_socket.onmessage = function (event) {
	console.log("Server: " + event.data);
};

debug_socket.onopen = function (event) {
	console.log("Socket opened");
};

function send(message) {
	debug_socket.send(message);
}

function sendJSON(message) {
	debug_socket.send(JSON.stringify(message));
}

function close() {
	debug_socket.close();
}

extends Node

var recorded: Array[String] = []

func _save_last_map_path(path: String) -> void:
    recorded.append(path)
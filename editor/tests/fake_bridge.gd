extends Node

var store: Dictionary = {}

func set_config_value(key: String, value: String) -> void:
    store[key] = value

func get_config_value(key: String) -> String:
    return str(store.get(key, ""))
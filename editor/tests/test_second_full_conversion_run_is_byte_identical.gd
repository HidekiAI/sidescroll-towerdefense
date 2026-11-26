extends SceneTree
# Headless guard: run the whole conversion twice into throwaway dirs and
# compare emitted bytes, not decoded pixels. A second pass must upsert 30 terrain entries rather
# than grow to 46, and no artifact may contain a launch timestamp.
# Single-process writes into two separate dirs; no shared mutable state, so no race applies.

const TOOL_PATH: String = "res://scripts/slope_conversion_tool.gd"
const FIRST_RUN_DIR: String = "user://data/tools/sstd_conversion_guard_a"
const SECOND_RUN_DIR: String = "user://data/tools/sstd_conversion_guard_b"
const EXPECTED_ENTRY_COUNT: int = 30

var _failures: int = 0


func _initialize() -> void:
    _run.call_deferred()


func _check(condition: bool, description: String) -> void:
    if not condition:
        _failures += 1
        printerr("FAIL: " + description)


func _wipe_dir(path: String) -> void:
    if not DirAccess.dir_exists_absolute(path):
        return
    var dir: DirAccess = DirAccess.open(path)
    if dir == null:
        return
    dir.list_dir_begin()
    var entry_name: String = dir.get_next()
    while entry_name != "":
        var child_path: String = path + "/" + entry_name
        if dir.current_is_dir():
            _wipe_dir(child_path)
        else:
            dir.remove(entry_name)
        entry_name = dir.get_next()
    dir.list_dir_end()
    DirAccess.remove_absolute(ProjectSettings.globalize_path(path))


func _collect_files(path: String, relative_prefix: String, files: PackedStringArray) -> void:
    var dir: DirAccess = DirAccess.open(path)
    if dir == null:
        return
    dir.list_dir_begin()
    var entry_name: String = dir.get_next()
    while entry_name != "":
        var child_path: String = path + "/" + entry_name
        if dir.current_is_dir():
            _collect_files(child_path, relative_prefix + entry_name + "/", files)
        else:
            files.append(relative_prefix + entry_name)
        entry_name = dir.get_next()
    dir.list_dir_end()


func _run() -> void:
    var script: GDScript = load(TOOL_PATH) as GDScript
    if script == null:
        _check(false, "SlopeConversionTool script exists at %s" % TOOL_PATH)
        _finish()
        return
    var tool: Object = script.new()
    if not tool.has_method("convert_all"):
        _check(false, "SlopeConversionTool.convert_all exists")
        _finish()
        return

    _wipe_dir(FIRST_RUN_DIR)
    _wipe_dir(SECOND_RUN_DIR)
    DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(FIRST_RUN_DIR))
    DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(SECOND_RUN_DIR))
    var first: Variant = tool.call("convert_all", FIRST_RUN_DIR)
    var second: Variant = tool.call("convert_all", SECOND_RUN_DIR)
    _check(first is Dictionary and second is Dictionary, "both conversion runs return a result")
    if not (first is Dictionary and second is Dictionary):
        _finish()
        return

    var first_result: Dictionary = first
    var second_result: Dictionary = second
    _check(bool(first_result.get("ok", false)), "first conversion reports success: %s" % str(first_result.get("error", "")))
    _check(bool(second_result.get("ok", false)), "second conversion reports success: %s" % str(second_result.get("error", "")))
    _check(int(first_result.get("entries", -1)) == EXPECTED_ENTRY_COUNT,
        "first run registers %d entries" % EXPECTED_ENTRY_COUNT)
    _check(int(second_result.get("entries", -1)) == EXPECTED_ENTRY_COUNT,
        "second run upserts rather than growing past %d entries" % EXPECTED_ENTRY_COUNT)

    var first_files: PackedStringArray = PackedStringArray()
    var second_files: PackedStringArray = PackedStringArray()
    _collect_files(FIRST_RUN_DIR, "", first_files)
    _collect_files(SECOND_RUN_DIR, "", second_files)
    first_files.sort()
    second_files.sort()
    _check(first_files.size() > 0, "first run emitted at least one artifact")
    _check(first_files == second_files,
        "both runs emitted the same file set, got %d vs %d" % [first_files.size(), second_files.size()])
    for relative_path in first_files:
        if not second_files.has(relative_path):
            continue
        var first_bytes: PackedByteArray = FileAccess.get_file_as_bytes(FIRST_RUN_DIR + "/" + relative_path)
        var second_bytes: PackedByteArray = FileAccess.get_file_as_bytes(SECOND_RUN_DIR + "/" + relative_path)
        _check(first_bytes == second_bytes, "%s is byte-identical across runs" % relative_path)

    _finish()


func _finish() -> void:
    print("second_full_conversion failures=%d" % _failures)
    quit(0 if _failures == 0 else 1)

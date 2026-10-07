extends SceneTree

# Regression guard for the pre-Run bridge auto-build: build_bridge() must
# return true only when scripts/build-bridge.sh really ran, and every real run
# must leave a strictly newer editor/rust/libsstd_editor_bridge.so behind
# (build-bridge.sh ends in an unconditional cp).
#
# The guard builds twice with more than a second in between because file
# mtimes are second-granular: baseline is the first build's mtime and the
# second build must strictly exceed it. A build_bridge() mutated into a no-op
# never moves the mtime, so the strict ">" comparison is the non-vacuity
# tripwire -- verified by mutation, not by reading.

const BridgeAutoBuildPlugin := preload("res://addons/bridge_autobuild/plugin.gd")
const BRIDGE_SO_PATH: String = "res://rust/libsstd_editor_bridge.so"
const MTIME_TICK_GUARD_MSEC: int = 1100

var _failures: int = 0
var _completed: Array[String] = []

func check(condition: bool, message: String) -> void:
    if condition:
        print("ok: " + message)
    else:
        _failures += 1
        print("FAIL: " + message)

func _initialize() -> void:
    _run.call_deferred()

func _run() -> void:
    print("--- pre-Run bridge auto-build ---")

    var first_build_ok: bool = BridgeAutoBuildPlugin.build_bridge()
    check(first_build_ok, "first build_bridge() returns true")
    var baseline_mtime: int = FileAccess.get_modified_time(BRIDGE_SO_PATH)
    check(baseline_mtime > 0, "bridge .so exists after the first build (mtime %d)" % baseline_mtime)

    OS.delay_msec(MTIME_TICK_GUARD_MSEC)

    var second_build_ok: bool = BridgeAutoBuildPlugin.build_bridge()
    check(second_build_ok, "second build_bridge() returns true")
    var final_mtime: int = FileAccess.get_modified_time(BRIDGE_SO_PATH)
    check(final_mtime > baseline_mtime, "bridge .so mtime strictly advanced (%d -> %d)" % [baseline_mtime, final_mtime])

    _completed.append("bridge_autobuild")
    # Truncation check: without it, a guard that aborts on a runtime error is
    # silently reported as a pass by any runner that keeps going.
    for guard_name in ["bridge_autobuild"]:
        check(_completed.has(guard_name), "guard '%s' ran to completion" % guard_name)

    print("=== bridge autobuild done, failures=%d === " % _failures)
    quit(0 if _failures == 0 else 1)

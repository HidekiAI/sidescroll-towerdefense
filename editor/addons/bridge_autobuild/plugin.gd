@tool
extends EditorPlugin

# Pre-Run auto-build for the Rust GDExtension bridge. The editor calls _build()
# right before launching the game; returning false aborts the Run, so a failed
# bridge build never lets the game start against a stale .so.
#
# build_bridge() is static because EditorPlugin cannot be instantiated outside
# the editor ("Class 'EditorPlugin' can only be instantiated by editor"), so
# editor/tests/test_bridge_autobuild.gd calls the static directly; _build()
# below is the one-line delegation to it.
#
# Freshness of the .so is cargo's fingerprinting; this hook is only the
# trigger.
#
# ponytail: OS.execute blocks the editor UI thread for the build duration --
# deliberate, warm debug builds are sub-second; upgrade path is EditorProgress
# plus a Thread if cold builds ever hurt.

const LOG_PREFIX: String = "[bridge-autobuild]"
const BUILD_SCRIPT: String = "scripts/build-bridge.sh"

static func build_bridge() -> bool:
    var repo_root: String = ProjectSettings.globalize_path("res://").path_join("..")
    # 2>&1 merges cargo's stderr into stdout: OS.execute collects the two
    # streams as separate elements, which would print all echo lines before all
    # cargo lines and hide the chronological order of the rebuild.
    var build_command: String = "export PATH=\"$HOME/.cargo/bin:$PATH\" && cd '%s' && exec %s 2>&1" % [repo_root, BUILD_SCRIPT]
    print("%s building %s (repo root: %s)" % [LOG_PREFIX, BUILD_SCRIPT, repo_root])
    var build_started_msec: int = Time.get_ticks_msec()
    var output: Array = []
    var exit_code: int = OS.execute("bash", ["-c", build_command], output, true)
    for output_element in output:
        for output_line in str(output_element).split("\n"):
            if not output_line.is_empty():
                print("%s %s" % [LOG_PREFIX, output_line])
    var build_elapsed_msec: int = Time.get_ticks_msec() - build_started_msec
    if exit_code != 0:
        push_error("%s build FAILED (exit code %d after %d ms) -- Run aborted" % [LOG_PREFIX, exit_code, build_elapsed_msec])
        return false
    print("%s build OK (%d ms)" % [LOG_PREFIX, build_elapsed_msec])
    return true


func _build() -> bool:
    return build_bridge()

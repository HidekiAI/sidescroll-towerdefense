extends SceneTree
# RED (corrected) probe — ref #68 #74
# Tests the ACTUAL accommodation: Simulator tab must expose a scrollbar that
# drives the backdrop stack's Camera2D 1:1 target, and the Near band must trail
# at scroll_scale.x = 0.9. It intentionally exercises the TAB WIRING (not the
# already-green Parallax2D native contract), so it must fail today:
# main.tscn's Simulator tab carries no HScrollBar node at all.

const MAIN := "res://scenes/main.tscn"
const SIM_PATH := "TabContainer/Simulator"

func _init() -> void:
    var main: Node = (load(MAIN) as PackedScene).instantiate()
    root.add_child(main)

    var tabs: TabContainer = main.get_node("TabContainer") as TabContainer
    tabs.set_current_tab(4)  # 4 == Simulator
    await process_frame

    var sim: Control = main.get_node(SIM_PATH) as Control
    var scrollbar := sim.get_node_or_null("BackdropScroll")

    if scrollbar == null:
        print("[RED] Simulator tab has no BackdropScroll bar to drive the Camera2D")
        quit(1)
        return

    var camera: Camera2D = sim.get_node("BackdropStrip/Camera2D") as Camera2D
    var near: Parallax2D = sim.get_node("BackdropStrip/Backdrops/L2_near") as Parallax2D

    # The 6400px strip scrolls 1:1; scrub to 800px.
    scrollbar.set_value(800.0)

    # 1:1 camera contract.
    if not is_equal_approx(camera.position.x, 800.0):
        print("[RED] camera did NOT follow scrollbar 1:1: got %.1f want 800.0" % camera.position.x)
        quit(1)
        return
    # Near band trails at 0.9x — the Parallax2D scroll_scale static contract
    # (position.x itself is a Parallax2D-derived runtime value that only
    # accumulates from camera motion across render frames, so a single
    # synchronous scrub can never be read on it; the .tscn pins the truth).
    if not is_equal_approx(near.scroll_scale.x, 0.9):
        print("[RED] Near band scroll_scale.x is not the 0.9x trail contract: got %.2f want 0.9" % near.scroll_scale.x)
        quit(1)
        return

    print("[GREEN] Simulator scrollbar drives Camera2D 1:1; Near trails 0.9x")
    quit(0)

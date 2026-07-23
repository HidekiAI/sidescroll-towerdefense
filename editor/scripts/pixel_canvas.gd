extends Control

var _image: Image
var _zoom: int = 8
var _grid_w: int = 32
var _grid_h: int = 32
var selected_color: Color = Color.WHITE
var _painting: bool = false

signal pixel_changed(x: int, y: int, color: Color)

func set_image(img: Image) -> void:
    _image = img
    _grid_w = img.get_width()
    _grid_h = img.get_height()
    _auto_zoom()
    queue_redraw()

func fill(color: Color) -> void:
    if not _image:
        return
    for y in _grid_h:
        for x in _grid_w:
            _image.set_pixel(x, y, color)
    queue_redraw()

func _auto_zoom() -> void:
    var available := custom_minimum_size
    if available.x <= 0 or available.y <= 0:
        available = Vector2(256, 256)
    var zoom_x := int(available.x / _grid_w)
    var zoom_y := int(available.y / _grid_h)
    _zoom = max(2, min(zoom_x, zoom_y))

func get_canvas_size() -> Vector2:
    return Vector2(_grid_w * _zoom, _grid_h * _zoom)

func _draw() -> void:
    if not _image:
        return

    for y in _grid_h:
        for x in _grid_w:
            var color := _image.get_pixel(x, y)
            var rect := Rect2(x * _zoom, y * _zoom, _zoom, _zoom)
            draw_rect(rect, color)
            draw_rect(rect, Color(0.2, 0.2, 0.2, 0.15), false, 1)

func _gui_input(event: InputEvent) -> void:
    if not _image:
        return
    if event is InputEventMouseButton and event.pressed:
        match event.button_index:
            MOUSE_BUTTON_LEFT:
                _painting = true
                _paint_at(event.position)
            MOUSE_BUTTON_RIGHT:
                _erase_at(event.position)
            _:
                pass
    elif event is InputEventMouseButton and not event.pressed:
        _painting = false
    elif event is InputEventMouseMotion and _painting:
        _paint_at(event.position)

func _paint_at(pos: Vector2) -> void:
    var tx := int(pos.x / _zoom)
    var ty := int(pos.y / _zoom)
    if tx < 0 or tx >= _grid_w or ty < 0 or ty >= _grid_h:
        return
    var current := _image.get_pixel(tx, ty)
    if current.is_equal_approx(selected_color):
        return
    _image.set_pixel(tx, ty, selected_color)
    pixel_changed.emit(tx, ty, selected_color)
    queue_redraw()

func _erase_at(pos: Vector2) -> void:
    var tx := int(pos.x / _zoom)
    var ty := int(pos.y / _zoom)
    if tx < 0 or tx >= _grid_w or ty < 0 or ty >= _grid_h:
        return
    _image.set_pixel(tx, ty, Color(0, 0, 0, 0))
    pixel_changed.emit(tx, ty, Color(0, 0, 0, 0))
    queue_redraw()

func save_png(path: String) -> void:
    if not _image:
        return
    var dir := path.get_base_dir()
    DirAccess.make_dir_recursive_absolute(dir)
    _image.save_png(path)

func load_png(path: String) -> bool:
    var img: Image = Image.new()
    if img.load(path) != OK:
        return false
    set_image(img)
    return true

func get_image() -> Image:
    return _image

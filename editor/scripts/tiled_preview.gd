extends Control

var _image: Image
var _tile_w: int = 32
var _tile_h: int = 32
var _zoom: int = 2

func set_image(img: Image) -> void:
    _image = img
    _tile_w = img.get_width()
    _tile_h = img.get_height()
    _auto_zoom()
    queue_redraw()

func set_zoom(z: int) -> void:
    _zoom = max(1, z)
    queue_redraw()

func _auto_zoom() -> void:
    var available := custom_minimum_size
    if available.x > 0 and available.y > 0:
        var zx := int(available.x / (_tile_w * 3))
        var zy := int(available.y / (_tile_h * 3))
        _zoom = max(1, min(zx, zy))

func _draw() -> void:
    if not _image:
        return

    for row in 3:
        for col in 3:
            var ox := col * _tile_w * _zoom
            var oy := row * _tile_h * _zoom
            for y in _tile_h:
                for x in _tile_w:
                    var sx := (x + col * _tile_w) % _tile_w
                    var sy := (y + row * _tile_h) % _tile_h
                    var color := _image.get_pixel(sx, sy)
                    draw_rect(Rect2(ox + x * _zoom, oy + y * _zoom, _zoom, _zoom), color)

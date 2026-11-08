extends Node2D
func _draw() -> void:
	var c = Color(0.9,0.8,0.2)
	draw_circle(Vector2(0,-6), 3, c)
	draw_line(Vector2(0,-3), Vector2(0,2), c, 2)
	draw_line(Vector2(0,-1), Vector2(-2,1), c, 1)
	draw_line(Vector2(0,-1), Vector2(2,1), c, 1)
	draw_line(Vector2(0,2), Vector2(-1,5), c, 1)
	draw_line(Vector2(0,2), Vector2(1,5), c, 1)

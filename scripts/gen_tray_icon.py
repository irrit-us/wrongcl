"""Generate the tray icon as two sharp V shapes clipped by a reference circle.

Geometry rules:
- The centerline is a single left-to-right stroke with alternating up/down turns.
- The first V is symmetric around a vertical axis.
- The second V leans to the right.
- Both V apex angles are 36 degrees.
- Visible circle arcs come from clipping an extended centerline, not round caps.
- Stroke width grows monotonically along arc length using the golden-ratio
  conjugate tau = (sqrt(5) - 1) / 2.

Run:
    python3 scripts/gen_tray_icon.py
"""

from __future__ import annotations

import math
from fractions import Fraction
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw

# --- Parameters --------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parents[1]
PNG_OUTPUT_PATH = REPO_ROOT / "assets" / "tray_icon.png"
ICO_OUTPUT_PATH = REPO_ROOT / "assets" / "tray_icon.ico"
SVG_OUTPUT_PATH = REPO_ROOT / "assets" / "tray_icon.svg"

PNG_SIZE = 1024                       # large PNG for easier inspection/editing
ICO_SOURCE_SIZE = 256                 # render size used to build the ICO
BACKGROUND = (0, 0, 0, 0)             # transparent RGBA
FILL_COLOR = (0x1A, 0x1A, 0x1A, 0xFF)

REFERENCE_CIRCLE_RADIUS_RATIO = Fraction(1, 2)  # inscribed circle, tangent to the canvas border
MAX_STROKE_WIDTH_TO_RADIUS_RATIO = Fraction(23, 100)

V_APEX_ANGLE_RAD = math.pi / 5
LEFT_V_AXIS_RAD = math.pi / 2
RIGHT_V_AXIS_RAD = LEFT_V_AXIS_RAD - V_APEX_ANGLE_RAD / 3

TAU = (math.sqrt(5.0) - 1.0) / 2.0
WIDTH_SCALE_START = 2.0 * TAU
WIDTH_SCALE_DELTA = 1.0 - TAU   # tau^2 = 1 - tau

# All normalized coordinates use the reference circle radius as unit length and
# math coordinates (positive Y = up). The left V's vertical symmetry axis passes
# through the circle points at ±2π/3, so pinning the lower apex to -2π/3 fixes
# the axis at x = -1/2.
LEFT_APEX_CIRCLE_ANGLE_RAD = -2 * math.pi / 3
RIGHT_APEX_DISTANCE = Fraction(32, 25)
END_EXTENSION = Fraction(6, 5)        # extend past circle so clipping reveals arcs

# Debug aid: draw the reference circle over the final icon.
DRAW_GUIDE_CIRCLE = False
GUIDE_COLOR = (0, 0, 0, 48)

SUPERSAMPLE = 4
ICO_SIZES = [(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (24, 24), (16, 16)]


# --- Drawing -----------------------------------------------------------------

Point = tuple[float, float]


def vec(angle_rad: float) -> Point:
    return (math.cos(angle_rad), math.sin(angle_rad))


def add(a: Point, b: Point) -> Point:
    return (a[0] + b[0], a[1] + b[1])


def sub(a: Point, b: Point) -> Point:
    return (a[0] - b[0], a[1] - b[1])


def scale(v: Point, factor: float) -> Point:
    return (v[0] * factor, v[1] * factor)


def unit(v: Point) -> Point:
    length = math.hypot(v[0], v[1])
    if length == 0:
        raise ValueError("zero-length vector")
    return (v[0] / length, v[1] / length)


def perp_left(v: Point) -> Point:
    return (-v[1], v[0])


def line_intersection(point_a: Point, dir_a: Point, point_b: Point, dir_b: Point) -> Point:
    """Return the intersection of two parametric lines."""
    det = dir_b[0] * dir_a[1] - dir_a[0] * dir_b[1]
    if abs(det) < 1e-9:
        return ((point_a[0] + point_b[0]) / 2.0, (point_a[1] + point_b[1]) / 2.0)

    delta = sub(point_b, point_a)
    t = (delta[0] * (-dir_b[1]) + delta[1] * dir_b[0]) / det
    return add(point_a, scale(dir_a, t))


def ray_circle_hit(origin: Point, angle_rad: float, radius: float = 1.0) -> Point:
    """Intersect a ray starting inside the unit circle with the circle boundary."""
    direction = vec(angle_rad)
    ox, oy = origin
    dx, dy = direction

    b = 2.0 * (ox * dx + oy * dy)
    c = ox * ox + oy * oy - radius * radius
    disc = b * b - 4.0 * c
    if disc < 0:
        raise ValueError("ray does not intersect the reference circle")

    sqrt_disc = math.sqrt(disc)
    t1 = (-b - sqrt_disc) / 2.0
    t2 = (-b + sqrt_disc) / 2.0
    t = max(candidate for candidate in (t1, t2) if candidate > 0)
    return add(origin, scale(direction, t))


def line_circle_other_hit(point_on_circle: Point, angle_rad: float, radius: float = 1.0) -> Point:
    """Return the other circle intersection of the infinite line through a point."""
    direction = vec(angle_rad)
    px, py = point_on_circle
    dx, dy = direction

    b = 2.0 * (px * dx + py * dy)
    c = px * px + py * py - radius * radius
    disc = b * b - 4.0 * c
    if disc < 0:
        raise ValueError("line does not intersect the reference circle")

    sqrt_disc = math.sqrt(disc)
    roots = [(-b - sqrt_disc) / 2.0, (-b + sqrt_disc) / 2.0]
    candidates = [root for root in roots if abs(root) > 1e-8]
    if not candidates:
        raise ValueError("expected a second circle intersection")
    t = max(candidates, key=abs)
    return add(point_on_circle, scale(direction, t))


def build_geometry() -> dict[str, Point | list[Point]]:
    half_angle = V_APEX_ANGLE_RAD / 2.0
    left_apex = vec(LEFT_APEX_CIRCLE_ANGLE_RAD)

    left_outer_visible = line_circle_other_hit(left_apex, LEFT_V_AXIS_RAD + half_angle)
    left_outer_hidden = add(
        left_outer_visible,
        scale(vec(LEFT_V_AXIS_RAD + half_angle), float(END_EXTENSION)),
    )

    center_visible = line_circle_other_hit(left_apex, LEFT_V_AXIS_RAD - half_angle)

    right_to_peak_dir = vec(RIGHT_V_AXIS_RAD + half_angle)
    right_apex = sub(center_visible, scale(right_to_peak_dir, float(RIGHT_APEX_DISTANCE)))
    right_outer_visible = ray_circle_hit(right_apex, RIGHT_V_AXIS_RAD - half_angle)
    right_outer_hidden = add(
        right_outer_visible,
        scale(vec(RIGHT_V_AXIS_RAD - half_angle), float(END_EXTENSION)),
    )

    return {
        "left_outer_visible": left_outer_visible,
        "left_apex": left_apex,
        "center_visible": center_visible,
        "right_apex": right_apex,
        "right_outer_visible": right_outer_visible,
        "centerline": [
            left_outer_hidden,
            left_apex,
            center_visible,
            right_apex,
            right_outer_hidden,
        ],
        "circle_vertices": [
            left_outer_visible,
            left_apex,
            center_visible,
            right_outer_visible,
        ],
    }


def cumulative_path_progress(points: list[Point]) -> list[float]:
    """Return normalized cumulative arc length for each point on a polyline."""
    distances = [0.0]
    total = 0.0

    for start, end in zip(points, points[1:]):
        total += math.hypot(end[0] - start[0], end[1] - start[1])
        distances.append(total)

    if total == 0:
        raise ValueError("centerline has zero length")
    return [distance / total for distance in distances]


def width_scale(progress: float) -> float:
    """Monotone left-to-right widening with doubled initial width and unchanged delta."""
    return WIDTH_SCALE_START + WIDTH_SCALE_DELTA * progress


def build_stroke_outline(points: list[Point], stroke_widths: list[float]) -> list[Point]:
    """Expand a centerline with varying widths into a filled outline."""
    if len(points) != len(stroke_widths):
        raise ValueError("points and stroke_widths must have the same length")

    segment_dirs: list[Point] = []

    for start, end in zip(points, points[1:]):
        direction = unit(sub(end, start))
        segment_dirs.append(direction)

    left_outline: list[Point] = []
    right_outline: list[Point] = []

    start_normal = perp_left(segment_dirs[0])
    left_outline.append(add(points[0], scale(start_normal, stroke_widths[0] / 2.0)))
    right_outline.append(add(points[0], scale(start_normal, -stroke_widths[0] / 2.0)))

    for index in range(1, len(points) - 1):
        point = points[index]
        half_width = stroke_widths[index] / 2.0
        prev_normal = perp_left(segment_dirs[index - 1])
        next_normal = perp_left(segment_dirs[index])

        prev_left = add(point, scale(prev_normal, half_width))
        next_left = add(point, scale(next_normal, half_width))
        left_outline.append(
            line_intersection(prev_left, segment_dirs[index - 1], next_left, segment_dirs[index])
        )

        prev_right = add(point, scale(prev_normal, -half_width))
        next_right = add(point, scale(next_normal, -half_width))
        right_outline.append(
            line_intersection(prev_right, segment_dirs[index - 1], next_right, segment_dirs[index])
        )

    end_normal = perp_left(segment_dirs[-1])
    left_outline.append(add(points[-1], scale(end_normal, stroke_widths[-1] / 2.0)))
    right_outline.append(add(points[-1], scale(end_normal, -stroke_widths[-1] / 2.0)))

    return left_outline + list(reversed(right_outline))


def to_pixel(point: Point, center: Point, radius_px: float) -> Point:
    return (center[0] + point[0] * radius_px, center[1] - point[1] * radius_px)


def build_outline_pixels(canvas_size: int) -> dict[str, object]:
    center = (canvas_size / 2.0, canvas_size / 2.0)
    radius_px = canvas_size * float(REFERENCE_CIRCLE_RADIUS_RATIO)
    geometry = build_geometry()
    centerline = geometry["centerline"]
    progress_values = cumulative_path_progress(centerline)
    stroke_widths = [
        float(MAX_STROKE_WIDTH_TO_RADIUS_RATIO) * width_scale(progress)
        for progress in progress_values
    ]
    outline = build_stroke_outline(centerline, stroke_widths)
    outline_pixels = [to_pixel(point, center, radius_px) for point in outline]
    return {
        "center": center,
        "radius_px": radius_px,
        "outline_pixels": outline_pixels,
        "geometry": geometry,
    }


def render(size: int) -> Image.Image:
    work = size * SUPERSAMPLE
    outline_data = build_outline_pixels(work)
    center = outline_data["center"]
    radius_px = outline_data["radius_px"]

    shape_mask = Image.new("L", (work, work), 0)
    mask_draw = ImageDraw.Draw(shape_mask)
    mask_draw.polygon(outline_data["outline_pixels"], fill=255)

    clip_mask = Image.new("L", (work, work), 0)
    clip_draw = ImageDraw.Draw(clip_mask)
    clip_draw.ellipse(
        (
            center[0] - radius_px,
            center[1] - radius_px,
            center[0] + radius_px,
            center[1] + radius_px,
        ),
        fill=255,
    )

    final_mask = ImageChops.multiply(shape_mask, clip_mask)
    img = Image.new("RGBA", (work, work), BACKGROUND)
    img.paste(FILL_COLOR, (0, 0), final_mask)

    if DRAW_GUIDE_CIRCLE:
        guide_draw = ImageDraw.Draw(img)
        guide_draw.ellipse(
            (
                center[0] - radius_px,
                center[1] - radius_px,
                center[0] + radius_px,
                center[1] + radius_px,
            ),
            outline=GUIDE_COLOR,
            width=max(1, work // 128),
        )

    if SUPERSAMPLE != 1:
        img = img.resize((size, size), Image.LANCZOS)
    return img


def svg_number(value: float) -> str:
    text = f"{value:.15g}"
    return "0" if text == "-0" else text


def polygon_path_data(points: list[Point]) -> str:
    commands = [f"M {svg_number(points[0][0])} {svg_number(points[0][1])}"]
    commands.extend(f"L {svg_number(x)} {svg_number(y)}" for x, y in points[1:])
    commands.append("Z")
    return " ".join(commands)


def color_hex(color: tuple[int, int, int, int]) -> str:
    r, g, b, _ = color
    return f"#{r:02X}{g:02X}{b:02X}"


def render_svg(size: int) -> str:
    outline_data = build_outline_pixels(size)
    center = outline_data["center"]
    radius_px = outline_data["radius_px"]
    path_data = polygon_path_data(outline_data["outline_pixels"])
    fill_hex = color_hex(FILL_COLOR)

    parts = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        (
            f'<svg xmlns="http://www.w3.org/2000/svg" '
            f'width="{size}" height="{size}" viewBox="0 0 {size} {size}">'
        ),
        "  <defs>",
        '    <clipPath id="tray-icon-clip-circle">',
        (
            f'      <circle cx="{svg_number(center[0])}" cy="{svg_number(center[1])}" '
            f'r="{svg_number(radius_px)}" />'
        ),
        "    </clipPath>",
        "  </defs>",
        (
            f'  <path d="{path_data}" fill="{fill_hex}" '
            'clip-path="url(#tray-icon-clip-circle)" />'
        ),
    ]

    if DRAW_GUIDE_CIRCLE:
        guide_opacity = GUIDE_COLOR[3] / 255.0
        parts.append(
            (
                f'  <circle cx="{svg_number(center[0])}" cy="{svg_number(center[1])}" '
                f'r="{svg_number(radius_px)}" fill="none" '
                f'stroke="{color_hex(GUIDE_COLOR)}" stroke-opacity="{svg_number(guide_opacity)}" '
                'stroke-width="1" />'
            )
        )

    parts.append("</svg>")
    return "\n".join(parts) + "\n"


def main() -> None:
    PNG_OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)

    png_img = render(PNG_SIZE)
    png_img.save(PNG_OUTPUT_PATH, format="PNG")

    ico_img = render(ICO_SOURCE_SIZE)
    ico_img.save(ICO_OUTPUT_PATH, format="ICO", sizes=ICO_SIZES)

    SVG_OUTPUT_PATH.write_text(render_svg(PNG_SIZE), encoding="utf-8")

    print(f"Wrote {PNG_OUTPUT_PATH} ({PNG_SIZE}x{PNG_SIZE})")
    print(f"Wrote {ICO_OUTPUT_PATH} ({ICO_SOURCE_SIZE} source, {len(ICO_SIZES)} sizes)")
    print(f"Wrote {SVG_OUTPUT_PATH} ({PNG_SIZE} viewBox)")


if __name__ == "__main__":
    main()

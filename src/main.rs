use minifb::{Key, Window, WindowOptions};
use std::time::Duration;

mod framebuffer;
mod map;

use framebuffer::Framebuffer;
use map::{initialize_map, Map};

const WIDTH: usize = 640;  // Ancho de la ventana (en píxeles)
const HEIGHT: usize = 480; // Altura de la ventana (en píxeles)
const CELL_SIZE: usize = 1; // Tamaño de cada celda (en píxeles)

const COLOR_FONDO: u32 = 0x000000;
const COLOR_PARED: u32 = 0xFFFFFF;

fn draw_2d_map(map: &Map, framebuffer: &mut Framebuffer) {
    let cell_width = framebuffer.width / map.width;
    let cell_height = framebuffer.height / map.height;

    for y in 0..map.height {
        for x in 0..map.width {
            let color = if map.is_wall(x as f64, y as f64) {
                COLOR_PARED
            } else {
                COLOR_FONDO
            };

            for py in 0..cell_height {
                for px in 0..cell_width {
                    framebuffer.point(x * cell_width + px, y * cell_height + py, color);
                }
            }
        }
    }
}

fn main() {
    let map = initialize_map();
    let window_width = WIDTH * CELL_SIZE;
    let window_height = HEIGHT * CELL_SIZE;
    let frame_delay = Duration::from_millis(16); // 16ms para aproximadamente 60 FPS

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut window = Window::new(
        "2D Map View",
        window_width,
        window_height,
        WindowOptions::default(),
    )
    .unwrap();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        framebuffer.buffer.fill(COLOR_FONDO);

        // Dibuja el mapa en 2D
        draw_2d_map(&map, &mut framebuffer);

        let mut display_buffer = vec![COLOR_FONDO; window_width * window_height];
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if framebuffer.is_point_set(x, y) {
                    display_buffer[y * window_width + x] = framebuffer.buffer[y * WIDTH + x];
                }
            }
        }

        window
            .update_with_buffer(&display_buffer, window_width, window_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}

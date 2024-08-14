use minifb::{Key, Window, WindowOptions};
use std::time::{Duration, Instant};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
mod framebuffer;
mod map;

use framebuffer::Framebuffer;
use map::{initialize_map, Map};

mod player;
use player::Player;

mod raycaster;
use raycaster::cast_ray;

const WIDTH: usize = 640;  // Ancho de la ventana (en píxeles)
const HEIGHT: usize = 480; // Altura de la ventana (en píxeles)
const CELL_SIZE: usize = 1; // Tamaño de cada celda (en píxeles)

const COLOR_FONDO: u32 = 0x000000;
const COLOR_PARED: u32 = 0xFFFFFF;



fn render_scene(map: &Map, player: &Player, framebuffer: &mut Framebuffer) {
    for x in 0..framebuffer.width {
        // Calcular el ángulo del rayo para esta columna de la pantalla
        let camera_x = 2.0 * (x as f64) / (framebuffer.width as f64) - 1.0;
        let angle_offset = player.fov / 2.0 * camera_x;

        // Lanzar el rayo y obtener la distancia a la pared
        let (perp_wall_dist, is_horizontal) = cast_ray(map, player, angle_offset);

        // Calcular la altura de la pared en la pantalla
        let mut wall_height = (framebuffer.height as f64 / perp_wall_dist) as usize;
        
        // Limitar la altura máxima de la pared
        wall_height = wall_height.min(framebuffer.height);

        let start = (framebuffer.height / 2).saturating_sub(wall_height / 2);
        let end = (framebuffer.height / 2) + wall_height / 2;

        // Dibujar la pared en la pantalla
        let color = if is_horizontal { 0xCCCCCC } else { 0xAAAAAA }; // Diferente color para paredes horizontales y verticales
        for y in start..end {
            framebuffer.point(x, y, color);
        }
    }
}



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

fn draw_minimap(map: &Map, player: &Player, framebuffer: &mut Framebuffer) {
    let minimap_scale = 4; // Escala del minimapa
    let minimap_size = map.width * minimap_scale;

    for y in 0..map.height {
        for x in 0..map.width {
            let color = if map.is_wall(x as f64, y as f64) {
                0xFFFFFF // Color de las paredes en el minimapa
            } else {
                0x000000 // Color del suelo en el minimapa
            };

            for py in 0..minimap_scale {
                for px in 0..minimap_scale {
                    framebuffer.point(x * minimap_scale + px, y * minimap_scale + py, color);
                }
            }
        }
    }

    // Dibujar la posición del jugador en el minimapa
    let player_x = (player.x * minimap_scale as f64) as usize;
    let player_y = (player.y * minimap_scale as f64) as usize;
    for py in 0..minimap_scale {
        for px in 0..minimap_scale {
            framebuffer.point(player_x + px, player_y + py, 0xFF0000); // Rojo para la posición del jugador
        }
    }
}

const FONT: [[u8; 5]; 13] = [
    [0b01110, 0b10001, 0b10001, 0b10001, 0b01110], // 0
    [0b00100, 0b01100, 0b00100, 0b00100, 0b01110], // 1
    [0b01110, 0b10001, 0b00110, 0b01000, 0b11111], // 2
    [0b01110, 0b10001, 0b00110, 0b10001, 0b01110], // 3
    [0b00010, 0b00110, 0b01010, 0b11111, 0b00010], // 4
    [0b11111, 0b10000, 0b11110, 0b00001, 0b11110], // 5
    [0b01110, 0b10000, 0b11110, 0b10001, 0b01110], // 6
    [0b11111, 0b00010, 0b00100, 0b01000, 0b10000], // 7
    [0b01110, 0b10001, 0b01110, 0b10001, 0b01110], // 8
    [0b01110, 0b10001, 0b01111, 0b00001, 0b01110], // 9
    [0b11111, 0b10000, 0b11110, 0b10000, 0b10000], // F
    [0b11110, 0b10001, 0b11110, 0b10000, 0b10000], // P
    [0b01111, 0b10000, 0b01110, 0b00001, 0b11110], // S
];

fn draw_digit(framebuffer: &mut Framebuffer, x: usize, y: usize, index: usize, color: u32) {
    if index >= FONT.len() { return; }

    for (row, byte) in FONT[index].iter().enumerate() {
        for col in 0..5 {
            if byte & (1 << (4 - col)) != 0 {
                framebuffer.point(x + col, y + row, color);
            }
        }
    }
}

fn draw_text(framebuffer: &mut Framebuffer, x: usize, y: usize, text: &str, color: u32) {
    let mut x_offset = 0;

    for ch in text.chars() {
        let index = match ch {
            '0'..='9' => ch as usize - '0' as usize,
            'F' => 10,
            'P' => 11,
            'S' => 12,
            _ => continue,
        };

        draw_digit(framebuffer, x + x_offset, y, index, color);
        x_offset += 6; // Espacio entre caracteres
    }
}

fn main() {
    // Inicializa el sistema de audio
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    // Cargar el archivo de música
    let file = BufReader::new(File::open("src/music-_1_.wav").unwrap());

    // Decodificar el archivo de música
    let source = Decoder::new(file).unwrap();

    // Hacer que la música se repita indefinidamente
    sink.append(source.repeat_infinite());

    // Establecer el volumen inicial
    let mut volume = 0.1;
    sink.set_volume(volume);

    // Comienza a reproducir la música en segundo plano
    sink.play();

    // Inicialización del juego
    let map = initialize_map();
    let mut player = Player::new(12.0, 12.0, 0.0);

    let window_width = WIDTH;
    let window_height = HEIGHT;

    let target_fps = 60;
    let frame_duration = Duration::from_secs_f64(1.0 / target_fps as f64);

    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    let mut window = Window::new(
        "3D Raycaster",
        window_width,
        window_height,
        WindowOptions::default(),
    )
    .unwrap();

    let mut last_time = Instant::now();
    let mut frame_count = 0;
    let mut fps = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let start_time = Instant::now();

        framebuffer.buffer.fill(COLOR_FONDO);

        // Capturar entradas del teclado para mover al jugador
        if window.is_key_down(Key::W) || window.is_key_down(Key::Up) {
            player.move_forward(0.05, &map); // Reduce la velocidad de movimiento
        }
        if window.is_key_down(Key::S) || window.is_key_down(Key::Down) {
            player.move_backward(0.05, &map); // Reduce la velocidad de movimiento
        }
        if window.is_key_down(Key::A) || window.is_key_down(Key::Left) {
            player.turn_left(0.03); // Reduce la velocidad de rotación
        }
        if window.is_key_down(Key::D) || window.is_key_down(Key::Right) {
            player.turn_right(0.03); // Reduce la velocidad de rotación
        }

        // Control del volumen
        if window.is_key_down(Key::Equal) { // Tecla "+"
            volume = (volume + 0.001).min(4.0); // Aumenta el volumen hasta un máximo de 1.0
            sink.set_volume(volume);
        }
        if window.is_key_down(Key::Minus) { // Tecla "-"
            volume = (volume - 0.001).max(0.0); // Disminuye el volumen hasta un mínimo de 0.0
            sink.set_volume(volume);
        }

        // Renderiza la escena 3D
        render_scene(&map, &player, &mut framebuffer);

        // Dibujar el minimapa en la esquina superior izquierda
        draw_minimap(&map, &player, &mut framebuffer);

        // Calcular FPS
        let current_time = Instant::now();
        frame_count += 1;

        if current_time.duration_since(last_time).as_secs_f64() >= 1.0 {
            fps = frame_count;
            last_time = current_time;
            frame_count = 0;
        }

        // Dibujar FPS en la esquina superior derecha con la palabra "FPS"
        let width = framebuffer.width;
        draw_text(&mut framebuffer, width - 70, 10, &format!("{}FPS", fps), 0xFFFFFF);

        // Actualiza el búfer de pantalla con el contenido del framebuffer
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

        // Control estricto del tiempo para mantener los FPS
        let elapsed_time = start_time.elapsed();
        if frame_duration > elapsed_time {
            std::thread::sleep(frame_duration - elapsed_time);
        }
    }
}

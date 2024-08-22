use minifb::{Key, Window, WindowOptions};
use std::time::{Duration, Instant};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use rand::Rng;

mod framebuffer;
mod map;

use framebuffer::Framebuffer;
use map::{initialize_map, Map};

mod player;
use player::Player;

mod raycaster;
use raycaster::cast_ray;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;
const COLOR_FONDO: u32 = 0x000000;
const COLOR_PARED: u32 = 0xFFFFFF;

enum GameState {
    WelcomeScreen,
    Playing,
    WinScreen,
}

struct Item {
    x: f64,
    y: f64,
    collected: bool,
}

fn render_scene(map: &Map, player: &Player, framebuffer: &mut Framebuffer) {
    for x in 0..framebuffer.width {
        let camera_x = 2.0 * (x as f64) / (framebuffer.width as f64) - 1.0;
        let angle_offset = player.fov / 2.0 * camera_x;

        let (perp_wall_dist, is_horizontal) = cast_ray(map, player, angle_offset);

        let mut wall_height = (framebuffer.height as f64 / perp_wall_dist) as usize;
        wall_height = wall_height.min(framebuffer.height);

        let start = (framebuffer.height / 2).saturating_sub(wall_height / 2);
        let end = (framebuffer.height / 2) + wall_height / 2;

        let color = if is_horizontal { 0xCCCCCC } else { 0xAAAAAA };
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

fn draw_minimap(map: &Map, player: &Player, framebuffer: &mut Framebuffer, key: &Item, goal: &Item) {
    let minimap_scale = 4;

    for y in 0..map.height {
        for x in 0..map.width {
            let color = if map.is_wall(x as f64, y as f64) {
                0xFFFFFF
            } else {
                0x000000
            };

            for py in 0..minimap_scale {
                for px in 0..minimap_scale {
                    framebuffer.point(x * minimap_scale + px, y * minimap_scale + py, color);
                }
            }
        }
    }

    let player_x = (player.x * minimap_scale as f64) as usize;
    let player_y = (player.y * minimap_scale as f64) as usize;
    for py in 0..minimap_scale {
        for px in 0..minimap_scale {
            framebuffer.point(player_x + px, player_y + py, 0xFF0000);
        }
    }

    if !key.collected {
        let key_x = (key.x * minimap_scale as f64) as usize;
        let key_y = (key.y * minimap_scale as f64) as usize;
        for py in 0..minimap_scale {
            for px in 0..minimap_scale {
                framebuffer.point(key_x + px, key_y + py, 0xFFFF00); // Amarillo para la llave
            }
        }
    }

    let goal_x = (goal.x * minimap_scale as f64) as usize;
    let goal_y = (goal.y * minimap_scale as f64) as usize;
    for py in 0..minimap_scale {
        for px in 0..minimap_scale {
            framebuffer.point(goal_x + px, goal_y + py, 0x00FF00); // Verde para la meta
        }
    }
}

const FONT: [[u8; 5]; 21] = [
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
    [0b10001, 0b10001, 0b10101, 0b10101, 0b01010], // W
    [0b01110, 0b10000, 0b11110, 0b10000, 0b01110], // E
    [0b10000, 0b10000, 0b10000, 0b10000, 0b11110], // L
    [0b01110, 0b10001, 0b10001, 0b10001, 0b01110], // O
    [0b11110, 0b10001, 0b10001, 0b10001, 0b10001], // M
    [0b10001, 0b11001, 0b10101, 0b10011, 0b10001], // N
    [0b01010, 0b11111, 0b10001, 0b10001, 0b10001], // C
    [0b11111, 0b00100, 0b00100, 0b00100, 0b11111], //I
];

fn draw_digit(framebuffer: &mut Framebuffer, x: usize, y: usize, index: usize, color: u32, scale: usize) {
    if index >= FONT.len() { return; }

    for (row, byte) in FONT[index].iter().enumerate() {
        for col in 0..5 {
            if byte & (1 << (4 - col)) != 0 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        framebuffer.point(x + col * scale + sx, y + row * scale + sy, color);
                    }
                }
            }
        }
    }
}

fn draw_text(framebuffer: &mut Framebuffer, x: usize, y: usize, text: &str, color: u32, scale: usize) {
    let mut x_offset = 0;
    for ch in text.chars() {
        let index = match ch {
            '0'..='9' => ch as usize - '0' as usize,
            'F' => 10,
            'P' => 11,
            'S' => 12,
            'W' => 13,
            'E' => 14,
            'L' => 15,
            'O' => 16,
            'M' => 17,
            'N' => 18,
            'C' => 19,
            'I' => 20,
            _ => continue,
        };
        draw_digit(framebuffer, x + x_offset, y, index, color, scale);
        x_offset += 6 * scale;
    }
}

fn draw_centered_text(framebuffer: &mut Framebuffer, text: &str, color: u32, scale: usize) {
    let char_width = 6 * scale;
    let char_height = 5 * scale;
    let text_width = text.len() * char_width;
    let text_height = char_height;

    let x_start = (WIDTH - text_width) / 2;
    let y_start = (HEIGHT - text_height) / 2;

    draw_text(framebuffer, x_start, y_start, text, color, scale);
}

fn draw_fps(framebuffer: &mut Framebuffer, fps: usize) {
    draw_text(framebuffer, WIDTH - 70, 10, &format!("{}FPS", fps), 0xFFFFFF, 1);
}

fn generate_random_position(map: &Map) -> (f64, f64) {
    let mut rng = rand::thread_rng();
    let mut x;
    let mut y;
    
    // Ajustamos los límites para asegurarnos de que no se generen fuera del mapa
    let min_limit = 1;
    let max_x_limit = map.width - 20;  // Ajustamos para evitar bordes
    let max_y_limit = map.height - 20; // Ajustamos para evitar bordes
    
    loop {
        x = rng.gen_range(min_limit..=max_x_limit) as f64;
        y = rng.gen_range(min_limit..=max_y_limit) as f64;
        
        if !map.is_wall(x, y) {
            break;
        }
    }

    (x, y)
}


fn main() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    let music_sink = Sink::try_new(&stream_handle).unwrap();
    let music_file = BufReader::new(File::open("src/music-_1_.wav").unwrap());
    let music_source = Decoder::new(music_file).unwrap();
    music_sink.append(music_source.repeat_infinite());

    let walk_sink = Sink::try_new(&stream_handle).unwrap();

    let mut volume = 0.1;
    music_sink.set_volume(volume);
    walk_sink.set_volume(2.0);

    music_sink.play();

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

    let mut last_mouse_x = WIDTH as f32 / 2.0;
    let mut game_state = GameState::WelcomeScreen;

    let mut last_time = Instant::now();
    let mut frame_count = 0;
    let mut fps = 0;

    // Generar llave y meta en posiciones válidas dentro del mapa
    let (key_x, key_y) = generate_random_position(&map);
    let (goal_x, goal_y) = generate_random_position(&map);

    let mut key = Item { x: key_x, y: key_y, collected: false };
    let mut goal = Item { x: goal_x, y: goal_y, collected: false };

    while window.is_open() && !window.is_key_down(Key::Escape) {
        match game_state {
            GameState::WelcomeScreen => {
                framebuffer.buffer.fill(COLOR_FONDO);
                draw_centered_text(&mut framebuffer, "WELCOCE", 0xFFFFFF, 3);
                draw_centered_text(&mut framebuffer, "", 0xFFFFFF, 2);
                window.update_with_buffer(&framebuffer.buffer, WIDTH, HEIGHT).unwrap();

                if window.is_key_down(Key::Enter) {
                    game_state = GameState::Playing;
                    last_time = Instant::now();  // Reiniciar el tiempo para el cálculo de FPS
                    frame_count = 0;
                }
            }
            GameState::Playing => {
                let start_time = Instant::now();
                framebuffer.buffer.fill(COLOR_FONDO);

                if window.is_key_down(Key::W) || window.is_key_down(Key::Up) {
                    player.move_forward(0.05, &map, &walk_sink);
                }
                if window.is_key_down(Key::S) || window.is_key_down(Key::Down) {
                    player.move_backward(0.05, &map, &walk_sink);
                }
                if window.is_key_down(Key::A) || window.is_key_down(Key::Left) {
                    player.turn_left(0.03);
                }
                if window.is_key_down(Key::D) || window.is_key_down(Key::Right) {
                    player.turn_right(0.03);
                }

                if window.is_key_down(Key::Equal) {
                    volume = (volume + 0.001).min(4.0);
                    music_sink.set_volume(volume);
                }
                if window.is_key_down(Key::Minus) {
                    volume = (volume - 0.001).max(0.0);
                    music_sink.set_volume(volume);
                }

                if let Some((mouse_x, _)) = window.get_mouse_pos(minifb::MouseMode::Pass) {
                    let mouse_delta = mouse_x - last_mouse_x;
                    player.turn_right(mouse_delta as f64 * 0.002);
                    last_mouse_x = mouse_x;
                }

                // Comprobar si el jugador ha recogido la llave
                if (player.x - key.x).abs() < 0.5 && (player.y - key.y).abs() < 0.5 {
                    key.collected = true;
                }

                // Comprobar si el jugador ha llegado a la meta después de recoger la llave
                if key.collected && (player.x - goal.x).abs() < 0.5 && (player.y - goal.y).abs() < 0.5 {
                    game_state = GameState::WinScreen;
                }

                render_scene(&map, &player, &mut framebuffer);
                draw_minimap(&map, &player, &mut framebuffer, &key, &goal);

                frame_count += 1;
                let current_time = Instant::now();
                if current_time.duration_since(last_time).as_secs_f64() >= 1.0 {
                    fps = frame_count;
                    last_time = current_time;
                    frame_count = 0;
                }

                draw_fps(&mut framebuffer, fps);

                let mut display_buffer = vec![COLOR_FONDO; window_width * window_height];
                for y in 0..HEIGHT {
                    for x in 0..WIDTH {
                        if framebuffer.is_point_set(x, y) {
                            display_buffer[y * window_width + x] = framebuffer.buffer[y * WIDTH + x];
                        }
                    }
                }

                window.update_with_buffer(&display_buffer, window_width, window_height).unwrap();

                let elapsed_time = start_time.elapsed();
                if frame_duration > elapsed_time {
                    std::thread::sleep(frame_duration - elapsed_time);
                }
            }
            GameState::WinScreen => {
                framebuffer.buffer.fill(COLOR_FONDO);
                draw_centered_text(&mut framebuffer, "WIN", 0x00FF00, 4);
                window.update_with_buffer(&framebuffer.buffer, WIDTH, HEIGHT).unwrap();
            }
        }
    }
}

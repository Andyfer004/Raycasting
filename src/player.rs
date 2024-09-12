use std::{fs::File, io::BufReader};

use rodio::{Decoder, Sink};

use crate::map::Map;
pub struct Player {
    pub x: f64,
    pub y: f64,
    pub direction: f64, // Ángulo de dirección del jugador
    pub fov: f64,       // Campo de visión
    pub plane_x: f64,   // Plano de la cámara en el eje X
    pub plane_y: f64,   // Plano de la cámara en el eje Y
}

impl Player {
    pub fn new(x: f64, y: f64, direction: f64) -> Player {
        let fov = 66.0_f64.to_radians(); // Establecer un campo de visión
        let plane_x = -direction.sin() * fov / 2.0; // Plano en X
        let plane_y = direction.cos() * fov / 2.0;  // Plano en Y
        Player {
            x,
            y,
            direction,
            fov,
            plane_x,
            plane_y,
        }
    }

    pub fn move_forward(&mut self, distance: f64, map: &Map, walk_sink: &Sink) {
        let new_x = self.x + self.direction.cos() * distance;
        let new_y = self.y + self.direction.sin() * distance;

        if !map.is_wall(new_x, self.y) {
            self.x = new_x;
            self.play_walk_sound(walk_sink);
        }

        if !map.is_wall(self.x, new_y) {
            self.y = new_y;
            self.play_walk_sound(walk_sink);
        }
    }

    pub fn move_backward(&mut self, distance: f64, map: &Map, walk_sink: &Sink) {
        let new_x = self.x - self.direction.cos() * distance;
        let new_y = self.y - self.direction.sin() * distance;

        if !map.is_wall(new_x, self.y) {
            self.x = new_x;
            self.play_walk_sound(walk_sink);
        }

        if !map.is_wall(self.x, new_y) {
            self.y = new_y;
            self.play_walk_sound(walk_sink);
        }
    }

    fn play_walk_sound(&self, walk_sink: &Sink) {
        // Si el Sink está vacío, entonces reproducimos el sonido
        if walk_sink.empty() {
            let walk_file = BufReader::new(File::open("src/pasos.wav").unwrap());
            let walk_source = Decoder::new(walk_file).unwrap();
    
            walk_sink.append(walk_source);
            walk_sink.play(); // Asegúrate de que se reproduzca
        }
    }
    

    pub fn turn_left(&mut self, angle: f64) {
        self.direction -= angle;
    }

    pub fn turn_right(&mut self, angle: f64) {
        self.direction += angle;
    }
}

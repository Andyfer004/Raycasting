use std::{fs::File, io::BufReader};

use rodio::{Decoder, Sink};

use crate::map::Map;
pub struct Player {
    pub x: f64,
    pub y: f64,
    pub direction: f64, // En radianes
    pub fov: f64, // Campo de visión en radianes
}

impl Player {
    pub fn new(x: f64, y: f64, direction: f64) -> Self {
        Self {
            x,
            y,
            direction,
            fov: 60.0_f64.to_radians(), // FOV predeterminado de 60 grados
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
